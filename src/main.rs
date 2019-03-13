mod errors;
mod commands;

use clap::{App, Arg, SubCommand};
use std::path::Path;
use error_chain::{bail, quick_main};

use crate::errors::*;
use crate::commands::workon::work_on;

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn run() -> Result<()> {
    let matches = App::new("gut")
        .version(VERSION)
        .author("Daniel Vigovszky <daniel.vigovszky@gmail.com>")
        .about("Some Git/GitHub commands")
        .arg(Arg::with_name("in")
            .help("The repository's directory, if not the current one")
            .long("in")
            .takes_value(true)
            .value_name("PATH")
        )
        .subcommand(SubCommand::with_name("work-on")
            .about("specifying which feature you are working on")
            .arg(clap::Arg::with_name("name")
                .required(true)
                .index(1)))
        .get_matches();

    let dir = match matches.value_of("in") {
        None => std::env::current_dir().unwrap(),
        Some(path) => Path::new(path).to_path_buf()
    };

    match matches.subcommand() {
        ("work-on", Some(sub)) => {
            let name: Result<&str> = sub.value_of("name").ok_or("Name parameter is missing".into());
            work_on(&dir, name?)?;
        }
        (other, _) => {
            bail!("Invalid command: {}", other);
        }
    }

    Ok(())
}

quick_main!(run);

