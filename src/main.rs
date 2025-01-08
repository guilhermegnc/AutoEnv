use regex::Regex;
use std::env;
use std::fs;
use std::process::{Command, exit};
use std::path::Path;

fn extract_imported_libraries(file_path: &str) -> Vec<String> {
    let content = fs::read_to_string(file_path).expect("Erro ao ler o arquivo.");
    
    // Regex que captura importações tanto do tipo "import" quanto "from ... import"
    let import_regex = Regex::new(r"(?m)^\s*(?:import|from)\s+([a-zA-Z_][a-zA-Z0-9_\.]*)")
        .expect("Erro ao compilar regex.");
    
    let mut libraries: Vec<String> = import_regex
        .captures_iter(&content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();
    
    // Remover submódulos para manter apenas a biblioteca principal
    libraries.iter_mut().for_each(|lib| {
        if let Some(pos) = lib.find('.') {
            *lib = lib[..pos].to_string(); // Apenas a parte principal da biblioteca
        }
    });

    // Lista de bibliotecas padrão do Python
    let standard_libraries = vec![
    "abc", "aifc", "antigravity", "argparse", "array", "asyncio", "base64", "binascii", "bisect", 
    "builtins", "bz2", "calendar", "cmath", "collections", "collections.abc", "contextlib", "copy", 
    "csv", "datetime", "decimal", "difflib", "dis", "doctest", "email", "encodings", "enum", "filecmp", 
    "fileinput", "fnmatch", "fractions", "ftplib", "functools", "gc", "gettext", "glob", "gzip", "hashlib", 
    "heapq", "hmac", "http", "imaplib", "importlib", "inspect", "io", "ipaddress", "itertools", "json", 
    "logging", "math", "mmap", "multiprocessing", "netrc", "nis", "nntplib", "numbers", "operator", 
    "optparse", "os", "pathlib", "pdb", "pickle", "pipes", "pkgutil", "platform", "plistlib", "poplib", 
    "pdb", "pydoc", "queue", "random", "re", "reprlib", "resource", "sched", "secrets", "select", "shutil", 
    "signal", "site", "smtplib", "socket", "sqlite3", "ssl", "string", "stringprep", "struct", "subprocess", 
    "sys", "sysconfig", "tabnanny", "tarfile", "telnetlib", "tempfile", "threading", "time", "timeit", "token", 
    "tokenize", "trace", "traceback", "tracemalloc", "types", "typing", "unittest", "urllib", "uuid", "venv", 
    "warnings", "weakref", "xdrlib", "xml", "zipfile", "zlib"
];

    
    // Filtra bibliotecas externas (excluindo as bibliotecas padrão)
    libraries.retain(|lib| !standard_libraries.contains(&lib.as_str()));
    
    // Ordena e remove duplicatas
    libraries.sort();
    libraries.dedup();
    libraries
}

fn venv_exists(venv_name: &str) -> bool {
    let venv_path = format!(".\\{}", venv_name);
    fs::metadata(venv_path).is_ok()
}

fn is_inside_venv(python_file: &str) -> bool {
    let python_path = Path::new(python_file);
    let python_file_abs_path = python_path.canonicalize().expect("Erro ao obter o caminho absoluto.");
    let mut current_dir = python_file_abs_path.parent().unwrap_or_else(|| Path::new(""));

    // Subindo a partir do diretório onde o arquivo Python está para encontrar a venv
    while let Some(parent) = current_dir.parent() {
        if parent.join("venv").exists() {
            return true;
        }
        current_dir = parent;
    }

    false
}

fn create_venv(venv_name: &str) {
    println!("Criando ambiente virtual '{}'...", venv_name);
    let status = Command::new("python")
        .args(["-m", "venv", venv_name])
        .status()
        .expect("Erro ao criar o ambiente virtual.");
    if !status.success() {
        eprintln!("Falha ao criar o ambiente virtual '{}'.", venv_name);
        exit(1);
    }
    println!("Ambiente virtual '{}' criado com sucesso!", venv_name);
}

fn install_libraries(venv_name: &str, python_file: &str, libraries: &[String]) {
    if libraries.is_empty() {
        println!("Nenhuma biblioteca para instalar.");
        return;
    }
    println!("Instalando bibliotecas no ambiente virtual '{}': {:?}", venv_name, libraries);
    let mut activate_script = String::new();
    // Caminho para ativar o ambiente virtual no Windows
    if !is_inside_venv(python_file) {
        activate_script = format!(".\\{}\\Scripts\\activate", venv_name);
    }
    else {
        activate_script = format!("activate");
    }

    // Usando cmd para ativar o venv e rodar o pip install
    let status = Command::new("cmd")
        .args(["/C", &format!("{} && pip install {}", activate_script, libraries.join(" "))])
        .status()
        .expect("Erro ao instalar bibliotecas no ambiente virtual.");
    if !status.success() {
        eprintln!("Falha ao instalar bibliotecas no ambiente virtual '{}'.", venv_name);
        exit(1);
    }
    println!("Bibliotecas instaladas com sucesso!");
}

fn move_program_to_venv(venv_name: &str, python_file: &str) {
    let venv_script_path = format!(".\\{}/Scripts", venv_name);
    let destination = Path::new(&venv_script_path).join(python_file);
    fs::copy(python_file, destination).expect("Erro ao mover o arquivo para o venv.");
    println!("O arquivo '{}' foi movido para o ambiente virtual.", python_file);
    // Deletar o arquivo original
    fs::remove_file(python_file).expect("Erro ao deletar o arquivo original.");
    println!("O arquivo '{}' foi deletado do local original.", python_file);
}

fn dependencies_process(venv_name: &str, python_file: &str) {
    // Extrair as bibliotecas importadas do arquivo Python
    let libraries = extract_imported_libraries(python_file);
    println!("Bibliotecas detectadas: {:?}", libraries);

    // Instalar as dependências no ambiente virtual
    install_libraries(venv_name, python_file, &libraries);

    println!("Processo concluído com sucesso! O arquivo Python está no ambiente virtual e as dependências foram instaladas.");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Uso: programa_rust <arquivo.py>");
        exit(1);
    }
    let python_file = &args[1];

    // Verificar se o arquivo Python existe
    if !fs::metadata(python_file).is_ok() {
        eprintln!("Arquivo não encontrado: {}", python_file);
        exit(1);
    }

    let venv_name = "venv";
    
    // Verificar se o arquivo Python já está dentro de uma venv (na pasta Scripts)
    if !is_inside_venv(python_file) {
        // Se não estiver, verificamos se a venv existe
        if !venv_exists(venv_name) {
            println!("Ambiente virtual '{}' não encontrado.", venv_name);
            create_venv(venv_name);
        } else {
            println!("Ambiente virtual '{}' encontrado.", venv_name);
        }

        dependencies_process(venv_name, python_file);

        // Mover o programa Python para o ambiente virtual
        move_program_to_venv(venv_name, python_file);
    } else {
        println!("O arquivo '{}' já está dentro do ambiente virtual.", python_file);

        dependencies_process(venv_name, python_file);
    }
}
