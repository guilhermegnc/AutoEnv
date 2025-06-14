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
    // 1. Primeiro tenta encontrar no mesmo diretório do executável
    let exe_dir = env::current_exe()
        .expect("Falha ao obter caminho do executável")
        .parent()
        .expect("Falha ao obter diretório do executável")
        .to_path_buf();
    
    let mapping_in_exe_dir = exe_dir.join("mapeamento.toml");
    if mapping_in_exe_dir.exists() {
        return mapping_in_exe_dir;
    }

    // 2. Se não encontrou, tenta subir 3 níveis para chegar na raiz do projeto
    // (target/release/ -> target/ -> projeto/)
    let project_dir = exe_dir
        .parent().and_then(|p| p.parent()).and_then(|p| p.parent());
    
    if let Some(project_dir) = project_dir {
        let mapping_in_project = project_dir.join("mapeamento.toml");
        if mapping_in_project.exists() {
            return mapping_in_project;
        }
    }

    // 3. Se não encontrou em nenhum lugar, retorna o caminho padrão (relativo ao executável)
    exe_dir.join("mapeamento.toml")
}

fn extract_imported_libraries(file_path: &str, mapping: &HashMap<String, String>) -> Vec<String> {
    let content = fs::read_to_string(file_path).expect("Failed to read file");

    let import_regex = Regex::new(
        r#"(?imx)
          ^ \s*
          (?: from \s+ ([\w\.]+) \s+ import | import \s+ ([\w\.]+) ) # Captura o nome do módulo
          .*?                                                        # Corresponde a qualquer caractere
          (?: \# \s* install: \s* ([\w\-.]+) )?                      # Captura opcional do nome do pacote
          \s* $                                                      # Até o final da linha
        "#
    ).expect("Invalid regex pattern");

    let mut libraries = Vec::new();
    for cap in import_regex.captures_iter(&content) {
        if let Some(package_from_comment) = cap.get(3) {
            libraries.push(package_from_comment.as_str().to_string());
        } else if let Some(m) = cap.get(1).or_else(|| cap.get(2)) {
            let module_name = m.as_str().split('.').next().unwrap();
            
            // Aplica o mapeamento se existir, senão usa o nome do módulo
            let package = mapping.get(module_name)
                .map(|s| s.as_str())
                .unwrap_or(module_name);
                
            libraries.push(package.to_string());
        }
    }

    let standard_libs = get_standard_libraries();
    libraries.retain(|lib| !standard_libs.contains(lib) && lib != "random");
    
    libraries.sort();
    libraries.dedup();
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

    let mapping_path = find_mapping_file();
    
    let mapping = load_mapping(&mapping_path)
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load mapping - {}", e);
            HashMap::new()
        });

    // Verifica se o arquivo Python existe (usando caminho absoluto se necessário)
    if !Path::new(python_file).exists() {
        eprintln!("Python file not found: {}", python_file);
        exit(1);
    }

    if !venv_exists(venv_name) {
        create_venv(venv_name);
    }

    let libraries = extract_imported_libraries(python_file, &mapping);
    install_libraries(venv_name, &libraries);

    println!("\nSuccess! Now run your script using:");
    if cfg!(windows) {
        println!("    {}\\Scripts\\activate && python {}", venv_name, python_file);
    } else {
        println!("    source {}/bin/activate && python {}", venv_name, python_file);
    }
}