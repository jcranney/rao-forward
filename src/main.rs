use anyhow::Result;
use clap::Parser;
use rao_forward::*;
use std::io::{self, IsTerminal, Read, Write};

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = "build an AO simulation based on a specified config file, \
            and output the relevant simulation results."
)]
struct Args {
    /// reads input configuration json from this filename instead of standard input
    #[arg(short, long)]
    input: Option<String>,
    /// save the output results to this filename instead of standard output
    #[arg(short, long)]
    output: Option<String>,
}

fn main() -> Result<()> {
    let Args { output, input } = Args::parse();
    let system_config: Config = match input {
        None => {
            // check if stdin is terminal (problem)
            let stdinput = std::io::stdin();
            if stdinput.is_terminal() {
                return Err(anyhow::anyhow!(
                    "nothing in standard input, perhaps you meant to pass a config file with '-f'.\nSee '--help' for options"
                ));
            }
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            if buffer.is_empty() {
                eprintln!("Warning: config file not provided, and stdin empty")
            }
            buffer.parse()?
        }
        Some(filename) => {
            Config::from_file(&filename)?
        }
    };
    let system = system_config.to_system();
    let results = system.evaluate();
    match output {
        Some(filename) => {
            // save to filename
            let mut file = std::fs::File::create(filename)?;
            write!(file, "{}", results.to_string()?)?;
        }
        None => {
            // write to stdout
            println!("{}", results.to_string()?);
        }
    }
    Ok(())
}
