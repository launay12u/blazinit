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


generate-changelog:
    git cliff

bump-version:
    git cliff --bump