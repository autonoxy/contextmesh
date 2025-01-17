use std::fs;

pub fn collect_files(directory: &str, extensions: &[&str]) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(directory) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();

            // Skip hidden dirs, target, etc.
            if file_name.starts_with(".")
                || file_name == "target"
                || file_name == "node_modules"
                || file_name == "tests"
            {
                continue;
            }
            if path.is_dir() {
                files.extend(collect_files(path.to_str().unwrap(), extensions));
            } else if let Some(ext) = path.extension() {
                if extensions.contains(&ext.to_str().unwrap()) {
                    files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    files
}
