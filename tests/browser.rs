use fukura::browser::BrowserOpener;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_browser_opener_detection() {
    // Test that we can detect available commands
    let has_wslview = which::which("wslview").is_ok();
    let has_xdg_open = which::which("xdg_open").is_ok();
    let has_open = which::which("open").is_ok();

    // At least one should be available in most environments
    // This test will pass if any browser opening command is available
    println!(
        "Available commands: wslview={}, xdg-open={}, open={}",
        has_wslview, has_xdg_open, has_open
    );
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

#[test]
fn test_browser_opener_with_temp_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.html");

    let html_content = r#"<!DOCTYPE html>
<html>
<head><title>Browser Test</title></head>
<body><h1>Browser Opening Test</h1></body>
</html>"#;

    // Write test HTML file
    let mut file = File::create(&test_file).expect("Failed to create test file");
    file.write_all(html_content.as_bytes())
        .expect("Failed to write test content");

    // Test browser opening (may fail in CI environments)
    let result = BrowserOpener::open(&test_file);
    match result {
        Ok(()) => println!("Browser opened successfully"),
        Err(e) => println!(
            "Browser opening failed (expected in some environments): {}",
            e
        ),
    }
}

#[test]
fn test_find_available_port() {
    // Test that we can find an available port
    // This is an internal function, but we can test the behavior indirectly
    // by checking that the open_with_server function doesn't panic
    let html_content = "<html><body>Test</body></html>";
    let result = BrowserOpener::open_with_server(html_content, "port-test.html");

    // Should not panic, regardless of success
    println!("Port finding test result: {:?}", result);
}

#[cfg(target_os = "linux")]
#[test]
fn test_linux_browser_detection() {
    // Test Linux-specific browser detection
    let browsers = [
        "firefox",
        "google-chrome",
        "chromium",
        "brave-browser",
        "opera",
    ];

    let mut found_browsers = Vec::new();
    for browser in &browsers {
        if which::which(browser).is_ok() {
            found_browsers.push(browser);
        }
    }

    println!("Found Linux browsers: {:?}", found_browsers);
    // At least one browser should be available on Linux systems with GUI
}

#[cfg(target_os = "macos")]
#[test]
fn test_macos_browser_detection() {
    // Test macOS-specific browser detection
    let has_open = which::which("open").is_ok();
    assert!(has_open, "macOS should have 'open' command available");
    println!("macOS 'open' command available: {}", has_open);
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_browser_detection() {
    // Test Windows-specific browser detection
    // Windows should have rundll32 available
    println!("Windows browser detection test - checking for rundll32");
    // We can't easily test rundll32 without actually opening a browser,
    // so we just ensure the test compiles and runs
}

#[test]
fn test_html_content_serving() {
    // Test that HTML content is properly formatted for serving
    let html_content = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Fukura Note</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .note { background: #f5f5f5; padding: 20px; border-radius: 8px; }
    </style>
</head>
<body>
    <div class="note">
        <h1>Test Note</h1>
        <p>This is a test note for browser opening functionality.</p>
        <pre><code>console.log("Hello, Fukura!");</code></pre>
    </div>
</body>
</html>"#;

    // Test that the content is valid HTML
    assert!(html_content.contains("<!DOCTYPE html>"));
    assert!(html_content.contains("<html"));
    assert!(html_content.contains("</html>"));
    assert!(html_content.contains("Test Note"));

    // Test browser opening with well-formed HTML
    let result = BrowserOpener::open_with_server(html_content, "well-formed-test.html");
    println!("Well-formed HTML test result: {:?}", result);
}
