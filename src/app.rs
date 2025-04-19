//! Contains the App struct and related logic.

pub struct App {
    pub package_name: String,
    pub app_name: String,
}

impl App {
    pub fn new(package_name: &str, app_name: &str) -> Self {
        Self {
            package_name: package_name.to_string(),
            app_name: app_name.to_string(),
        }
    }
} 