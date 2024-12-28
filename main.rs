#![windows_subsystem = "windows"]
use anyhow::Result;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::mpsc;
use std::path::PathBuf;
use wry::{
    application::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::{Theme, WindowBuilder},
    },
    webview::WebViewBuilder,
};
use dirs;
use notify_rust::Notification;
use std::env;
use winreg::{enums::*, RegKey};
use winreg;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct PersistentCookieStore {
    cookies: HashMap<String, String>,
}

fn is_launched_from_startup() -> bool {
    env::args().nth(1)
        .map(|arg| arg == "--startup")
        .unwrap_or(false)
}

fn get_initial_url() -> String {
    if is_launched_from_startup() {
        "https://youtu.be/".to_string()
    } else {
        "https://youtu.be/".to_string()
    }
}

fn add_to_startup() -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = env::current_exe()?;
    let mut exe_path_str = exe_path.to_str().ok_or("Invalid path")?.to_string();
    exe_path_str.push_str(" --startup");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
    let (key, _) = hkcu.create_subkey(path)?;
    
    key.set_value("YouTube", &exe_path_str)?;
    
    Ok(())
}

impl PersistentCookieStore {
    fn get_cookie_path() -> PathBuf {
        let mut path = dirs::data_local_dir().expect("Could not find local data directory");
        path.push("WebviewBrowser");
        fs::create_dir_all(&path).expect("Could not create cookies directory");
        path.push("cookies.json");
        path
    }

    fn load() -> Self {
        let path = Self::get_cookie_path();
        if path.exists() {
            let content = fs::read_to_string(&path).expect("Could not read cookies file");
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) -> Result<()> {
        let path = Self::get_cookie_path();
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    fn add_cookies_from_header(&mut self, cookie_header: &str) {
        for cookie in cookie_header.split(';') {
            let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
            if parts.len() == 2 {
                self.cookies.insert(parts[0].to_string(), parts[1].to_string());
            }
        }
        let _ = self.save();
    }

    fn get_cookie_header(&self) -> String {
        self.cookies
            .iter()
            .map(|(name, value)| format!("{}={}", name, value))
            .collect::<Vec<String>>()
            .join("; ")
    }
}

enum AppMessage {
    NavigateToUrl(String),
    ConnectionError(String),
    ShowNotification { title: String, message: String },
}

fn get_webview_data_directory() -> PathBuf {
    if let Some(mut path) = dirs::data_local_dir() {
        path.push("TrendsIgniteWebView");
        if !path.exists() {
            std::fs::create_dir_all(&path).expect("Could not create WebView2 data directory");
        }
        path
    } else {
        PathBuf::from("TrendsIgniteWebView")
    }
}

fn show_notification(title: &str, message: &str) -> Result<()> {
    Notification::new()
        .summary("YouTube")
        .body("Notification")
        .icon("trendsignite")
        .show()?;
    Ok(())
}

fn main() -> Result<()> {
    let _cookie_store = PersistentCookieStore::load();
    let _client = Client::new();
    let initial_url = get_initial_url();
    let css_content = include_str!("style.css");
    let script_content = include_str!("script.js");
    let main_html = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>YouTube</title>
    <link href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.0.0-beta3/css/all.min.css" rel="stylesheet">
    <style>{}</style>
    <script>
        window.onload = function() {{
            window.location.href = '{}';
        }};
    </script>
</head>
<body>
    <div id="loading-container" class="container">
        <div class="logo">YouTube</div>
        <div class="loading-wrapper">
            <div class="terminal-loader">
                <div class="terminal-header">
                    <div class="terminal-title">Status</div>
                    <div class="terminal-controls">
                        <div class="control close"></div>
                        <div class="control minimize"></div>
                        <div class="control maximize"></div>
                    </div>
                </div>
                <div class="text">Loading...</div>
            </div>
           <p>Starting...</p>
        </div>
        <div class="footer">Software by Fluctuss</div>
    </div>

    <div id="error-container" class="container hidden">
        <div class="logo">YouTube</div>
        <div id="error-message" class="error-message"></div>
        <div class="footer">Software by Fluctuss</div>
    </div>

<script>{}</script>
</body>
</html>
"#,
        css_content, initial_url, script_content
    );
    
    let event_loop = EventLoop::new();
    let mut window_builder = WindowBuilder::new()
        .with_title("YouTube")
        .with_transparent(false)
        .with_inner_size(wry::application::dpi::LogicalSize::new(1280.0, 1024.0))
        .with_theme(Some(Theme::Dark));
    
    if is_launched_from_startup() {
        window_builder = window_builder.with_visible(false);
    }
    
    let window = window_builder.build(&event_loop)?;

    if !is_launched_from_startup() {
        window.set_maximized(true);
    }

    let (tx, rx) = mpsc::channel();
    let webview = WebViewBuilder::new(window)?
        .with_html(main_html)?
        .with_ipc_handler(move |_window, msg| {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&msg) {
                if let Some(notification) = msg.get("notification") {
                    if let (Some(title), Some(message)) = (
                        notification.get("title").and_then(|v| v.as_str()),
                        notification.get("message").and_then(|v| v.as_str()),
                    ) {
                        tx.send(AppMessage::ShowNotification {
                            title: title.to_string(),
                            message: message.to_string(),
                        })
                        .unwrap_or_else(|e| eprintln!("Failed to send notification: {:?}", e));
                    }
                }
            }
        })
        .build()?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
        
        if let Err(e) = add_to_startup() {
            eprintln!("Failed to add to startup: {:?}", e);
        }
        
        if let Ok(message) = rx.try_recv() {
            match message {
                AppMessage::NavigateToUrl(received_url) => {
                    if let Err(e) = webview.evaluate_script(&format!("window.location.href = '{}';", received_url)) {
                        eprintln!("Navigation error: {:?}", e);
                    }
                },
                AppMessage::ConnectionError(error_msg) => {
                    if let Err(e) = webview.evaluate_script(&format!("displayError('{}');", error_msg)) {
                        eprintln!("Error display failed: {:?}", e);
                    }
                },
                AppMessage::ShowNotification { title, message } => {
                    if let Err(e) = show_notification(&title, &message) {
                        eprintln!("Failed to show notification: {:?}", e);
                    }
                }
            }
        }
    });
}