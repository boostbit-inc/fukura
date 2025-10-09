use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Cross-platform browser opening functionality
pub struct BrowserOpener;

impl BrowserOpener {
    /// Open a file or URL in the default browser with intelligent fallbacks
    pub fn open(path: &Path) -> Result<()> {
        // Try multiple strategies in order of preference
        let strategies = [
            Self::try_wslview,
            Self::try_xdg_open,
            Self::try_open_command,
            Self::try_browser_env,
            Self::try_system_default,
        ];

        for strategy in &strategies {
            if let Ok(()) = strategy(path) {
                return Ok(());
            }
        }

        // If all strategies fail, provide helpful error message
        Err(anyhow::anyhow!(
            "Could not open browser. Please manually open: {}",
            path.display()
        ))
    }

    /// Try WSL2's wslview command (Windows Subsystem for Linux)
    fn try_wslview(path: &Path) -> Result<()> {
        if which::which("wslview").is_err() {
            return Err(anyhow::anyhow!("wslview not found"));
        }

        let status = Command::new("wslview")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("Failed to execute wslview")?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("wslview failed with exit code: {}", status))
        }
    }

    /// Try xdg-open (Linux desktop environments)
    fn try_xdg_open(path: &Path) -> Result<()> {
        if which::which("xdg-open").is_err() {
            return Err(anyhow::anyhow!("xdg-open not found"));
        }

        let status = Command::new("xdg-open")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("Failed to execute xdg-open")?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "xdg-open failed with exit code: {}",
                status
            ))
        }
    }

    /// Try the `open` command (macOS)
    fn try_open_command(path: &Path) -> Result<()> {
        if which::which("open").is_err() {
            return Err(anyhow::anyhow!("open command not found"));
        }

        let status = Command::new("open")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("Failed to execute open")?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "open command failed with exit code: {}",
                status
            ))
        }
    }

    /// Try using the BROWSER environment variable
    fn try_browser_env(path: &Path) -> Result<()> {
        let browser = std::env::var("BROWSER").context("BROWSER environment variable not set")?;

        let status = Command::new(&browser)
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("Failed to execute browser from BROWSER env var")?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Browser from BROWSER env var failed with exit code: {}",
                status
            ))
        }
    }

    /// Try system-specific default browser detection
    fn try_system_default(path: &Path) -> Result<()> {
        if cfg!(target_os = "windows") {
            Self::try_windows_default(path)
        } else if cfg!(target_os = "macos") {
            Self::try_macos_default(path)
        } else if cfg!(target_os = "linux") {
            Self::try_linux_default(path)
        } else {
            Err(anyhow::anyhow!("Unsupported operating system"))
        }
    }

    /// Try Windows default browser
    fn try_windows_default(path: &Path) -> Result<()> {
        let status = Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", &path.to_string_lossy()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("Failed to execute rundll32")?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Windows default browser failed with exit code: {}",
                status
            ))
        }
    }

    /// Try macOS default browser
    fn try_macos_default(path: &Path) -> Result<()> {
        Self::try_open_command(path)
    }

    /// Try Linux default browser
    fn try_linux_default(path: &Path) -> Result<()> {
        // Try common Linux browsers
        let browsers = [
            "firefox",
            "google-chrome",
            "chromium",
            "brave-browser",
            "opera",
        ];

        for browser in &browsers {
            if which::which(browser).is_ok() {
                let status = Command::new(browser)
                    .arg(path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .context("Failed to execute Linux browser")?;

                if status.success() {
                    return Ok(());
                }
            }
        }

        Err(anyhow::anyhow!("No suitable Linux browser found"))
    }

    /// Start a local HTTP server and open the URL in browser
    pub fn open_with_server(html_content: &str, filename: &str) -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(filename);

        // Write HTML content to temporary file
        std::fs::write(&file_path, html_content).context("Failed to write HTML file")?;

        // Try to open the file directly first
        if Self::open(&file_path).is_ok() {
            return Ok(());
        }

        // If direct opening fails, start a local server
        Self::start_local_server(&file_path)
    }

    /// Start a local HTTP server to serve the HTML file
    fn start_local_server(file_path: &Path) -> Result<()> {
        let port = Self::find_available_port()?;
        let url = format!("http://localhost:{}", port);

        // Start a simple HTTP server in a separate thread
        let server_path = file_path.to_path_buf();
        thread::spawn(move || {
            if let Err(e) = Self::run_http_server(port, &server_path) {
                eprintln!("HTTP server error: {}", e);
            }
        });

        // Wait a moment for the server to start
        thread::sleep(Duration::from_millis(500));

        // Try to open the URL in browser
        if Self::open_url(&url).is_err() {
            // If opening fails, print the URL for manual opening
            println!("🌐 Please open this URL in your browser: {}", url);
            println!(" Or open this file directly: {}", file_path.display());
        }

        Ok(())
    }

    /// Find an available port for the HTTP server
    fn find_available_port() -> Result<u16> {
        use std::net::{SocketAddr, TcpListener};

        // Try ports from 8080 to 8090
        for port in 8080..8090 {
            if let Ok(listener) = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))) {
                drop(listener);
                return Ok(port);
            }
        }

        Err(anyhow::anyhow!("No available ports found"))
    }

    /// Run a simple HTTP server to serve the HTML file
    fn run_http_server(port: u16, file_path: &Path) -> Result<()> {
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;

        // Read the HTML content
        let html_content = std::fs::read_to_string(file_path)?;
        let content_length = html_content.len();

        for stream in listener.incoming() {
            let stream = stream?;
            let html_content = html_content.clone();

            thread::spawn(move || {
                if let Err(e) = Self::handle_http_request(stream, &html_content, content_length) {
                    eprintln!("HTTP request error: {}", e);
                }
            });
        }

        Ok(())
    }

    /// Handle HTTP request
    fn handle_http_request(
        mut stream: std::net::TcpStream,
        html_content: &str,
        content_length: usize,
    ) -> Result<()> {
        use std::io::{Read, Write};

        let mut buffer = [0; 1024];
        let _bytes_read = stream.read(&mut buffer)?;

        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: text/html; charset=utf-8\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            content_length, html_content
        );

        stream.write_all(response.as_bytes())?;
        stream.flush()?;
        Ok(())
    }

    /// Open a URL in the browser
    fn open_url(url: &str) -> Result<()> {
        if cfg!(target_os = "windows") {
            Command::new("rundll32")
                .args(["url.dll,FileProtocolHandler", url])
                .status()?;
        } else if cfg!(target_os = "macos") {
            Command::new("open").arg(url).status()?;
        } else {
            // Try xdg-open first, then wslview
            if which::which("xdg-open").is_ok() {
                Command::new("xdg-open").arg(url).status()?;
            } else if which::which("wslview").is_ok() {
                Command::new("wslview").arg(url).status()?;
            } else {
                return Err(anyhow::anyhow!("No suitable command to open URL"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_opener_detection() {
        // Test that we can detect available commands
        let has_wslview = which::which("wslview").is_ok();
        let has_xdg_open = which::which("xdg-open").is_ok();
        let has_open = which::which("open").is_ok();

        // At least one should be available in most environments
        assert!(has_wslview || has_xdg_open || has_open);
    }

    #[test]
    fn test_open_with_server() {
        let html_content = r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body><h1>Test Page</h1></body>
</html>"#;

        // This should not panic, even if browser opening fails
        let result = BrowserOpener::open_with_server(html_content, "test.html");
        // We don't assert success because browser opening might fail in test environment
        println!("Open with server result: {:?}", result);
    }
}
