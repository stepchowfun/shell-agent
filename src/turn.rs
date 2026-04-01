use {
    crate::{Cli, OPENAI_API_KEY_ENV_VAR, format::CodeStr, get_instructions},
    async_openai::{
        Client,
        config::OpenAIConfig,
        types::responses::{
            ContextManagementParam, CreateResponse, CreateResponseArgs, FunctionCallOutput,
            FunctionCallOutputItemParam, FunctionTool, InputContent, InputItem, InputMessage,
            InputParam, InputRole, InputTextContent, Item, OutputItem, ResponseStreamEvent, Tool,
        },
    },
    futures::StreamExt,
    serde::{Deserialize, Serialize},
    std::{error::Error, io::Write, process::Stdio},
    tokio::process::Command,
};

// Tools
const RUN_SHELL_COMMAND_TOOL: &str = "run_shell_command";

#[derive(Debug, Deserialize)]
struct RunShellCommandFunctionArgs {
    command: String,
}

#[derive(Debug, Serialize)]
struct RunShellCommandFunctionResult {
    stdout: String,
    stderr: String,
    exit_status: i32,
}

#[derive(Debug, Serialize)]
struct CreateResponseWithContextManagement {
    #[serde(flatten)]
    request: CreateResponse,
    context_management: Vec<ContextManagementParam>,
}

// Run a shell command and collect the output.
async fn run_shell_command(args: RunShellCommandFunctionArgs) -> RunShellCommandFunctionResult {
    eprintln!("Running: {}", args.command.code_str());

    let mut command = Command::new("sh");
    command
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .env_remove(OPENAI_API_KEY_ENV_VAR)
        .arg("-c")
        .arg(args.command);

    match command.output().await {
        Ok(output) => RunShellCommandFunctionResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_status: output.status.code().unwrap_or(1_i32),
        },
        Err(_) => RunShellCommandFunctionResult {
            stdout: String::new(),
            stderr: String::new(),
            exit_status: 1,
        },
    }
}

// Set up the tools.
fn tools() -> Vec<Tool> {
    vec![Tool::Function(FunctionTool {
        name: RUN_SHELL_COMMAND_TOOL.to_owned(),
        description: Some("Run a shell command and return the output.".to_owned()),
        defer_loading: None,
        parameters: Some(serde_json::json!(
            {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to run."
                    },
                },
                "required": [
                    "command",
                ],
                "additionalProperties": false
            }
        )),
        strict: None,
    })]
}

// Construct an `InputItem` corresponding to a user message.
pub fn user_message(text: &str) -> InputItem {
    InputMessage {
        content: vec![InputContent::InputText(InputTextContent {
            text: text.to_owned(),
        })],
        role: InputRole::User,
        status: None,
    }
    .into()
}

// Construct an `InputItem` corresponding to a system message.
pub fn system_message(text: &str) -> InputItem {
    InputMessage {
        content: vec![InputContent::InputText(InputTextContent {
            text: text.to_owned(),
        })],
        role: InputRole::System,
        status: None,
    }
    .into()
}

// Convert an `OutputItem` into an `InputItem`.
fn output_item_to_input_item(output_item: OutputItem) -> Result<InputItem, serde_json::Error> {
    serde_json::from_value(serde_json::to_value(output_item)?)
}

// Remove items before compaction items in the conversation.
fn prune_compacted_history(conversation: &mut Vec<InputItem>) {
    if let Some(index) = conversation
        .iter()
        .rposition(|item| matches!(item, InputItem::Item(Item::Compaction(_))))
        && index > 0
    {
        conversation.drain(..index);
        eprintln!("{}", "Context compacted.".code_str());
    }
}

// Run a single turn of the agent.
#[allow(clippy::too_many_lines)]
pub async fn run_turn(
    client: &Client<OpenAIConfig>,
    settings: &Cli,
    conversation: &[InputItem],
) -> Result<Vec<InputItem>, Box<dyn Error>> {
    // Make a local copy of the conversation so we can mutate it.
    let mut conversation = conversation.to_vec();

    // Let the agent cook until it's done.
    loop {
        // Build the API request.
        let mut request_builder = CreateResponseArgs::default();
        request_builder
            .model(settings.model.clone())
            .stream(true)
            .instructions(get_instructions()?)
            .tools(tools())
            .input(InputParam::Items(conversation.clone()));
        let request = CreateResponseWithContextManagement {
            // The `unwrap` has been manually verified to be safe.
            request: request_builder.build().unwrap(),
            context_management: vec![ContextManagementParam {
                type_: "compaction".to_owned(),
                compact_threshold: Some(settings.compaction_threshold),
            }],
        };

        // Send the request to the OpenAI API and stream the response.
        let mut stream = client.responses().create_stream_byot(request).await?;
        let mut received_output = false;
        let mut output_items = Vec::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => match event {
                    ResponseStreamEvent::ResponseOutputTextDelta(event) => {
                        // Output the response delta.
                        print!("{}", event.delta);
                        let _ = std::io::stdout().flush();
                        received_output = true;
                    }
                    ResponseStreamEvent::ResponseCompleted(event) => {
                        // Capture output items.
                        output_items.extend(event.response.output);
                    }
                    _ => {
                        // Ignore other events.
                    }
                },
                Err(error) => {
                    return Err(Box::new(error));
                }
            }
        }

        // Output a newline after the response to separate it from the next
        // prompt.
        if received_output {
            println!();
        }

        // Add the agent responses to the conversation.
        for output_item in &output_items {
            conversation.push(output_item_to_input_item(output_item.clone())?);
        }

        // Handle compaction.
        prune_compacted_history(&mut conversation);

        // Handle the function tool calls.
        let mut any_function_tool_calls = false;
        let mut interrupted = false;
        for output_item in output_items {
            if let OutputItem::FunctionCall(function_tool_call) = output_item {
                any_function_tool_calls = true;

                if function_tool_call.name == RUN_SHELL_COMMAND_TOOL {
                    let args: RunShellCommandFunctionArgs =
                        serde_json::from_str(&function_tool_call.arguments)?;
                    let call_id = function_tool_call.call_id;
                    let result = tokio::select! {
                        output = run_shell_command(args) => output,
                        signal = tokio::signal::ctrl_c() => {
                            if let Err(error) = signal {
                                eprintln!("Error waiting for CTRL-C: {error}");
                            }
                            interrupted = true;
                            RunShellCommandFunctionResult {
                                stdout: String::new(),
                                stderr: String::new(),
                                exit_status: 130,
                            }
                        }
                    };
                    conversation.push(
                        Item::FunctionCallOutput(FunctionCallOutputItemParam {
                            call_id,
                            output: FunctionCallOutput::Text(serde_json::to_string(&result)?),
                            id: None,
                            status: None,
                        })
                        .into(),
                    );
                } else {
                    eprintln!("Unexpected function tool call: {function_tool_call:?}");
                }
            }
        }

        // If there were no function tool calls, the turn is over.
        if !any_function_tool_calls {
            break;
        }

        // If the user interrupted any function tool calls, inform the model so
        // it doesn't retry.
        if interrupted {
            conversation.push(system_message(
                "The user interrupted this turn with CTRL-C.",
            ));
        }
    }

    // Return the latest conversation state for the next turn.
    Ok(conversation)
}
