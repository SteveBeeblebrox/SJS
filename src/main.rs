use std::panic;

use backtrace::Backtrace;
use deno_core::error::AnyError;

use clap::{Arg, App};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AnyError> {

    let matches = App::new("SJS")
        .version(clap::crate_version!())
        .version_short("v")
        .author(clap::crate_authors!())
        .about("A simple JavaScript runtime")

        .arg(Arg::with_name("input-name")
            .short("i")
            .long("input-name")
            .value_name("INPUT-NAME")
            .help("Sets the file name for the input when reading from stdin (ignored otherwise)")
            .takes_value(true)
        )

        .arg(Arg::with_name("verbose")
            .short("V")
            .long("verbose")
            .help("Prints verbose error messages")
        )

        .arg(Arg::with_name("INPUT")
            .help("Sets the input file to execute (Leave blank or set to '-' to read from stdin)")
            .index(1)
        )
        .get_matches();

    let verbose = matches.occurrences_of("verbose") > 0;
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

    sjs::run().await
}