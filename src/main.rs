use {
    async_openai::{
        Client,
        error::OpenAIError,
        types::responses::{CreateResponseArgs, ResponseStreamEvent},
    },
    clap::{App, Arg},
    futures::StreamExt,
    rustyline::{DefaultEditor, error::ReadlineError},
    std::error::Error,
};

// The program version
const VERSION: &str = env!("CARGO_PKG_VERSION");

// Defaults
const DEFAULT_MODEL: &str = "gpt-5.2";

// Command-line argument and option names
const MODEL_OPTION: &str = "model";

// Model instructions
const INSTRUCTIONS: &str = "You are a helpful assistant.";

// Let the fun begin!
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse the command-line arguments.
    let matches = App::new("Shell Agent")
        .version(VERSION)
        .version_short("v")
        .author("Stephan Boyer <stephan@stephanboyer.com>")
        .about("A simple AI agent that only knows how to run shell commands.")
        .arg(
            Arg::with_name(MODEL_OPTION)
                .value_name("MODEL")
                .short("m")
                .long(MODEL_OPTION)
                .help(&format!(
                    "Which OpenAI model to use (default: {DEFAULT_MODEL})",
                )),
        )
        .get_matches();

    // Determine which model to use.
    let model = matches.value_of(MODEL_OPTION).unwrap_or(DEFAULT_MODEL);

    // Set up the OpenAI state.
    let client = Client::new();
    let mut previous_response_id: Option<String> = None;

    // Set up the Rustyline state.
    let mut rustyline = DefaultEditor::new()?;

    // The main agent loop!
    'agent_loop: loop {
        // Read a line from the user.
        match rustyline.readline("❯ ") {
            Ok(line) => {
                // Ignore empty lines.
                if line.trim().is_empty() {
                    continue;
                }

                // Remember the user's input in case they want it later.
                rustyline.add_history_entry(line.as_str())?;

                // Build the OpenAI request.
                let mut request_builder = CreateResponseArgs::default();
                request_builder
                    .model(model)
                    .stream(true)
                    .input(line)
                    .instructions(INSTRUCTIONS)
                    .max_output_tokens(512u32);
                if let Some(ref id) = previous_response_id {
                    request_builder.previous_response_id(id);
                }
                let request = request_builder.build()?;

                // Send the request to the OpenAI API and stream the response.
                let mut stream = client.responses().create_stream(request).await?;
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(event) => match event {
                            ResponseStreamEvent::ResponseCreated(event) => {
                                // Remember the response ID for the next
                                // request.
                                previous_response_id = Some(event.response.id);
                            }
                            ResponseStreamEvent::ResponseOutputTextDelta(event) => {
                                // Output the response delta.
                                print!("{}", event.delta);
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
                            continue 'agent_loop;
                        }
                        Err(error) => {
                            eprintln!("Error: {error:?}");
                            continue 'agent_loop;
                        }
                    }
                }

                // Output a newline after the response to separate it from the
                // next prompt.
                println!();
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                // The user wants to interrupt the loop.
                break;
            }
            Err(error) => {
                // There was a readline error. Just log it and continue.
                eprintln!("Error: {error:?}");
            }
        }
    }

    // The loop was interrupted.
    Ok(())
}
