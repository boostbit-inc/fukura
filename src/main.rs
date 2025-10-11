use colored::Colorize;
use std::process;

#[tokio::main]
async fn main() {
    if let Err(error) = fukura::run().await {
        eprintln!("{} {}", "error:".red().bold(), error);
        process::exit(1);
    }
}
