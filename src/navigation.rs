use std::{env, path::PathBuf};

pub fn get_current_working_dir() -> String {
    match env::current_dir() {
        Ok(path) => path.display().to_string(),
        Err(e) => {
            eprintln!("Failed to get the current path: {}", e);
            "UNKNOWN".to_string()
        }
    }
}
pub fn change_directory(new_dir: &str) {
    let path = if new_dir == "~" {
        match env::var("HOME") {
            Ok(path) => PathBuf::from(path),
            Err(_) => {
                eprintln!("cd: HOME not set");
                return;
            }
        }
    } else if new_dir.starts_with("~/") {
        if let Ok(home) = env::var("HOME") {
            let mut path = PathBuf::from(home);
            path.push(&new_dir[2..]); // 去掉开头的 "~/"
            path
        } else {
            eprintln!("cd: HOME not set");
            return;
        }
    } else {
        PathBuf::from(new_dir)
    };

    match env::set_current_dir(&path) {
        Ok(_) => {}
        Err(_e) => {
            eprintln!("cd: {}: No such file or directory", new_dir);
        }
    }
}
