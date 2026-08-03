# AutoClicker Humanizado GUI 🦀

![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)

Nota: Esta aplicação tem como intuito o aprendizado para criar aplicações nativas para Linux mantendo a estética do GNOME e a fidelidade de estilização.

Um AutoClicker moderno e humanizado desenvolvido em Rust com GTK4, Libadwaita e Relm4, projetado especificamente para o ambiente de trabalho GNOME e ecossistema Linux.

---

## Screenshots

| Perfis Configurados | Assistente de Criação de Macro |
| :---: | :---: |
| ![Perfis Configurados](assets/preview_main.png) | ![Assistente de Criação](assets/preview_wizard.png) |

---

## Estrutura do Código e Arquitetura

A aplicação segue uma separação clara entre a camada de apresentação (Interface) e a regra de negócio (Engine de cliques e eventos):

```
                                +---------------------------+
                                |      Ponto de Entrada     |
                                |       (src/main.rs)       |
                                +-------------+-------------+
                                              |
                                              v
                                +---------------------------+
                                |    Camada de Interface    |
                                |       (src/ui/mod.rs)     |
                                +------+-------------+------+
                                       |             |
                     +-----------------+             +-----------------+
                     |                                                 |
                     v                                                 v
       +---------------------------+                     +---------------------------+
       |   Camada de Configuração  |                     |  Engine (Regra de Negócio)|
       |   (src/config.rs &        |                     |      (src/engine/mod.rs)  |
       |    src/profiles/mod.rs)   |                     +--------------+------------+
       +---------------------------+                                    |
                                                                        v
                                                         +---------------------------+
                                                         | Dispositivos de Entrada   |
                                                         |       (/dev/input/*)      |
                                                         +---------------------------+
```

### Detalhamento dos Componentes

#### 1. Camada de Interface (Apresentação)
- `src/ui/mod.rs`: Implementa a interface gráfica reativa em GTK4 e Libadwaita utilizando o framework Relm4. Gerencia janelas, componentes visuais, assistente interativo de configuração de macros e atualização de estados visuais.

#### 2. Regra de Negócio (Engine)
- `src/engine/mod.rs`: Contém toda a lógica central de funcionamento do autoclicker:
  - Captura nativa de eventos de teclado e mouse usando a biblioteca `evdev`.
  - Algoritmo de simulação humanizada com distribuição estatística de tempo entre cliques (evitando padrões robóticos).
  - Controle assíncrono e gerenciamento de threads de execução (Hold/Toggle).

#### 3. Gestão de Dados e Configuração
- `src/config.rs`: Define as estruturas de dados para modos de clique (Humanizado, Fixo, Duplo Clique) e tipos de gatilho.
- `src/profiles/mod.rs`: Gerencia a leitura, escrita e persistência dos perfis de autoclicker no formato JSON.

---

## Funcionalidades

- Interface Nativa GNOME: Construída com Libadwaita garantindo integração visual com o sistema operacional.
- Modo Humanizado: Variações estatísticas de tempo entre cliques para simular comportamento humano real.
- Assistente Interativo: Passo a passo para captura automática de botões de gatilho e ação.
- Modos de Disparo: Suporte a segurar (Hold) e alternar (Toggle).
- Gerenciamento de Perfis: Salve e alterne facilmente entre múltiplos perfis.

---

## 🛠️ Tutorial de Instalação do Rust

Se você ainda não possui o Rust instalado em seu sistema Linux, siga o passo a passo abaixo utilizando o utilitário oficial `rustup`:

### Passo 1: Executar o instalador oficial do Rust
Abra o seu terminal e rode o seguinte comando:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Passo 2: Confirmar a instalação
Durante a execução do script, o terminal exibirá opções de instalação. Pressione a tecla **Enter** para aceitar a instalação padrão (*Option 1: Proceed with installation (default)*).

### Passo 3: Carregar as variáveis de ambiente
Após a conclusão da instalação, carregue o ambiente do Cargo no seu terminal atual:
```bash
source "$HOME/.cargo/env"
```

### Passo 4: Confirmar a instalação
Para verificar se o compilador do Rust e o gerenciador de pacotes Cargo foram instalados corretamente, execute:
```bash
rustc --version
cargo --version
```
Se ambos exibirem os números de versão (ex: `rustc 1.80.0 ...`), o Rust está pronto para uso!

---

## 📋 Dependências do Sistema

Além do Rust, você precisará das bibliotecas de desenvolvimento gráfico do GTK4/Libadwaita e permissões de entrada:

### 1. Bibliotecas de Sistema (GTK4 e Libadwaita)

Ubuntu / Debian:
```bash
sudo apt update
sudo apt install build-essential libgtk-4-dev libadwaita-1-dev
```

Fedora:
```bash
sudo dnf install gtk4-devel libadwaita-devel
```

Arch Linux:
```bash
sudo pacman -S gtk4 libadwaita
```

### 2. Permissões de Dispositivo de Entrada (evdev)
O programa lê e envia eventos nativos através de `/dev/input/*`. Adicione seu usuário ao grupo `input` para permitir acesso sem privilégios de root:

```bash
sudo usermod -aG input $USER
```
Nota: É necessário encerrar a sessão (logoff) ou reiniciar o sistema para que a alteração de grupo surta efeito.

---

## 🚀 Como Executar

1. Clone o repositório:
```bash
git clone git@github.com:DouradoCtrl/autoclicker-gui-rust.git
cd autoclicker-gui-rust
```

2. Execute o projeto:
```bash
cargo run --release
```

---

## Licença

Este projeto é disponibilizado sob a licença MIT.
