use std::env;

pub fn get_current_working_dir() -> String {
    match env::current_dir() {
        Ok(path) => path.display().to_string(),
        Err(e) => {
            eprintln!("Failed to get the current path: {}", e);
            "UNKNOWN".to_string()
        }
    }
}
