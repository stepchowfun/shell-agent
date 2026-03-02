use clap::App;

// The program version
const VERSION: &str = env!("CARGO_PKG_VERSION");

// Let the fun begin!
fn main() {
    // Parse the command-line arguments.
    App::new("Shell Agent")
        .version(VERSION)
        .version_short("v")
        .author("Stephan Boyer <stephan@stephanboyer.com>")
        .about("A simple AI agent that only knows how to run shell commands.")
        .get_matches();

    // Greet the user.
    println!("Hello, World!");
}
