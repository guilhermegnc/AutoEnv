# AutoEnv

**AutoEnv** is a tool written in Rust that automates the creation and management of Python virtual environments. It detects the libraries used in the code, creates a virtual environment, installs the dependencies, and moves the Python file into the virtual environment.

## Features

- **Automatic Virtual Environment Creation**: Creates a Python virtual environment in your project directory.
- **Dependency Detection**: Automatically detects imported libraries in your Python files (`.py`).
- **`requirements.txt` Support**: If a `requirements.txt` file is found, AutoEnv will install the packages listed there.
- **Custom Module Mapping**: A `mapeamento.toml` file can be used to map import names to package names (e.g., `bs4` to `beautifulsoup4`).
- **Local Module Exclusion**: Automatically excludes local modules from the dependency installation.

## How to use

### 1. Installation

Clone the repository and compile the project:

```bash
git clone https://github.com/guilhermegnc/AutoEnv.git
cd AutoEnv
cargo build --release
```

The executable will be available at `./target/release/AutoEnv`.

### 2. Usage

You can run AutoEnv with a single Python file, a directory, or no arguments (which defaults to the current directory).

**Analyze a specific Python file:**

```bash
./target/release/AutoEnv your_script.py
```

**Analyze an entire project directory:**

```bash
./target/release/AutoEnv /path/to/your/project
```

**Use a custom name for the virtual environment:**

```bash
./target/release/AutoEnv your_script.py my_venv
```

If no arguments are provided, AutoEnv will analyze the current directory and create a `venv` virtual environment.

## Configuration

AutoEnv can be configured using a `mapeamento.toml` file to handle cases where the import name is different from the package name. For example, to install `python-dotenv` for the `dotenv` import, you can create a `mapeamento.toml` file in the same directory as the executable with the following content:

```toml
[mapping]
dotenv = "python-dotenv"
BeautifulSoup = "beautifulsoup4"
```
    
## Contributing

Contributions are welcome! If you have a suggestion or find a bug, please open an issue to discuss it.

To contribute code:

1. Fork the repository.
2. Create a new branch for your feature (`git checkout -b feature/your-feature`).
3. Make your changes and commit them (`git commit -m 'Add your feature'`).
4. Push to your branch (`git push origin feature/your-feature`).
5. Open a pull request.

## License

This project is licensed under the [MIT License](LICENSE).
