mod cli;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    cli::run(cli);
}
