mod format;
mod turn;

use {
    crate::format::CodeStr,
    async_openai::{Client, config::OpenAIConfig},
    clap::{App, Arg},
    rustyline::{DefaultEditor, error::ReadlineError},
    std::{
        env,
        env::VarError,
        error::Error,
        io::{self, IsTerminal},
    },
};

// The program version
const VERSION: &str = env!("CARGO_PKG_VERSION");

// The name of the environment variable for the OpenAI API key
pub const OPENAI_API_KEY_ENV_VAR: &str = "OPENAI_API_KEY";

// Defaults
const DEFAULT_COMPACTION_THRESHOLD: u32 = 200_000;
const DEFAULT_MODEL: &str = "gpt-5.2";

// Command-line argument and option names
const COMPACTION_THRESHOLD_OPTION: &str = "compaction-threshold";
const MODEL_OPTION: &str = "model";

// This struct represents the command-line arguments.
pub struct Settings {
    pub compaction_threshold: u32,
    pub model: String,
}

// Parse the command-line arguments.
fn settings() -> Settings {
    // Set up the command-line interface.
    let matches = App::new("Shell Agent")
        .version(VERSION)
        .version_short("v")
        .author("Stephan Boyer <stephan@stephanboyer.com>")
        .about("A simple AI agent that only knows how to run shell commands.")
        .arg(
            Arg::with_name(COMPACTION_THRESHOLD_OPTION)
                .value_name("TOKENS")
                .short("c")
                .long(COMPACTION_THRESHOLD_OPTION)
                .help(&format!(
                    "Compact context when it exceeds this many tokens (default: \
                     {DEFAULT_COMPACTION_THRESHOLD})",
                )),
        )
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

    let compaction_threshold = matches
        .value_of(COMPACTION_THRESHOLD_OPTION)
        .map(str::parse::<u32>)
        .transpose()
        .unwrap_or_else(|error| {
            eprintln!("Invalid value for `--{COMPACTION_THRESHOLD_OPTION}`: {error}");
            std::process::exit(2);
        })
        .unwrap_or(DEFAULT_COMPACTION_THRESHOLD);

    // Determine which model to use.
    let model = matches
        .value_of(MODEL_OPTION)
        .unwrap_or(DEFAULT_MODEL)
        .to_owned();

    Settings {
        compaction_threshold,
        model,
    }
}

// Let the fun begin!
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Determine whether to print colored output.
    colored::control::set_override(io::stderr().is_terminal());

    // Parse the command-line arguments.
    let settings = settings();

    // Set up the OpenAI state.
    let api_key = match env::var(OPENAI_API_KEY_ENV_VAR) {
        Ok(api_key) => api_key,
        Err(VarError::NotPresent) => {
            eprintln!(
                "Please set the {} environment variable.",
                OPENAI_API_KEY_ENV_VAR.code_str(),
            );
            std::process::exit(2);
        }
        Err(VarError::NotUnicode(_)) => {
            eprintln!(
                "The {} environment variable contains invalid Unicode.",
                OPENAI_API_KEY_ENV_VAR.code_str(),
            );
            std::process::exit(2);
        }
    };
    let client = Client::with_config(OpenAIConfig::new().with_api_key(api_key));
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
                if let Err(error) = rustyline.add_history_entry(line.as_str()) {
                    eprintln!("Error recording message history: {error}");
                }

                // Run a single turn of the agent.
                match turn::run_turn(&client, &settings, &line, previous_response_id.clone()).await
                {
                    Ok(new_previous_response_id_option) => {
                        if let Some(new_previous_response_id) = new_previous_response_id_option {
                            previous_response_id = Some(new_previous_response_id);
                        }
                    }
                    Err(error) => {
                        eprintln!("Error: {error}");
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // The user wants to interrupt something, but nothing is
                // happening.
            }
            Err(ReadlineError::Eof) => {
                // The user wants to exit the loop.
                break;
            }
            Err(error) => {
                // There was a readline error. Just log it and continue.
                eprintln!("Error reading message: {error}");
            }
        }
    }

    // The loop was interrupted.
    Ok(())
}
