use backtrace::Backtrace;
use deno_core::error::AnyError;

use clap::{Arg, Command, ArgAction};

use sjs::ScriptSource;

use std::panic;
use std::io;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AnyError> {
    let matches = Command::new("SJS")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .before_help(format!("SJS {}\n{}\n{}", clap::crate_version!(), clap::crate_authors!(), "A simple JavaScript runtime"))

        // Use '-v' instead of '-V' for the short version flag
        .disable_version_flag(true)
        .arg(Arg::new("version")
            .short('v')
            .long("version")
            .action(ArgAction::Version)
            .help("Print version")
        )

        .arg(Arg::new("verbose")
            .short('V')
            .long("verbose")
            .help("Prints verbose error messages")
            .action(ArgAction::SetTrue)
        )

        .arg(Arg::new("clear-cache")
            .short('c')
            .long("clear-cache")
            .help("Clear dependency cache and exit if no source is specified")
            .action(ArgAction::SetTrue)
        )

        .arg(Arg::new("remote")
            .short('r')
            .long("remote")
            .help("Allow running and importing URLs")
            .action(ArgAction::SetTrue)
        )

        .external_subcommand_value_parser(clap::value_parser!(String))
        .allow_external_subcommands(true)
        .subcommand_value_name("SOURCE")

        .get_matches();

    
    let verbose = matches.get_flag("verbose");
    if cfg!(not(debug_assertions)) {
        panic::set_hook(Box::new(move |info| {
            eprintln!("\x1b[93merror\x1b[0m: {}", panic_message::panic_info_message(info));
            
            if verbose {
                eprintln!("{:?}", Backtrace::new());
            } else {
                eprintln!("rerun with -V for verbose error messages");
            }
        }));
    }

    if matches.get_flag("clear-cache") {
        if let Some(path) = sjs::get_storage_directory() {
            std::fs::remove_dir_all(path.join("libs")).unwrap();
        }
        if matches.subcommand() == None {
            return Ok(())
        }
    };

    let (source, args) = match matches.subcommand() {
        Some(("-", args)) => {
            (ScriptSource::Text(read_stdin()), args.get_many::<String>("").unwrap_or_default().map(|s| s.to_string()).collect())
        }
        Some((input, args)) => {
            (ScriptSource::FileOrURL(input.to_string()), args.get_many::<String>("").unwrap_or_default().map(|s| s.to_string()).collect())
        }
        _ => {
            (ScriptSource::Text(read_stdin()), vec![])
        }
    };

    sjs::run(source, args, matches.get_flag("remote")).await    
}

fn read_stdin() -> String {
    return io::read_to_string(io::stdin()).expect("Error reading stdin")
}