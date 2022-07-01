mod app;
mod cli;
mod display;

use anyhow::Result;
use app::TraneApp;
use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::{ColorMode, Config, Editor};

use crate::cli::TraneCli;

fn main() -> Result<()> {
    let mut app = TraneApp::default();

    let config = Config::builder()
        .auto_add_history(true)
        .color_mode(ColorMode::Enabled)
        .history_ignore_space(true)
        .build();
    let mut rl = Editor::<()>::with_config(config);

    let history_path = std::path::Path::new(".trane_history");
    if !history_path.exists() {
        match std::fs::File::create(history_path) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to create history file: {}", e);
            }
        }
    }
    match rl.load_history(history_path) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Failed to load history file at .trane_history: {}", e);
        }
    }

    loop {
        let readline = rl.readline("\x1b[1;31mtrane >>\x1b[0m ");
        match readline {
            Ok(line) => {
                let split: Vec<&str> = line.split(" ").into_iter().collect();
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
                    Err(err) => println!("Error: {}", err),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Press CTRL-D to exit");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("EOF: Exiting");
                break;
            }
            Err(err) => {
                println!("Error: {}", err);
                break;
            }
        }
    }

    match rl.save_history(history_path) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Failed to save history to file .trane_history: {}", e);
        }
    }
    Ok(())
}
