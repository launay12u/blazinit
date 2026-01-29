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
        .stderr(predicate::str::contains("is already present in profile"));
}

#[test]
fn test_create_profile_with_default_flag() {
    let temp_dir = setup_test_env();

    // Create a new profile with --default flag
    blazinit_cmd(&temp_dir)
        .arg("create")
        .arg("my-default")
        .arg("--default")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Successfully created profile 'my-default'",
        ))
        .stdout(predicate::str::contains(
            "Default profile set to 'my-default'",
        ));

    // Verify it's now the default
    blazinit_cmd(&temp_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("my-default (default)"));
}

#[test]
fn test_set_default_on_non_existent_profile() {
    let temp_dir = setup_test_env();

    blazinit_cmd(&temp_dir)
        .arg("set-default")
        .arg("non-existent")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Profile 'non-existent' does not exist",
        ));
}

#[test]
fn test_create_duplicate_profile_fails() {
    let temp_dir = setup_test_env();

    // Create profile
    blazinit_cmd(&temp_dir)
        .arg("create")
        .arg("duplicate")
        .assert()
        .success();

    // Try to create again
    blazinit_cmd(&temp_dir)
        .arg("create")
        .arg("duplicate")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Profile 'duplicate' already exists",
        ));
}

#[test]
fn test_delete_non_existent_profile_fails() {
    let temp_dir = setup_test_env();

    blazinit_cmd(&temp_dir)
        .arg("delete")
        .arg("non-existent")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Profile 'non-existent' does not exist",
        ));
}

#[test]
fn test_show_non_existent_profile_fails() {
    let temp_dir = setup_test_env();

    blazinit_cmd(&temp_dir)
        .arg("show")
        .arg("non-existent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_remove_from_non_existent_profile_fails() {
    let temp_dir = setup_test_env();

    blazinit_cmd(&temp_dir)
        .arg("remove")
        .arg("some-package")
        .arg("non-existent")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Profile 'non-existent' does not exist",
        ));
}

#[test]
fn test_add_to_non_existent_profile_fails() {
    let temp_dir = setup_test_env();

    blazinit_cmd(&temp_dir)
        .arg("add")
        .arg("some-package")
        .arg("non-existent")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Profile 'non-existent' does not exist",
        ));
}
