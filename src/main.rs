//! A command-line interface for Trane.

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
        Ok(_) => (),
        Err(e) => {
            eprintln!("Failed to load history file at .trane_history: {e}");
        }
    }

    print!("{}", TraneApp::startup_message());
    loop {
        let readline = rl.readline("trane >> ");

        match readline {
            Ok(line) => {
                if line.starts_with('#') || line.eq("") {
                    continue;
                };
                let split: Vec<&str> = line.split(' ').collect();
                let mut args = if !split.is_empty() && split[0] == "trane" {
                    vec![]
                } else {
                    vec!["trane"]
                };
                args.extend(split);

                let cli = TraneCli::try_parse_from(args.iter());
                if cli.is_err() {
                    println!("{}", cli.unwrap_err());
                    continue;
                }

                match cli.unwrap().execute_subcommand(&mut app) {
                    Ok(()) => (),
                    Err(err) => println!("Error: {err:#}"),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Press CTRL-D to exit");
                continue;
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
        Ok(_) => (),
        Err(e) => {
            eprintln!("Failed to save history to file .trane_history: {e}");
        }
    }
    Ok(())
}
