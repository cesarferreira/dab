//! Contains the App struct and related logic.

pub struct App {
    pub package_name: String,
}

impl App {
    pub fn new(package_name: &str) -> Self {
        Self {
            package_name: package_name.to_string(),
        }
    }
}
