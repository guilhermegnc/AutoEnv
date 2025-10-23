use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, exit};
use toml::Value;

fn load_mapping(path: &Path) -> io::Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    
    if path.exists() {
        let content = fs::read_to_string(path)?;
        
        let parsed: Value = content.parse().map_err(|e: toml::de::Error| {
            eprintln!("Erro ao analisar TOML: {}", e);
            io::Error::new(io::ErrorKind::InvalidData, e.to_string())
        })?;

        if let Value::Table(table) = parsed {
            for (key, value) in table {
                if let Value::String(package) = value {
                    map.insert(key, package);
                }
            }
        }
    }
    
    Ok(map)
}

fn find_mapping_file() -> PathBuf {
    let exe_dir = env::current_exe()
        .expect("Falha ao obter caminho do executável")
        .parent()
        .expect("Falha ao obter diretório do executável")
        .to_path_buf();
    
    let mapping_in_exe_dir = exe_dir.join("mapeamento.toml");
    if mapping_in_exe_dir.exists() {
        return mapping_in_exe_dir;
    }

    let project_dir = exe_dir
        .parent().and_then(|p| p.parent()).and_then(|p| p.parent());
    
    if let Some(project_dir) = project_dir {
        let mapping_in_project = project_dir.join("mapeamento.toml");
        if mapping_in_project.exists() {
            return mapping_in_project;
        }
    }

    exe_dir.join("mapeamento.toml")
}

/// Verifica se um módulo é local ao projeto
/// Procura por arquivos .py ou diretórios com o nome do módulo
fn is_local_module(module_name: &str, file_dir: &Path, project_root: &Path) -> bool {
    // Verifica no diretório do arquivo atual
    let py_file_in_dir = file_dir.join(format!("{}.py", module_name));
    if py_file_in_dir.exists() {
        return true;
    }
    
    let package_in_dir = file_dir.join(module_name);
    if package_in_dir.is_dir() {
        return true;
    }
    
    // Verifica na raiz do projeto
    let py_file_in_root = project_root.join(format!("{}.py", module_name));
    if py_file_in_root.exists() {
        return true;
    }
    
    let package_in_root = project_root.join(module_name);
    if package_in_root.is_dir() {
        return true;
    }
    
    // Verifica em diretórios comuns de projetos Python
    for common_dir in &["src", "lib", "app", "core"] {
        let common_path = project_root.join(common_dir);
        if common_path.exists() {
            let py_file = common_path.join(format!("{}.py", module_name));
            if py_file.exists() {
                return true;
            }
            
            let package = common_path.join(module_name);
            if package.is_dir() {
                return true;
            }
        }
    }
    
    false
}

fn extract_imported_libraries(
    file_path: &str, 
    mapping: &HashMap<String, String>,
    project_root: &Path
) -> Vec<String> {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Warning: Failed to read file {}", file_path);
            return Vec::new();
        }
    };

    let file_dir = Path::new(file_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));

    let import_regex = Regex::new(
        r#"(?imx)
          ^ \s*
          (?: from \s+ ([\w\.]+) \s+ import | import \s+ ([\w\.]+) )
          .*?
          (?: \# \s* install: \s* ([\w\-.]+) )?
          \s* $
        "#
    ).expect("Invalid regex pattern");

    let mut libraries = Vec::new();
    for cap in import_regex.captures_iter(&content) {
        // Se tem comentário # install:, usa esse nome diretamente
        if let Some(package_from_comment) = cap.get(3) {
            libraries.push(package_from_comment.as_str().to_string());
            continue;
        }
        
        // Pega o módulo importado
        if let Some(m) = cap.get(1).or_else(|| cap.get(2)) {
            let full_module = m.as_str();
            
            // IGNORA imports relativos (começam com .)
            if full_module.starts_with('.') {
                continue;
            }
            
            // Pega apenas o primeiro componente do módulo
            // Ex: "numpy.random" -> "numpy", "src.utils" -> "src"
            let module_name = full_module.split('.').next().unwrap();
            
            // IGNORA se o módulo corresponde a um arquivo ou diretório local
            if is_local_module(module_name, file_dir, project_root) {
                continue;
            }
            
            // Usa o mapeamento se existir, senão usa o nome do módulo
            let package = mapping.get(module_name)
                .map(|s| s.as_str())
                .unwrap_or(module_name);
                
            libraries.push(package.to_string());
        }
    }

    // Remove bibliotecas padrão do Python
    let standard_libs = get_standard_libraries();
    libraries.retain(|lib| !standard_libs.contains(lib));
    
    libraries
}

fn get_standard_libraries() -> Vec<String> {
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

fn find_requirements_txt(dir: &Path) -> Option<PathBuf> {
    // Procura requirements.txt no diretório e subdiretórios
    fn search_recursive(dir: &Path, depth: usize) -> Option<PathBuf> {
        // Limita a profundidade para evitar busca infinita
        if depth > 3 {
            return None;
        }
        
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_file() && path.file_name() == Some(std::ffi::OsStr::new("requirements.txt")) {
                    return Some(path);
                }
                
                // Ignora diretórios especiais
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name() {
                        let dir_str = dir_name.to_string_lossy();
                        if dir_str != "venv" && dir_str != "__pycache__" 
                            && !dir_str.starts_with('.') && dir_str != "node_modules" {
                            if let Some(found) = search_recursive(&path, depth + 1) {
                                return Some(found);
                            }
                        }
                    }
                }
            }
        }
        None
    }
    
    search_recursive(dir, 0)
}

fn parse_requirements_txt(path: &Path) -> Vec<String> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read requirements.txt: {}", e);
            return Vec::new();
        }
    };

    let mut libraries = Vec::new();
    let package_regex = Regex::new(r"^([a-zA-Z0-9\-_.]+)").expect("Invalid regex");

    for line in content.lines() {
        let line = line.trim();
        
        // Ignora linhas vazias, comentários e flags do pip
        if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
            continue;
        }

        // Extrai o nome do pacote (antes de ==, >=, etc.)
        if let Some(cap) = package_regex.captures(line) {
            if let Some(package) = cap.get(1) {
                libraries.push(package.as_str().to_string());
            }
        }
    }

    libraries
}

fn find_all_python_files(dir: &Path) -> Vec<PathBuf> {
    let mut python_files = Vec::new();
    
    fn search_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_file() && path.extension() == Some(std::ffi::OsStr::new("py")) {
                    files.push(path);
                } else if path.is_dir() {
                    // Ignora diretórios especiais
                    if let Some(dir_name) = path.file_name() {
                        let dir_str = dir_name.to_string_lossy();
                        if dir_str != "venv" && dir_str != "__pycache__" 
                            && !dir_str.starts_with('.') && dir_str != "node_modules" {
                            search_recursive(&path, files);
                        }
                    }
                }
            }
        }
    }
    
    search_recursive(dir, &mut python_files);
    python_files
}

fn extract_libraries_from_directory(dir: &Path, mapping: &HashMap<String, String>) -> Vec<String> {
    println!("Searching for Python files in directory: {}", dir.display());
    
    // Primeiro procura requirements.txt
    if let Some(requirements_path) = find_requirements_txt(dir) {
        println!("Found requirements.txt: {}", requirements_path.display());
        let libraries = parse_requirements_txt(&requirements_path);
        println!("Extracted {} libraries from requirements.txt", libraries.len());
        return libraries;
    }
    
    println!("No requirements.txt found, analyzing Python files...");
    
    // Se não encontrou requirements.txt, procura todos os arquivos .py
    let python_files = find_all_python_files(dir);
    println!("Found {} Python files", python_files.len());
    
    let mut all_libraries = Vec::new();
    
    for file in python_files {
        if let Some(file_str) = file.to_str() {
            // Passa o diretório raiz (dir) para verificar módulos locais
            let libs = extract_imported_libraries(file_str, mapping, dir);
            all_libraries.extend(libs);
        }
    }
    
    // Remove duplicatas e ordena
    all_libraries.sort();
    all_libraries.dedup();
    
    println!("Extracted {} unique libraries from Python files", all_libraries.len());
    all_libraries
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
    
    if args.len() > 3 {
        eprintln!("Usage: {} [python-file-or-directory] [venv-name]", args[0]);
        eprintln!("  python-file-or-directory: Path to Python file or directory (default: current directory)");
        eprintln!("  venv-name: Name of virtual environment (default: venv)");
        exit(1);
    }

    // Se não passar argumento, usa o diretório atual
    let target_path = if args.len() >= 2 {
        PathBuf::from(&args[1])
    } else {
        env::current_dir().expect("Failed to get current directory")
    };

    let venv_name = args.get(2).map_or("venv", |s| s.as_str());

    let mapping_path = find_mapping_file();
    
    let mapping = load_mapping(&mapping_path)
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load mapping - {}", e);
            HashMap::new()
        });

    // Verifica se o caminho existe
    if !target_path.exists() {
        eprintln!("Path not found: {}", target_path.display());
        exit(1);
    }

    // Determina o diretório raiz do projeto
    let project_root = if target_path.is_file() {
        target_path.parent().unwrap_or_else(|| Path::new("."))
    } else {
        target_path.as_path()
    };

    // Determina as bibliotecas necessárias
    let libraries = if target_path.is_file() {
        println!("Processing Python file: {}", target_path.display());
        extract_imported_libraries(
            target_path.to_str().expect("Invalid file path"), 
            &mapping,
            project_root
        )
    } else if target_path.is_dir() {
        extract_libraries_from_directory(&target_path, &mapping)
    } else {
        eprintln!("Invalid path type: {}", target_path.display());
        exit(1);
    };

    // Cria venv se não existir
    if !venv_exists(venv_name) {
        create_venv(venv_name);
    } else {
        println!("Virtual environment '{}' already exists", venv_name);
    }

    install_libraries(venv_name, &libraries);

    println!("\n✓ Success! Virtual environment is ready.");
    println!("\nTo activate the environment:");
    if cfg!(windows) {
        println!("    {}\\Scripts\\activate", venv_name);
    } else {
        println!("    source {}/bin/activate", venv_name);
    }
    
    if target_path.is_file() {
        println!("\nTo run your script:");
        println!("    python {}", target_path.display());
    }
}