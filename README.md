# Blazinit

[![CI](https://github.com/launay12u/blazinit/actions/workflows/ci.yml/badge.svg)](https://github.com/launay12u/blazinit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Latest Release](https://img.shields.io/github/v/release/launay12u/blazinit)](https://github.com/launay12u/blazinit/releases/latest)

💥 Blazing fast CLI tool written in Rust for managing reproducible software installation profiles. Create profiles once, and install all your essential tools on any machine with a single command.

## Features

- **Profile Management** — Create, delete, and list software profiles
- **Dependency Tracking** — Add or remove packages from specific profiles
- **Single-Command Install** — Install everything in a profile at once
- **Portability** — Export and import profiles as TOML files
- **Cross-Platform** — Linux, macOS, and Windows
- **Auto-updating Registry** — Package registry stays up to date automatically

## Installation

### Linux & macOS

```sh
curl -fsSL https://github.com/launay12u/blazinit/releases/latest/download/blazinit-installer.sh | sh
```

### Windows (PowerShell)

```sh
irm https://github.com/launay12u/blazinit/releases/latest/download/blazinit-installer.ps1 | iex
```

### Manual download

Download the binary for your platform from the [latest release](https://github.com/launay12u/blazinit/releases/latest)

### From source

Requires [Rust](https://www.rust-lang.org/tools/install):

```sh
git clone https://github.com/launay12u/blazinit.git
cd blazinit
cargo install --path .
```

### Update

```sh
blazinit self-update
```

## Usage

### Quick start

```sh
# Add packages to your default profile
blazinit add git
blazinit add neovim
blazinit add docker

# Preview what would be installed
blazinit install --dry-run

# Install everything
blazinit install
```

### Profiles

```sh
# Create a named profile
blazinit create work

# Add packages to it
blazinit add docker work
blazinit add kubectl work

# Show its contents
blazinit show work

# Install it
blazinit install work

# Set it as default
blazinit set-default work

# List all profiles
blazinit list
```

### Export & import

```sh
# Export to a file (or stdout if no file given)
blazinit export work work.toml

# Import on another machine
blazinit import work.toml
```

### Registry

```sh
# Search available packages
blazinit registry list
blazinit registry list docker

# Add a custom package
blazinit registry add ./my-package.toml
```

## Configuration

Blazinit stores its data in your platform's standard config directory:

| OS | Path |
|---|---|
| Linux | `~/.config/blazinit/` |
| macOS | `~/Library/Application Support/blazinit/` |
| Windows | `%APPDATA%\blazinit\` |

Profiles are stored as TOML files under `profiles/`. The package registry is under `registry/` and updates automatically in the background on every run.

## Contributing

```sh
just quality-check   # fmt + clippy + type check
just test            # run test suite
just quality-format  # auto-fix formatting
just bump patch      # release a new version
```

## License

MIT — see [LICENCE.md](LICENCE.md)
