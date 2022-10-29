use clap::Parser;
use command::Command;

mod command;
mod repository;

#[derive(Parser, Debug, Clone)]
// #[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    println!("{:?}", args);

    args.command.perform()
}
