# Shell Agent

[![Build status](https://github.com/stepchowfun/shell-agent/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/stepchowfun/shell-agent/actions?query=branch%3Amain)

This is a simple AI agent that only knows how to run shell commands.

![A demo animation.](https://raw.githubusercontent.com/stepchowfun/shell-agent/main/demo.gif)

## Usage

Once Shell Agent is [installed](#installation-instructions), you can run it from the command line as follows:

```sh
shell-agent
```

OpenAI is the only supported AI provider for now. The API key must be provided via the `OPENAI_API_KEY` environment variable.

Here are the supported command-line options:

```
USAGE:
    shell-agent [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -v, --version    Prints version information

OPTIONS:
    -c, --compaction-threshold <TOKENS>    Compact context when it exceeds this many tokens (default: 200000)
    -m, --model <MODEL>                    Which OpenAI model to use (default: gpt-5.2)
```

## Installation instructions

### Installation on macOS or Linux (AArch64 or x86-64)

If you're running macOS or Linux (AArch64 or x86-64), you can install Shell Agent with this command:

```sh
curl https://raw.githubusercontent.com/stepchowfun/shell-agent/main/install.sh -LSfs | sh
```

The same command can be used again to update to the latest version.

The installation script supports the following optional environment variables:

- `VERSION=x.y.z` (defaults to the latest version)
- `PREFIX=/path/to/install` (defaults to `/usr/local/bin`)

For example, the following will install Shell Agent into the working directory:

```sh
curl https://raw.githubusercontent.com/stepchowfun/shell-agent/main/install.sh -LSfs | PREFIX=. sh
```

If you prefer not to use this installation method, you can download the binary from the [releases page](https://github.com/stepchowfun/shell-agent/releases), make it executable (e.g., with `chmod`), and place it in some directory in your [`PATH`](https://en.wikipedia.org/wiki/PATH_\(variable\)) (e.g., `/usr/local/bin`).

### Installation on Windows (AArch64 or x86-64)

If you're running Windows (AArch64 or x86-64), download the latest binary from the [releases page](https://github.com/stepchowfun/shell-agent/releases) and rename it to `shell-agent` (or `shell-agent.exe` if you have file extensions visible). Create a directory called `Shell Agent` in your `%PROGRAMFILES%` directory (e.g., `C:\Program Files\Shell Agent`), and place the renamed binary in there. Then, in the "Advanced" tab of the "System Properties" section of Control Panel, click on "Environment Variables..." and add the full path to the new `Shell Agent` directory to the `PATH` variable under "System variables". Note that the `Program Files` directory might have a different name if Windows is configured for a language other than English.

To update an existing installation, simply replace the existing binary.

### Installation with Cargo

If you have [Cargo](https://doc.rust-lang.org/cargo/), you can install Shell Agent as follows:

```sh
cargo install shell-agent
```

You can run that command with `--force` to update an existing installation.
