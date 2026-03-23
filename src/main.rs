mod format;
mod turn;

use {
    async_openai::Client,
    clap::{App, Arg},
    rustyline::{DefaultEditor, error::ReadlineError},
    std::{
        error::Error,
        io::{self, IsTerminal},
    },
};

// The program version
const VERSION: &str = env!("CARGO_PKG_VERSION");

// Defaults
const DEFAULT_MODEL: &str = "gpt-5.2";

// This struct represents the command-line arguments.
pub struct Settings {
    pub model: String,
}

// Command-line argument and option names
const MODEL_OPTION: &str = "model";

// Parse the command-line arguments.
fn settings() -> Settings {
    // Set up the command-line interface.
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
    let model = matches
        .value_of(MODEL_OPTION)
        .unwrap_or(DEFAULT_MODEL)
        .to_owned();

    Settings { model }
}

// Let the fun begin!
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Determine whether to print colored output.
    colored::control::set_override(io::stderr().is_terminal());

    // Parse the command-line arguments.
    let settings = settings();

    // Set up the OpenAI state.
    let client = Client::new();
    let mut previous_response_id: Option<String> = None;

    // Set up the Rustyline state.
    let mut rustyline = DefaultEditor::new()?;

    // The main agent loop!
    loop {
        // Read a line from the user.
        match rustyline.readline("❯ ") {
            Ok(line) => {
                // Ignore empty lines.
                if line.trim().is_empty() {
                    continue;
                }

                // Remember the user's input in case they want it later.
                rustyline.add_history_entry(line.as_str())?;

                // Run a single turn of the agent.
                turn::run_turn(&client, &settings, &line, &mut previous_response_id).await?;
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
