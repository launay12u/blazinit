quality-check:
    cargo fmt -- --check
    cargo clippy -- -D warnings
    cargo check --all
    cargo machete
    cargo audit
    cargo test

quality-format:
    cargo fmt
    cargo clippy --fix -Z unstable-options --allow-dirty
    cargo audit fix 


default_bump := "minor"

bump type=default_bump:
    #!/usr/bin/env bash
    set -euo pipefail
    
    # Get current version from latest git tag (strip 'v' prefix if present)
    latest_tag=$(git describe --tags --abbrev=0 2>/dev/null || echo "v0.1.0")
    current_version="${latest_tag#v}"
    
    # Split version into parts
    IFS='.' read -r major minor patch <<< "$current_version"
    
    # Calculate new version based on bump type
    case "{{type}}" in
        major) new_version="$((major + 1)).0.0" ;;
        minor) new_version="${major}.$((minor + 1)).0" ;;
        patch) new_version="${major}.${minor}.$((patch + 1))" ;;
        *) echo "Invalid: {{type}}"; exit 1 ;;
    esac
    
    echo "Releasing: v$current_version -> v$new_version"
    
    git switch -c release/v$new_version

    # Update Cargo.toml
    sed -i.bak "0,/^version = \".*\"/s//version = \"$new_version\"/" Cargo.toml
    rm -f Cargo.toml.bak
    cargo check --quiet 2>/dev/null || true
    
    # Initialize changelog if needed
    [[ -f CHANGELOG.md ]] || echo -e "# Changelog\n\nAll notable changes to this project will be documented in this file.\n" > CHANGELOG.md
    
    # Prepend unreleased changes
    git cliff --unreleased --tag "v$new_version" --prepend CHANGELOG.md
    
    # Commit and tag
    git add Cargo.toml Cargo.lock CHANGELOG.md
    git commit -m "chore(release): v$new_version"
    git tag "v$new_version"
    
    git push origin release/v$new_version
    git push --tags
    
    echo "✅ Released v$new_version"