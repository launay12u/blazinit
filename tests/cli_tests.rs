use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

fn blazinit_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("blazinit");
    cmd.env("HOME", temp_dir.path())
        .env("XDG_CONFIG_HOME", temp_dir.path().join(".config"));
    cmd
}

#[test]
fn test_help_command() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("blazinit");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Blazinit allows you to create"));
}

#[test]
fn test_create_and_list_profile() {
    let temp_dir = setup_test_env();

    blazinit_cmd(&temp_dir)
        .arg("create")
        .arg("integration-test")
        .assert()
        .success();

    blazinit_cmd(&temp_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("integration-test"));
}

#[test]
fn test_delete_profile() {
    let temp_dir = setup_test_env();

    blazinit_cmd(&temp_dir)
        .arg("create")
        .arg("to-delete")
        .assert()
        .success();

    blazinit_cmd(&temp_dir)
        .arg("delete")
        .arg("to-delete")
        .assert()
        .success();

    blazinit_cmd(&temp_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("to-delete").not());
}

#[test]
fn test_default_profile_exists_implicitly() {
    let temp_dir = setup_test_env();

    blazinit_cmd(&temp_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("default (default)"));
}

#[test]
fn test_cannot_delete_default_profile() {
    let temp_dir = setup_test_env();

    blazinit_cmd(&temp_dir).arg("list").assert().success();

    // Try delete
    blazinit_cmd(&temp_dir)
        .arg("delete")
        .arg("default")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Cannot delete the default profile",
        ));
}

#[test]
fn test_cannot_add_duplicate_package() {
    let temp_dir = setup_test_env();

    // Create a test profile
    blazinit_cmd(&temp_dir)
        .arg("create")
        .arg("test-profile")
        .assert()
        .success();

    // Manually write a profile with a package (since add requires registry)
    let config_path = temp_dir.path().join(".config/blazinit/profiles");
    std::fs::create_dir_all(&config_path).unwrap();
    let profile_file = config_path.join("test-profile.toml");
    std::fs::write(
        &profile_file,
        r#"name = "test-profile"

[[packages]]
name = "git"
"#,
    )
    .unwrap();

    // Try to add the same package again
    blazinit_cmd(&temp_dir)
        .arg("add")
        .arg("git")
        .arg("test-profile")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "is already present in profile",
        ));
}
