# Blazinit

[![CI](https://github.com/launay12u/blazinit/actions/workflows/ci.yml/badge.svg)](https://github.com/launay12u/blazinit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.1.2-blue.svg)](https://github.com/launay12u/blazinit/releases)

Blazing fast CLI tool written in Rust for managing reproducible software installation profiles. Create profiles once, and install all your essential tools on any machine with a single command.

## 🚀 Features

- **Profile Management**: Create, delete, and list software profiles.
- **Dependency Tracking**: Add or remove software identifiers from specific profiles.
- **Single-Command Install**: Install all software defined in a profile at once.
- **Portability**: Export and import profiles as TOML files for easy sharing.
- **Cross-Platform**: Works seamlessly on Linux, macOS, and Windows.

## 📦 Installation

### From Source

Ensure you have [Rust](https://www.rust-lang.org/) installed, then:

```bash
git clone https://github.com/launay12u/blazinit.git
cd blazinit
cargo install --path .
```

## 🛠️ Usage

### Quick Start

1.  **Add software** to your default profile:
    ```bash
    blazinit add rustup
    blazinit add neovim
    ```

2.  **List** your current profile:
    ```bash
    blazinit show
    ```

3.  **Install** everything:
    ```bash
    blazinit install
    ```

### Managing Profiles

- **Create a new profile**:
  ```bash
  blazinit create work
  ```

- **Add software to a specific profile**:
  ```bash
  blazinit add docker work
  ```

- **List all profiles**:
  ```bash
  blazinit list
  ```

- **Export/Import**:
  ```bash
  blazinit export work work-profile.toml
  blazinit import work-profile.toml
  ```

## ⚙️ Configuration

Blazinit stores its configuration and profiles in your platform's standard config directory:

- **Linux**: `~/.config/blazinit/`
- **macOS**: `~/Library/Application Support/blazinit/`
- **Windows**: `C:\Users\User\AppData\Roaming\blazinit\config\`

Profiles are stored as TOML files in the `profiles/` subdirectory.

## 🤝 Contributing

We welcome contributions! Please ensure your code passes the quality checks:

```bash
# Run all quality checks
just quality-check

# Run tests
just test

# Automatically format code
just quality-format
```

## 📄 License

This project is licensed under the MIT License - see the [LICENCE.md](LICENCE.md) file for details.
