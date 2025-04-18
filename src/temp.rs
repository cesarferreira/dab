use anyhow::Result;
use std::path::PathBuf;
use std::process::{Command, Output};

struct AdbClient {
    adb_path: PathBuf,
}

impl AdbClient {
    fn run_command(&self, args: &[&str]) -> Result<Output> {
        let output = Command::new(&self.adb_path)
            .args(args)
            .output()?;
        
        Ok(output)
    }
}

fn main() {
    println!("Hello, world!");
}
