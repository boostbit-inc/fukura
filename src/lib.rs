pub mod ui {
    pub mod browser;
    pub mod cli;
}

pub mod application {
    pub mod activity_monitor;
    pub mod claude_code;
    pub mod config_cmd;
    pub mod daemon;
    pub mod daemon_service;
    pub mod effectiveness;
    pub mod mcp;
}

pub mod domain {
    pub mod activity;
    pub mod activity_storage;
    pub mod attempt;
    pub mod models;
    pub mod ontology;
    pub mod pack;
    pub mod redaction;
}

pub mod adapter;

pub mod infrastructure {
    pub mod attempt_storage;
    pub mod config;
    pub mod directory_monitor;
    pub mod file_watcher;
    pub mod hooks;
    pub mod index;
    pub mod notification;
    pub mod remote_search;
    pub mod repo;
    pub mod sync;
}

pub mod shared {
    pub mod performance;
    pub mod time_parser;
}

pub use application::activity_monitor;
pub use application::claude_code;
pub use application::config_cmd;
pub use application::daemon;
pub use application::daemon_service;
pub use application::effectiveness;
pub use application::mcp;
pub use domain::activity;
pub use domain::activity_storage;
pub use domain::attempt;
pub use domain::models;
pub use domain::ontology;
pub use domain::pack;
pub use domain::redaction;
pub use infrastructure::attempt_storage;
pub use infrastructure::config;
pub use infrastructure::directory_monitor;
pub use infrastructure::file_watcher;
pub use infrastructure::hooks;
pub use infrastructure::index;
pub use infrastructure::notification;
pub use infrastructure::remote_search;
pub use infrastructure::repo;
pub use infrastructure::sync;
pub use shared::performance;
pub use shared::time_parser;
pub use ui::browser;
pub use ui::cli;

pub use ui::cli::run;
