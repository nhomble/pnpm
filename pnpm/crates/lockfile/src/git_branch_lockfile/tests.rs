use super::{
    clean_git_branch_lockfiles, current_branch, git_branch_lockfile_name,
    is_git_branch_lockfile_name, list_git_branch_lockfiles,
};
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

#[test]
fn sanitizes_slashes_and_uppercase() {
    assert_eq!(git_branch_lockfile_name("feature/Foo"), "pnpm-lock.feature!foo.yaml");
}

#[test]
fn keeps_dots_hyphens_and_underscores() {
    assert_eq!(git_branch_lockfile_name("release-1.2_rc"), "pnpm-lock.release-1.2_rc.yaml");
}

#[test]
fn requires_literal_dots_and_a_non_empty_branch_segment() {
    assert!(is_git_branch_lockfile_name("pnpm-lock.main.yaml"));
    assert!(is_git_branch_lockfile_name("pnpm-lock.feature.x.yaml"));
    assert!(!is_git_branch_lockfile_name("pnpm-lock.yaml"));
    assert!(!is_git_branch_lockfile_name("pnpm-lock-main-yaml"));
    assert!(!is_git_branch_lockfile_name("my-pnpm-lock.main.yaml"));
    assert!(!is_git_branch_lockfile_name("README.md"));
}

fn write(dir: &Path, name: &str) {
    fs::write(dir.join(name), "").unwrap();
}

#[test]
fn lists_only_git_branch_lockfiles() {
    let dir = TempDir::new().unwrap();
    for name in [
        "pnpm-lock.main.yaml",
        "pnpm-lock.feature.x.yaml",
        "pnpm-lock.yaml",
        "pnpm-lock-main-yaml",
        "README.md",
    ] {
        write(dir.path(), name);
    }

    let mut names: Vec<String> = list_git_branch_lockfiles(dir.path())
        .unwrap()
        .into_iter()
        .map(|path| path.file_name().unwrap().to_str().unwrap().to_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["pnpm-lock.feature.x.yaml".to_owned(), "pnpm-lock.main.yaml".to_owned()]
    );
}

#[test]
fn list_returns_empty_when_dir_is_missing() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("does-not-exist");
    assert_eq!(list_git_branch_lockfiles(&missing).unwrap(), Vec::<std::path::PathBuf>::new());
}

#[test]
fn clean_deletes_branch_lockfiles_but_not_the_main_one() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "pnpm-lock.main.yaml");
    write(dir.path(), "pnpm-lock.yaml");

    clean_git_branch_lockfiles(dir.path()).unwrap();

    assert!(!dir.path().join("pnpm-lock.main.yaml").exists());
    assert!(dir.path().join("pnpm-lock.yaml").exists());
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git").args(args).current_dir(dir).status().unwrap();
    assert!(status.success(), "git {args:?} failed in {dir:?}");
}

#[test]
fn current_branch_reads_the_checked_out_branch() {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "--initial-branch=main", "-q"]);
    git(dir.path(), &["config", "user.email", "test@example.com"]);
    git(dir.path(), &["config", "user.name", "Test"]);
    fs::write(dir.path().join("file.txt"), "content").unwrap();
    git(dir.path(), &["add", "."]);
    git(dir.path(), &["commit", "-q", "-m", "init"]);
    git(dir.path(), &["checkout", "-q", "-b", "feature/foo"]);

    assert_eq!(current_branch(dir.path()), Some("feature/foo".to_owned()));
}

#[test]
fn current_branch_is_none_on_detached_head() {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "--initial-branch=main", "-q"]);
    git(dir.path(), &["config", "user.email", "test@example.com"]);
    git(dir.path(), &["config", "user.name", "Test"]);
    fs::write(dir.path().join("file.txt"), "content").unwrap();
    git(dir.path(), &["add", "."]);
    git(dir.path(), &["commit", "-q", "-m", "init"]);
    git(dir.path(), &["checkout", "-q", "HEAD~0"]);

    assert_eq!(current_branch(dir.path()), None);
}

#[test]
fn current_branch_is_none_outside_a_git_repo() {
    let dir = TempDir::new().unwrap();
    assert_eq!(current_branch(dir.path()), None);
}
