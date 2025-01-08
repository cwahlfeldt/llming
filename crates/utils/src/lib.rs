use std::env;

pub fn get_home_dir() -> String {
    // Try $HOME for Unix-like systems first
    match env::var("HOME") {
        Ok(path) => path,
        Err(_) => {
            // Then try %USERPROFILE% for Windows
            env::var("USERPROFILE").unwrap_or_else(|_| String::from("/")) // Fallback to root if neither exists
        }
    }
}
