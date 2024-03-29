use anyhow::Result;
use std::{fs, path::Path};

/// Checks if a file exists and if it doesn't it initializes it.
/// Otherwise it just returns the path back
pub fn create_file_if_missing<'a>(path: &'a str, contents: &str) -> Result<&'a str> {
    if !Path::new(path).exists() {
        fs::write(path, contents)?;
    }
    Ok(path)
}

pub fn create_data_folders() {
    if !Path::new("data/markov data").exists() {
        fs::create_dir_all("data/markov data").expect("Couldn't create directory data/markov data");
    };
}
