use regex::Regex;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, exit};

fn extract_imported_libraries(file_path: &str) -> Vec<String> {
    let content = fs::read_to_string(file_path).expect("Failed to read file");
    
    // Improved regex to handle common import patterns
    let import_regex = Regex::new(
        r"(?mx)
        ^\s*(?:from\s+([\w\.]+)\s+import|import\s+([\w\.]+)(?:\s*,\s*)?(?:\\?\s*)?(?:as\s+\w+)?)
        "
    ).expect("Invalid regex pattern");

    let mut libraries = Vec::new();
    for cap in import_regex.captures_iter(&content) {
        if let Some(m) = cap.get(1).or_else(|| cap.get(2)) {
            let lib = m.as_str().split('.').next().unwrap().to_string();
            libraries.push(lib);
        }
    }

    // Filter standard libraries using Python's built-in list
    let standard_libs = get_standard_libraries();
    libraries.retain(|lib| !standard_libs.contains(lib));
    
    libraries.sort();
    libraries.dedup();
    libraries
}

fn get_standard_libraries() -> Vec<String> {
    // Get standard libraries by querying Python directly
    let output = Command::new("python")
        .args(["-c", "import sys; print(list(sys.stdlib_module_names))"])
        .output()
        .expect("Failed to get standard libraries");
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    output_str
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().trim_matches('\'').to_string())
        .collect()
}

fn venv_exists(venv_name: &str) -> bool {
    Path::new(venv_name).exists()
}

fn create_venv(venv_name: &str) {
    println!("Creating virtual environment '{}'...", venv_name);
    let status = Command::new("python")
        .args(["-m", "venv", venv_name])
        .status()
        .expect("Failed to create virtual environment");
    
    if !status.success() {
        eprintln!("Failed to create virtual environment");
        exit(1);
    }
}

fn install_libraries(venv_name: &str, libraries: &[String]) {
    if libraries.is_empty() {
        println!("No external libraries to install");
        return;
    }

    println!("Installing libraries: {:?}", libraries);
    
    let pip_exec = if cfg!(windows) {
        format!("{}\\Scripts\\pip.exe", venv_name)
    } else {
        format!("{}/bin/pip", venv_name)
    };

    let status = Command::new(pip_exec)
        .args(["install"])
        .args(libraries)
        .status()
        .expect("Failed to install libraries");

    if !status.success() {
        eprintln!("Failed to install dependencies");
        exit(1);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        eprintln!("Usage: {} <python-file> [venv-name]", args[0]);
        eprintln!("Default venv-name: venv");
        exit(1);
    }

    let python_file = &args[1];
    let venv_name = args.get(2).map_or("venv", |s| s.as_str());

    if !Path::new(python_file).exists() {
        eprintln!("Python file not found: {}", python_file);
        exit(1);
    }

    if !venv_exists(venv_name) {
        create_venv(venv_name);
    }

    let libraries = extract_imported_libraries(python_file);
    install_libraries(venv_name, &libraries);

    println!("\nSuccess! Now run your script using:");
    if cfg!(windows) {
        println!("  {}\\Scripts\\activate && python {}", venv_name, python_file);
    } else {
        println!("  source {}/bin/activate && python {}", venv_name, python_file);
    }
}