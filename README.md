# AutoEnv

O **AutoEnv** é uma ferramenta escrita em Rust que automatiza a criação e o gerenciamento de ambientes virtuais Python. Ele detecta as bibliotecas usadas no código, cria um ambiente virtual, instala as dependências e move o arquivo Python para dentro do ambiente virtual.

## Funcionalidades

- Criação automática de ambientes virtuais Python.
- Instalação das bibliotecas necessárias a partir do código Python.
- Mover o script Python para o diretório do ambiente virtual.

## Como usar

1. Clone o repositório:
   ```bash
   git clone https://github.com/guilhermegnc/AutoEnv.git
2. Compile o projeto:
    ```bash
    cargo build --release
3. Execute o programa com o arquivo Python como argumento:
    ```bash
    ./target/release/AutoEnv arquivo.py
    
## Contribuição

1. Faça um fork do projeto.
2. Crie uma branch para sua feature (git checkout -b feature/nome-da-feature).
3. Faça commit das suas mudanças (git commit -am 'Adiciona nova feature').
4. Faça push para sua branch (git push origin feature/nome-da-feature).
5. Crie um pull request.
   
## Licença

Este projeto é licenciado sob a Licença MIT.
