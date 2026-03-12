use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_cli_lifecycle() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // 1. 未初期化状態での add (エラーになるべき)
    let mut cmd = Command::cargo_bin("mytodo").unwrap();
    cmd.current_dir(root)
        .arg("add")
        .arg("Should Fail")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not initialized"));

    // 2. init 実行
    let mut cmd = Command::cargo_bin("mytodo").unwrap();
    cmd.current_dir(root)
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized .mytodo repository"));

    assert!(root.join(".mytodo/data.db").exists());

    // 3. task 追加
    let mut cmd = Command::cargo_bin("mytodo").unwrap();
    cmd.current_dir(root)
        .arg("add")
        .arg("Test CLI Task")
        .arg("-m")
        .arg("Description for CLI task")
        .assert()
        .success()
        .stdout(predicate::str::contains("Task created with ID: 1"));

    // 4. list 表示
    let mut cmd = Command::cargo_bin("mytodo").unwrap();
    cmd.current_dir(root)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Test CLI Task"))
        .stdout(predicate::str::contains("Open"));

    // 5. close 実行
    let mut cmd = Command::cargo_bin("mytodo").unwrap();
    cmd.current_dir(root)
        .arg("close")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Task 1 closed"));

    // 6. list (再度確認)
    let mut cmd = Command::cargo_bin("mytodo").unwrap();
    cmd.current_dir(root)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Close"));

    // 7. sync 実行 (tasks ディレクトリの確認)
    let mut cmd = Command::cargo_bin("mytodo").unwrap();
    cmd.current_dir(root).arg("sync").assert().success();

    assert!(root.join(".mytodo/tasks").is_dir());
    }


#[test]
fn test_tree_display() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Init
    Command::cargo_bin("mytodo")
        .unwrap()
        .current_dir(root)
        .arg("init")
        .assert()
        .success();

    // Add Parent
    Command::cargo_bin("mytodo")
        .unwrap()
        .current_dir(root)
        .arg("add")
        .arg("Parent")
        .assert()
        .success();

    // Add Child
    Command::cargo_bin("mytodo")
        .unwrap()
        .current_dir(root)
        .arg("add")
        .arg("Child")
        .arg("-p")
        .arg("1")
        .assert()
        .success();

    // List Tree
    let mut cmd = Command::cargo_bin("mytodo").unwrap();
    cmd.current_dir(root)
        .arg("list")
        .arg("--tree")
        .assert()
        .success()
        .stdout(predicate::str::contains("Parent (ID: 1)"))
        .stdout(predicate::str::contains("  [ ] Child (ID: 2)"));
}
