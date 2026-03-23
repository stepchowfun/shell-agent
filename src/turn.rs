use {
    crate::{Settings, format::CodeStr},
    async_openai::{
        Client,
        config::OpenAIConfig,
        error::OpenAIError,
        types::responses::{
            CreateResponseArgs, FunctionCallOutput, FunctionCallOutputItemParam, FunctionTool,
            InputItem, InputParam, Item, OutputItem, ResponseStreamEvent, Tool,
        },
    },
    futures::StreamExt,
    serde::{Deserialize, Serialize},
    std::{error::Error, process::Command},
};

// Model instructions
const INSTRUCTIONS: &str = "You are a helpful assistant that can run shell \
    commands.";

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

// Run a shell command and collect the output.
fn run_shell_command(args: RunShellCommandFunctionArgs) -> RunShellCommandFunctionResult {
    eprintln!("Running: {}", args.command.code_str());
    match Command::new("sh").arg("-c").arg(args.command).output() {
        Ok(output) => RunShellCommandFunctionResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_status: output.status.code().unwrap_or(1_i32),
        },
        Err(_) => RunShellCommandFunctionResult {
            stdout: String::new(),
            stderr: String::new(),
            exit_status: 1_i32,
        },
    }
}

// Set up the tools.
fn tools() -> Vec<Tool> {
    vec![Tool::Function(FunctionTool {
        name: "run_shell_command".to_string(),
        description: Some("Run a shell command and return the output.".to_string()),
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
pub async fn run_turn(
    client: &Client<OpenAIConfig>,
    settings: &Settings,
    line: &str,
    previous_response_id: &mut Option<String>,
) -> Result<(), Box<dyn Error>> {
    // Keep track of the function call outputs.
    let mut function_call_outputs: Vec<FunctionCallOutputItemParam> = Vec::new();

    // Let the agent cook until it's done.
    loop {
        // Build the API request.
        let mut request_builder = CreateResponseArgs::default();
        request_builder
            .model(settings.model.clone())
            .stream(true)
            .instructions(INSTRUCTIONS)
            .tools(tools())
            .input(if function_call_outputs.is_empty() {
                InputParam::Text(line.to_owned())
            } else {
                InputParam::Items(
                    function_call_outputs
                        .clone()
                        .into_iter()
                        .map(|output| InputItem::Item(Item::FunctionCallOutput(output)))
                        .collect(),
                )
            });
        if let Some(ref id) = *previous_response_id {
            request_builder.previous_response_id(id);
        }
        let request = request_builder.build().unwrap(); // Manually verified to be safe

        // Keep track of the function calls.
        let mut function_tool_calls = Vec::new();

        // Send the request to the OpenAI API and stream the response.
        let mut stream = client.responses().create_stream(request).await?;
        let mut received_output = false;
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => match event {
                    ResponseStreamEvent::ResponseCreated(event) => {
                        // Remember the response ID for the next
                        // request.
                        *previous_response_id = Some(event.response.id);
                    }
                    ResponseStreamEvent::ResponseOutputTextDelta(event) => {
                        // Output the response delta.
                        print!("{}", event.delta);
                        received_output = true;
                    }
                    ResponseStreamEvent::ResponseCompleted(event) => {
                        for output_item in event.response.output {
                            if let OutputItem::FunctionCall(function_tool_call) = output_item {
                                function_tool_calls.push(function_tool_call);
                            }
                        }
                    }
                    _ => {
                        // Ignore other events.
                    }
                },
                Err(OpenAIError::ApiError(error)) => {
                    if error.code == Some("invalid_api_key".to_string()) {
                        eprintln!(
                            "Invalid API key. Please set the \
                                        `OPENAI_API_KEY` environment \
                                        variable.",
                        );
                    } else {
                        eprintln!("Error: {error:?}");
                    }
                    return Ok(());
                }
                Err(error) => {
                    eprintln!("Error: {error:?}");
                    return Ok(());
                }
            }
        }

        // Output a newline after the response to separate it from the
        // next prompt.
        if received_output {
            println!();
        }

        // If there are no function calls, break the loop.
        if function_tool_calls.is_empty() {
            break;
        }

        // Handle the function calls.
        for function_tool_call in function_tool_calls {
            match function_tool_call.name.as_str() {
                "run_shell_command" => {
                    function_call_outputs.push(FunctionCallOutputItemParam {
                        call_id: function_tool_call.call_id,
                        output: FunctionCallOutput::Text(serde_json::to_string(
                            &run_shell_command(serde_json::from_str(
                                &function_tool_call.arguments,
                            )?),
                        )?),
                        id: None,
                        status: None,
                    });
                }
                _ => {
                    eprintln!("Unexpected function tool call: {function_tool_call:?}");
                }
            }
        }
    }

    Ok(())
}
