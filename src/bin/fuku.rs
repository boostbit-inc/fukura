use colored::Colorize;

#[tokio::main]
async fn main() {
    if let Err(error) = fukura::run().await {
        eprintln!("{} {}", "error:".red().bold(), error);
        std::process::exit(1);
    }
}
