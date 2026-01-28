# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Blazinit is a CLI tool written in Rust for managing software installation profiles. It allows users to create profiles containing lists of software identifiers, then install all software in a profile with a single command.

**Language**: Rust (Edition 2024)
**Key Dependencies**: clap (CLI parsing), serde/toml (serialization), colored (terminal output), dirs (cross-platform paths)

## Project Structure

```
src/
├── main.rs      - Entry point, command dispatch
├── cli.rs       - CLI argument parsing with clap
└── profile.rs   - Profile management (CRUD operations, file I/O)
```

**Profile Storage**: Profiles are stored as TOML files in `~/.config/blazinit/profiles/` (platform-specific config directory via `dirs` crate).

**Architecture**: Simple command-based structure where `main.rs` dispatches to profile operations based on parsed CLI commands.

## Development Commands

All commands use `just` as the task runner. Use `just <recipe>` to execute.

### Quality Checks (CI-equivalent)
```bash
just quality-check
```
Runs the full quality check suite:
- `cargo fmt -- --check` - Check code formatting
- `cargo clippy -- -D warnings` - Lint with all warnings as errors
- `cargo check --all` - Type checking
- `cargo machete` - Find unused dependencies
- `cargo audit` - Security audit
- `cargo test` - Run test suite

### Auto-fix Quality Issues
```bash
just quality-format
```
Automatically fixes formatting and some linting issues:
- `cargo fmt` - Auto-format code
- `cargo clippy --fix` - Auto-fix clippy warnings
- `cargo audit fix` - Fix security issues

### Build & Run
```bash
cargo build              # Debug build
cargo run -- <args>      # Run with arguments
cargo build --release    # Release build
```

### Testing
```bash
cargo test                    # Run all tests
cargo test <test_name>        # Run specific test
```

### Changelog Management
```bash
just generate-changelog    # Generate changelog with git-cliff
just bump-version         # Bump version and update changelog
```

## Code Style

**Formatting** is defined in `rustfmt.toml`:
- Max line width: 80 characters
- Comment width: 120 characters
- Edition: 2024
- Import grouping: `StdExternalCrate` (stdlib → external → local)
- `reorder_imports = true`, `merge_derives = true`

**Linting**: `cargo clippy` enforces all warnings as errors in CI.

## Configuration

`config.toml` defines the remote registry URL:
```toml
registry_url = "https://raw.githubusercontent.com/launay12u/blazinit/main/registry/software.toml"
```

## Changelog

Uses `git-cliff` with conventional commits:
- Follows [Conventional Commits](https://www.conventionalcommits.org)
- Commit types: `feat`, `fix`, `doc`, `perf`, `refactor`, `style`, `test`, `chore`
- Generates grouped changelog with emojis
- Skip patterns: `chore(release)`, `chore(deps)`, `chore(pr)`

## Key Implementation Details

**Profile Model**: `Profile` struct in `profile.rs` with `name: String` and `software: Vec<String>`.

**Default Profile**: A "default" profile is automatically created on first run via `ensure_default_profile()`.

**Error Handling**: Functions return `Result<(), String>` with user-facing error messages.

**CLI Design**: All commands default to the "default" profile unless specified otherwise.

## Important Constraints

- The default profile cannot be deleted (enforced in `delete_profile()`).
- Profile names are used as TOML filenames (`{name}.toml`).
- Commands `add`, `remove`, `show`, `export`, and `install` accept an optional profile argument (defaults to "default").
