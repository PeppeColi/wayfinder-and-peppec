//! The `notes` binary: parses arguments, reads the environment and the clock,
//! calls the library and maps its result to stderr and the exit code.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use jiff::Timestamp;

#[derive(Parser)]
#[command(
    name = "notes",
    about = "Take notes from the terminal, tag them, and find them again."
)]
struct Cli {
    /// Notebook file
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        env = "NOTES_NOTEBOOK",
        hide_env_values = true
    )]
    notebook: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add a note
    Add {
        /// Text of the note
        text: String,
        /// Tag the note (repeat for more tags)
        #[arg(short, long = "tag", value_name = "TAG")]
        tags: Vec<String>,
    },
    /// List notes, most recent first
    List,
    /// List the notes that have a tag
    Search {
        /// Tag to search for
        tag: String,
    },
    /// Write all notes to a file
    Export {
        /// Overwrite the file if it exists
        #[arg(long)]
        force: bool,
        /// File to write
        path: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let _ = writeln!(io::stderr().lock(), "error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), wayfinder_and_peppec::Error> {
    match cli.command {
        Command::Add { text, tags } => {
            let text = wayfinder_and_peppec::normalise_text(&text)?;
            let tags = wayfinder_and_peppec::normalise_tags(&tags)?;
            let now = now(std::env::var_os("NOTES_NOW"))?;
            let path = wayfinder_and_peppec::notebook_location(
                cli.notebook.as_deref(),
                dirs::data_dir().as_deref(),
            )?;
            wayfinder_and_peppec::add(&path, &text, &tags, now)
        }
        Command::List | Command::Search { .. } | Command::Export { .. } => Ok(()),
    }
}

/// The current instant: `NOTES_NOW` if set, else the real clock.
fn now(notes_now: Option<OsString>) -> Result<Timestamp, wayfinder_and_peppec::Error> {
    match notes_now {
        Some(value) => wayfinder_and_peppec::parse_now(&value.to_string_lossy()),
        None => Ok(Timestamp::now()),
    }
}
