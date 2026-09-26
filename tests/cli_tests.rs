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

#[test]
fn test_cli_attach() {
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
        .arg("Attach Task")
        .assert()
        .success();

    let file_path = root.join("attach_me.txt");
    std::fs::write(&file_path, "content").unwrap();

    // Attach
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("attach")
        .arg("1")
        .arg("attach_me.txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Files attached to task 1"));

    // Verify in context
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("context")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("- attach_me.txt"));

    // Fail if file doesn't exist
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .arg("attach")
        .arg("1")
        .arg("non_existent.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("File does not exist"));
}

#[test]
fn test_tui_uninitialized() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Run 'lissue' without arguments in an uninitialized directory
    let mut cmd = Command::cargo_bin("lissue").unwrap();
    cmd.current_dir(root)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not initialized"));
}

#[test]
fn test_add_invalid_file_does_not_persist_task() {
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
        .arg("Invalid attachment")
        .arg("-f")
        .arg("missing.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("File does not exist"));

    let output = Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let tasks: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 0);
    assert!(
        walkdir::WalkDir::new(root.join(".lissue/tasks"))
            .into_iter()
            .filter_map(Result::ok)
            .all(|entry| entry.path().extension().and_then(|ext| ext.to_str()) != Some("json"))
    );
}

#[test]
fn test_mv_failures_preserve_file_and_link() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("init")
        .assert()
        .success();
    std::fs::write(root.join("old.txt"), "original").unwrap();

    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("add")
        .arg("Move failures")
        .arg("-f")
        .arg("old.txt")
        .assert()
        .success();

    // Missing source must not alter the task or create a destination.
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .args(["mv", "missing.txt", "new.txt"])
        .assert()
        .failure();
    assert_eq!(
        std::fs::read_to_string(root.join("old.txt")).unwrap(),
        "original"
    );
    assert!(!root.join("new.txt").exists());

    // Missing destination parent must fail before any metadata update.
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .args(["mv", "old.txt", "missing-dir/new.txt"])
        .assert()
        .failure();
    assert_eq!(
        std::fs::read_to_string(root.join("old.txt")).unwrap(),
        "original"
    );
    assert!(!root.join("missing-dir/new.txt").exists());

    // An existing destination must never be overwritten.
    std::fs::write(root.join("new.txt"), "destination").unwrap();
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .args(["mv", "old.txt", "new.txt"])
        .assert()
        .failure();
    assert_eq!(
        std::fs::read_to_string(root.join("old.txt")).unwrap(),
        "original"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("new.txt")).unwrap(),
        "destination"
    );

    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("context")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("- old.txt"));
}

#[cfg(unix)]
#[test]
fn test_mv_partial_git_move_rolls_back_worktree_and_index() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command as ProcessCommand;

    let dir = tempdir().unwrap();
    let root = dir.path();

    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("init")
        .assert()
        .success();
    std::fs::write(root.join("old.txt"), "tracked content").unwrap();
    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("add")
        .arg("Partial git move")
        .arg("-f")
        .arg("old.txt")
        .assert()
        .success();

    let git_path = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("git executable must be available for this regression");
    assert!(
        ProcessCommand::new(&git_path)
            .current_dir(root)
            .arg("init")
            .status()
            .unwrap()
            .success()
    );
    assert!(
        ProcessCommand::new(&git_path)
            .current_dir(root)
            .args(["add", "old.txt"])
            .status()
            .unwrap()
            .success()
    );

    let fake_bin = root.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let fake_git = fake_bin.join("git");
    let git_path_display = git_path.display().to_string();
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"mv\" ] && [ \"$2\" = \"old.txt\" ]; then\n  \"{git_path_display}\" \"$@\"\n  exit 1\nfi\nexec \"{git_path_display}\" \"$@\"\n"
    );
    std::fs::write(&fake_git, script).unwrap();
    std::fs::set_permissions(&fake_git, std::fs::Permissions::from_mode(0o755)).unwrap();

    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .env("PATH", fake_bin)
        .args(["mv", "old.txt", "new.txt"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("rollback succeeded"));

    assert_eq!(
        std::fs::read_to_string(root.join("old.txt")).unwrap(),
        "tracked content"
    );
    assert!(!root.join("new.txt").exists());
    let staged = ProcessCommand::new(&git_path)
        .current_dir(root)
        .args(["diff", "--cached", "--name-status"])
        .output()
        .unwrap();
    assert!(staged.status.success());
    assert_eq!(String::from_utf8_lossy(&staged.stdout), "A\told.txt\n");

    Command::cargo_bin("lissue")
        .unwrap()
        .current_dir(root)
        .arg("context")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("- old.txt"));
}
