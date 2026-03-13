use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_cli_lifecycle() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // 1. 未初期化状態での add (エラーになるべき)
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("add")
        .arg("Should Fail")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not initialized"));

    // 2. init 実行
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized .lissue repository"));

    assert!(root.join(".lissue/data.db").exists());

    // 3. task 追加
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("add")
        .arg("Test CLI Task")
        .arg("-m")
        .arg("Description for CLI task")
        .assert()
        .success()
        .stdout(predicate::str::contains("Task created with ID: 1"));

    // 4. list 表示
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Test CLI Task"))
        .stdout(predicate::str::contains("Open"));

    // 5. close 実行
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("close")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Task 1 closed"));

    // 6. list (再度確認)
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Close"));

    // 7. sync 実行 (tasks ディレクトリの確認)
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root).arg("sync").assert().success();

    assert!(root.join(".lissue/tasks").is_dir());
}

#[test]
fn test_tree_display() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Init
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("init")
        .assert()
        .success();

    // Add Parent
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("add")
        .arg("Parent")
        .assert()
        .success();

    // Add Child
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("add")
        .arg("Child")
        .arg("-p")
        .arg("1")
        .assert()
        .success();

    // List Tree
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("list")
        .arg("--tree")
        .assert()
        .success()
        .stdout(predicate::str::contains("Parent (ID: 1)"))
        .stdout(predicate::str::contains("  [ ] Child (ID: 2)"));
}

#[test]
fn test_claim_and_context() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("init")
        .assert()
        .success();

    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("add")
        .arg("Context Task")
        .arg("-m")
        .arg("Deep description")
        .assert()
        .success();

    // Claim
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("claim")
        .arg("1")
        .arg("--by")
        .arg("Tester")
        .assert()
        .success()
        .stdout(predicate::str::contains("Task 1 claimed by Tester"));

    // Context
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("context")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Title: Context Task"))
        .stdout(predicate::str::contains("Description: Deep description"))
        .stdout(predicate::str::contains("Status: In Progress"))
        .stdout(predicate::str::contains("Assignee: Tester"));
}

#[test]
fn test_mv_and_rm_and_clear() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("init")
        .assert()
        .success();

    // Create a file to move
    let file_path = root.join("old.txt");
    std::fs::write(&file_path, "content").unwrap();

    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("add")
        .arg("Move Task")
        .arg("-f")
        .arg("old.txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Task created with ID: 1"));

    // Move file
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("mv")
        .arg("old.txt")
        .arg("new.txt")
        .assert()
        .success();

    // Verify link updated in context
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("context")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("- new.txt"));

    // Add another task and close it
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("add")
        .arg("To Clear")
        .assert()
        .success()
        .stdout(predicate::str::contains("Task created with ID: 2"));

    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("close")
        .arg("2")
        .assert()
        .success();

    // Clear
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("clear")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleared 1 closed tasks"));

    // Verify task 2 is gone
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("To Clear").not());

    // Rm
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("rm")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Task 1 removed permanently"));

    // Verify list empty
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("ID").and(predicate::str::contains("Move Task").not()));
}

#[test]
fn test_subdir_access() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let sub = root.join("a/b/c");
    std::fs::create_dir_all(&sub).unwrap();

    // Init at root
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("init")
        .assert()
        .success();

    // Add task from root
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("add")
        .arg("Root Task")
        .assert()
        .success();

    // List from deep subdir
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(&sub)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Root Task"));

    // Add task from subdir
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(&sub)
        .arg("add")
        .arg("Sub Task")
        .assert()
        .success();

    // Verify both exist in list
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Root Task"))
        .stdout(predicate::str::contains("Sub Task"));
}
