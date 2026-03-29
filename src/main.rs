mod format;
mod turn;

use {
    crate::{
        format::CodeStr,
        turn::{system_message, user_message},
    },
    async_openai::{Client, config::OpenAIConfig, types::responses::InputItem},
    clap::{App, Arg},
    colored::{Colorize, control::SHOULD_COLORIZE},
    rustyline::{DefaultEditor, error::ReadlineError},
    std::{
        env::{self, VarError},
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

// The welcome message from the agent
const WELCOME_MESSAGE: &str = "Hi, I’m Shell Agent!";

// The maximum size of errors to present to the model
const MAX_ERROR_CODE_UNITS: usize = 5_000;

// This struct represents the command-line arguments.
pub struct Settings {
    pub compaction_threshold: u32,
    pub model: String,
}

// Get instructions for the model.
/// # Errors
///
/// Will return `Err` if there was a problem identifying the current directory.
pub fn get_instructions() -> Result<String, io::Error> {
    Ok(format!(
        "You are a helpful command-line assistant named Shell Agent that can \
run shell commands.

The operating system is `{}`. The current directory is `{}`.

The user can quit by pressing CTRL+D. If the user asks to quit, \
inform them about that shortcut.",
        std::env::consts::OS,
        std::env::current_dir()?.display(),
    ))
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

    // Set up the Rustyline state.
    let mut rustyline = DefaultEditor::new()?;

    // Start the conversation.
    println!("{WELCOME_MESSAGE}");
    let mut conversation: Vec<InputItem> = vec![system_message(WELCOME_MESSAGE)];

    // The main agent loop!
    loop {
        // Add a blank line before the prompt for readability.
        println!();

        // Read a line from the user.
        match rustyline.readline(&if SHOULD_COLORIZE.should_colorize() {
            format!("{}", "❯ ".yellow())
        } else {
            "❯ ".to_owned()
        }) {
            Ok(line) => {
                // Ignore empty lines.
                if line.trim().is_empty() {
                    continue;
                }

                // Remember the user's input in case they want it later.
                if let Err(error) = rustyline.add_history_entry(line.as_str()) {
                    eprintln!("Error recording message history: {error}");
                }

                // Add the user's message to the conversation.
                conversation.push(user_message(&line));

                // Run a single turn of the agent.
                match turn::run_turn(&client, &settings, &conversation).await {
                    Ok(new_conversation) => {
                        conversation = new_conversation;
                    }
                    Err(error) => {
                        let error_str = format!("{error}");
                        eprintln!("Error: {error_str}");
                        conversation.push(system_message(&format!(
                            "I encountered the following error: {}",
                            if error_str.len() > MAX_ERROR_CODE_UNITS {
                                format!(
                                    "{}…",
                                    error_str
                                        .chars()
                                        .take(MAX_ERROR_CODE_UNITS - 1)
                                        .collect::<String>(),
                                )
                            } else {
                                error_str
                            },
                        )));
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
