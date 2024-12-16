use bincode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SymbolType {
    Import,
    Function,
    Struct,
    Enum,
    Variable,
    Field,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub file_path: String,
    pub line_number: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub dependencies: Vec<String>,
}

impl Symbol {
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.name);
        hasher.update(self.file_path.as_bytes());
        hasher.update(self.line_number.to_string().as_bytes());
        hasher.update(self.start_byte.to_string().as_bytes());
        hasher.update(self.end_byte.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

pub fn store_symbol(symbol: &Symbol) -> std::io::Result<()> {
    // Compute the hash and construct the directory path
    let hash = symbol.hash();
    let dir = format!(".contextmesh/objects/{}", &hash[0..2]);
    let file_name = format!("{}.bin", &hash[2..]);
    let full_path = Path::new(&dir).join(file_name);

    // Log the directory path and file path
    println!("Creating directory: {}", dir);
    println!("Storing symbol in file: {}", full_path.display());

    // Create the directory if it doesn't exist
    match fs::create_dir_all(&dir) {
        Ok(_) => println!("Successfully created directory: {}", dir),
        Err(e) => {
            eprintln!("Failed to create directory '{}': {}", dir, e);
            return Err(e);
        }
    }

    // Serialize the symbol using bincode
    let encoded: Vec<u8> = match bincode::serialize(symbol) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Serialization failed for symbol: {:?}", e);
            return Err(IoError::new(
                ErrorKind::Other,
                format!("Serialization failed: {}", e),
            ));
        }
    };

    // Write the serialized symbol to the file
    match fs::write(&full_path, encoded) {
        Ok(_) => println!("Successfully stored symbol in: {}", full_path.display()),
        Err(e) => {
            eprintln!("Failed to write symbol to '{}': {}", full_path.display(), e);
            return Err(e);
        }
    }

    Ok(())
}

pub fn delete_symbol(symbol: &Symbol) -> std::io::Result<()> {
    let directory_path = Path::new(".contextmesh")
        .join("objects")
        .join(&symbol.hash()[0..2]);

    let symbol_file_path = directory_path.join(format!("{}.bin", &symbol.hash()[2..]));

    if symbol_file_path.exists() {
        fs::remove_file(&symbol_file_path).map_err(|e| {
            eprintln!(
                "Failed to delete file '{}': {}",
                symbol_file_path.display(),
                e
            );
            e
        })?;
        println!("Successfully deleted file: {}", symbol_file_path.display());
    } else {
        println!("File does not exist: {}", symbol_file_path.display());
    }

    let is_empty = fs::read_dir(&directory_path)
        .map(|entries| entries.flatten().next().is_none())
        .unwrap_or(true);

    if is_empty {
        fs::remove_dir(&directory_path).map_err(|e| {
            eprintln!(
                "Failed to delete directory '{}': {}",
                directory_path.display(),
                e
            );
            e
        })?;
    }

    Ok(())
}

pub fn get_linked_symbols_from_objects(file: &str) -> Result<Vec<Symbol>, Box<dyn Error>> {
    let objects_dir = Path::new(".contextmesh/objects");

    if !objects_dir.exists() {
        return Err("Objects directory does not exist".into());
    }

    let linked_symbols = collect_linked_symbols(objects_dir, file)?;

    Ok(linked_symbols)
}

fn collect_linked_symbols(object_dir: &Path, file: &str) -> Result<Vec<Symbol>, Box<dyn Error>> {
    let mut linked_symbols = Vec::new();

    for subdir in fs::read_dir(object_dir)? {
        let subdir = subdir?;
        if subdir.path().is_dir() {
            linked_symbols.extend(collect_symbols_from_subdir(&subdir.path(), file)?);
        }
    }

    Ok(linked_symbols)
}

fn collect_symbols_from_subdir(
    subdir_path: &Path,
    file: &str,
) -> Result<Vec<Symbol>, Box<dyn Error>> {
    let mut symbols = Vec::new();

    for file_entry in fs::read_dir(subdir_path)? {
        let file_entry = file_entry?;
        let file_path = file_entry.path();

        if is_binary_file(&file_path) {
            if let Ok(symbol) = read_symbol_from_bin_file(&file_path) {
                if symbol.file_path == file {
                    symbols.push(symbol);
                }
            }
        }
    }

    Ok(symbols)
}

fn is_binary_file(file_path: &Path) -> bool {
    file_path.extension().map(|fe| fe == "bin").unwrap_or(false)
}

fn read_symbol_from_bin_file(path: &Path) -> Result<Symbol, Box<dyn Error>> {
    let data = fs::read(path)?;
    let symbol: Symbol = bincode::deserialize(&data)?;

    Ok(symbol)
}

