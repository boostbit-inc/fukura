use colored::Colorize;
use std::process;

#[tokio::main]
async fn main() {
    // Handle version display with correct binary name
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        let binary_name = std::env::current_exe()
            .ok()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "fukura".to_string());
        println!("{} {}", binary_name, env!("CARGO_PKG_VERSION"));
        return;
    }

    if let Err(error) = fukura::run().await {
        eprintln!("{} {}", "error:".red().bold(), error);
        process::exit(1);
    }
}
