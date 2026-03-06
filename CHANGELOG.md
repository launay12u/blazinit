# Changelog

All notable changes to this project will be documented in this file.

## [0.0.1] - 2026-03-06

### 🚀 Features

- *(core)* Add profile data model and file management
- *(cli)* Implement CLI argument parsing
- *(core)* Wire up CLI commands to profile operations
- *(registry)* Add bundled registry with version-based auto-update
- *(config)* Add configurable default profile system
- Implement registry operations and wire up CLI
- Validate package presence in remove command
- *(package/add)* Error when adding duplicate package
- *(profile)* Add --default flag to create command
- Implement install, export/import, registry subcommands
- Complete blazinit feature set with install, export/import, and registry
- *(registry)* Auto-update registry silently on CLI startup
- *(logging)* Add structured logging with log crate
- Add self-update command and GitHub release workflow
- Manual release workflow, install scripts, updated README

### 🐛 Bug Fixes

- Switch ureq to native-tls for cross-compile
- Switch ureq to tls for cross-compile

### 🚜 Refactor

- *(deps)* Switch to dirs-next and add include_dir
- *(config)* Centralize configuration and path management
- *(main)* Extract business logic to lib module
- Change terminology from software to package
- *(profile)* Introduce rich package metadata and dynamic defaults
- *(registry)* Migrate from single file to directory-based structure
- *(profile)* Store package refs instead of full package details
- Simplify registry and profile code
- *(registry)* Remove manual update CLI command
- *(registry)* Always check remote on startup, remove 24h staleness gate
- *(logging)* Auto-log command via Debug derive instead of manual per-arm logs

### 📚 Documentation

- Add project documentation

### ⚡ Performance

- *(startup)* Run registry update in background thread, fix TOCTOU in import

### 🧪 Testing

- *(cli)* Add error handling verification tests

