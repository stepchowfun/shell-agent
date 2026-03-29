use {
    crate::{OPENAI_API_KEY_ENV_VAR, Settings, format::CodeStr},
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

// Model instructions
const INSTRUCTIONS: &str = "You are a helpful assistant that can run shell \
    commands.";

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

// Run a single turn of the agent.
#[allow(clippy::too_many_lines)]
pub async fn run_turn(
    client: &Client<OpenAIConfig>,
    settings: &Settings,
    line: &str,
    mut previous_response_id: Option<String>,
) -> Result<Option<String>, Box<dyn Error>> {
    // Returns new `previous_response_id`
    // Keep track of items to feed into the next model request.
    let mut items: Vec<InputItem> = vec![
        InputMessage {
            content: vec![InputContent::InputText(InputTextContent {
                text: line.to_owned(),
            })],
            role: InputRole::User,
            status: None,
        }
        .into(),
    ];

    // Let the agent cook until it's done.
    loop {
        // Build the API request.
        let mut request_builder = CreateResponseArgs::default();
        request_builder
            .model(settings.model.clone())
            .stream(true)
            .instructions(INSTRUCTIONS)
            .tools(tools())
            .input(InputParam::Items(items.clone()));
        if let Some(ref id) = previous_response_id {
            request_builder.previous_response_id(id);
        }
        let request = CreateResponseWithContextManagement {
            // The `unwrap` has been manually verified to be safe.
            request: request_builder.build().unwrap(),
            context_management: vec![ContextManagementParam {
                type_: "compaction".to_owned(),
                compact_threshold: Some(settings.compaction_threshold),
            }],
        };

        // Clear the items for the next request.
        items.clear();

        // Send the request to the OpenAI API and stream the response.
        let mut stream = client.responses().create_stream_byot(request).await?;
        let mut function_tool_calls = Vec::new();
        let mut received_output = false;
        let mut compacted = false;
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => match event {
                    ResponseStreamEvent::ResponseCreated(event) => {
                        // Remember the response ID for the next request.
                        previous_response_id = Some(event.response.id);
                    }
                    ResponseStreamEvent::ResponseOutputTextDelta(event) => {
                        // Output the response delta.
                        print!("{}", event.delta);
                        let _ = std::io::stdout().flush();
                        received_output = true;
                    }
                    ResponseStreamEvent::ResponseCompleted(event) => {
                        // Capture tool calls and compaction events.
                        for output_item in event.response.output {
                            match output_item {
                                OutputItem::FunctionCall(function_tool_call) => {
                                    function_tool_calls.push(function_tool_call);
                                }
                                OutputItem::Compaction(_) => {
                                    compacted = true;
                                }
                                _ => {}
                            }
                        }
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

        // Let the user know if a compaction occurred.
        if compacted {
            eprintln!("{}", "Context compacted.".code_str());
        }

        // If there are no function calls, break the loop.
        if function_tool_calls.is_empty() {
            break;
        }

        // Handle the function calls.
        let mut interrupted = false;
        for function_tool_call in function_tool_calls {
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
                items.push(
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

        // If the user interrupted any function calls, inform the model so it
        // doesn't retry.
        if interrupted {
            items.push(
                InputMessage {
                    content: vec![InputContent::InputText(InputTextContent {
                        text: "The user interrupted this turn with CTRL-C.".to_owned(),
                    })],
                    role: InputRole::System,
                    status: None,
                }
                .into(),
            );
        }
    }

    // Return the latest previous response ID for the next turn.
    Ok(previous_response_id)
}
