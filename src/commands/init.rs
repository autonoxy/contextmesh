use std::fs;
use std::io::Write;
use std::path::Path;

use crate::commands::index;

pub fn handle_init() -> Result<(), Box<dyn std::error::Error>> {
    let dir_path = ".contextmesh";
    if !Path::new(dir_path).exists() {
        fs::create_dir(dir_path)?;
        println!("Created directory: {}", dir_path);
    } else {
        println!("Directory already exists: {}", dir_path);
    }

    let config_file_path = format!("{}/config.conf", dir_path);
    if !Path::new(&config_file_path).exists() {
        let project_path = std::env::current_dir()?;
        let mut config_file = fs::File::create(&config_file_path)?;
        writeln!(config_file, "project_path={}", project_path.display())?;
        println!("Created config file: {}", config_file_path);
    } else {
        println!("Config file already exists: {}", config_file_path);
    }

    println!("Indexing project...");
    let file = ".";
    let language = "rust";
    index::handle_index(file, language)?;

    println!("Project initialized successfully!");
    Ok(())
}
