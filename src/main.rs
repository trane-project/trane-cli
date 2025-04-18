//! A command-line interface for Trane.

// Allow pedantic warnings but disable some that are not useful.
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::needless_raw_string_hashes)]

mod app;
mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
mod cli;
mod display;
mod helper;

use anyhow::Result;
use app::TraneApp;
use clap::Parser;
use helper::MyHelper;
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{ColorMode, Config, Editor};

use crate::cli::TraneCli;

/// The entry-point for the command-line interface.
fn main() -> Result<()> {
    let mut app = TraneApp::default();

    let config = Config::builder()
        .auto_add_history(true)
        .max_history_size(2500)?
        .color_mode(ColorMode::Enabled)
        .history_ignore_space(true)
        .build();

    let mut rl = Editor::<MyHelper, FileHistory>::with_config(config)?;
    let helper = MyHelper::new();
    rl.set_helper(Some(helper));

    let history_path = std::path::Path::new(".trane_history");
    if !history_path.exists() {
        match std::fs::File::create(history_path) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to create history file: {e}");
            }
        }
    }
    match rl.load_history(history_path) {
        Ok(()) => (),
        Err(e) => {
            eprintln!("Failed to load history file at .trane_history: {e}");
        }
    }

    print!("{}", TraneApp::startup_message());
    loop {
        let readline = rl.readline("trane >> ");

        match readline {
            Ok(line) => {
                // Trim any blank space from the line.
                let line = line.trim();

                // Ignore comments and empty lines.
                if line.starts_with('#') || line.is_empty() {
                    continue;
                }

                // Split the line into a vector of arguments. Add an initial argument with value
                // "trane" if the line doesn't have it, so the parser can recognize the input.
                let split: Vec<&str> = line.split(' ').collect();
                let mut args = if !split.is_empty() && split[0] == "trane" {
                    vec![]
                } else {
                    vec!["trane"]
                };
                args.extend(split);

                // Parse the arguments.
                let cli = TraneCli::try_parse_from(args.iter());
                if cli.is_err() {
                    println!("{}", cli.unwrap_err());
                    continue;
                }

                // Execute the subcommand.
                match cli.unwrap().execute_subcommand(&mut app) {
                    Ok(continue_execution) => {
                        if continue_execution {
                            continue;
                        }
                        break;
                    }
                    Err(err) => println!("Error: {err:#}"),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Press CTRL-D or use the quit command to exit");
            }
            Err(ReadlineError::Eof) => {
                // Submit the current score before exiting. Ignore the error because it's not
                // guaranteed an instance of Trane is open.
                let _ = app.submit_current_score();

                println!("EOF: Exiting");
                break;
            }
            Err(err) => {
                println!("Error: {err:#}");
                break;
            }
        }
    }

    match rl.save_history(history_path) {
        Ok(()) => (),
        Err(e) => {
            eprintln!("Failed to save history to file .trane_history: {e}");
        }
    }
    Ok(())
}
