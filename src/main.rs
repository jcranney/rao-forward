use clap::Parser;
use rao_forward::Config;
use anyhow::Result;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// config file name
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

fn main() -> Result<()> {
    let Args { config } = Args::parse();
    let config_blank = Config::new();
    // println!("{:?}", config_blank);
    // println!("{}", config_blank.to_string()?);
    config_blank.to_file("config.toml")?;
    let config = Config::from_file(&config)?;
    let system = config.to_system();
    system.outputs[0].evaluate();
    Ok(())
}