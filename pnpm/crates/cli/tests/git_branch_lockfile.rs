//! End-to-end coverage for `gitBranchLockfile` / `branchLockfileDir`:
//! a fresh `pacquet install` on a named git branch writes
//! `pnpm-lock.<branch>.yaml` instead of `pnpm-lock.yaml`, optionally
//! redirected to a separate directory via `branchLockfileDir`.

#![cfg(unix)] // pnpm CLI: 'program not found' on Windows runners.

pub mod _utils;
pub use _utils::*;

use assert_cmd::prelude::*;
use command_extra::CommandExtra;
use pacquet_testing_utils::bin::{AddMockedRegistry, CommandTempCwd};
use std::{fs, path::Path, process::Command};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git").args(args).current_dir(dir).status().unwrap();
    assert!(status.success(), "git {args:?} failed in {dir:?}");
}

fn init_repo_on_branch(dir: &Path, branch: &str) {
    git(dir, &["init", "--initial-branch", branch, "-q"]);
    git(dir, &["config", "user.email", "test@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
}

fn write_manifest(workspace: &Path) {
    let manifest = serde_json::json!({ "dependencies": { "is-positive": "1.0.0" } });
    fs::write(workspace.join("package.json"), manifest.to_string()).expect("write package.json");
}

fn append_workspace_yaml(workspace: &Path, extra: &str) {
    let path = workspace.join("pnpm-workspace.yaml");
    let existing = fs::read_to_string(&path).expect("read pnpm-workspace.yaml");
    fs::write(&path, format!("{existing}{extra}")).expect("write pnpm-workspace.yaml");
}

/// `gitBranchLockfile: true` writes `pnpm-lock.<branch>.yaml` next to
/// the main lockfile location when `branchLockfileDir` is unset.
#[test]
fn writes_branch_named_lockfile_next_to_main_location() {
    let CommandTempCwd { pacquet, root, workspace, npmrc_info, .. } =
        CommandTempCwd::init().add_mocked_registry();
    let AddMockedRegistry { mock_instance, .. } = npmrc_info;

    init_repo_on_branch(&workspace, "feature");
    write_manifest(&workspace);
    append_workspace_yaml(&workspace, "gitBranchLockfile: true\n");

    pacquet.with_args(["install", "--ignore-scripts"]).assert().success();

    assert!(
        !workspace.join("pnpm-lock.yaml").exists(),
        "plain pnpm-lock.yaml should not be written when gitBranchLockfile is on",
    );
    assert!(
        workspace.join("pnpm-lock.feature.yaml").exists(),
        "pnpm-lock.feature.yaml should be written next to the main lockfile location",
    );

    drop((root, mock_instance));
}

/// `branchLockfileDir` redirects the git-branch-named lockfile into a
/// separate directory, created on demand, while the main lockfile
/// location stays untouched.
#[test]
fn writes_branch_named_lockfile_into_branch_lockfile_dir() {
    let CommandTempCwd { pacquet, root, workspace, npmrc_info, .. } =
        CommandTempCwd::init().add_mocked_registry();
    let AddMockedRegistry { mock_instance, .. } = npmrc_info;

    init_repo_on_branch(&workspace, "feature");
    write_manifest(&workspace);
    append_workspace_yaml(
        &workspace,
        "gitBranchLockfile: true\nbranchLockfileDir: .pnpm/lockfiles\n",
    );

    pacquet.with_args(["install", "--ignore-scripts"]).assert().success();

    assert!(
        !workspace.join("pnpm-lock.yaml").exists(),
        "plain pnpm-lock.yaml should not be written when gitBranchLockfile is on",
    );
    assert!(
        !workspace.join("pnpm-lock.feature.yaml").exists(),
        "branch lockfile should not land next to the main lockfile location",
    );
    assert!(
        workspace.join(".pnpm/lockfiles/pnpm-lock.feature.yaml").exists(),
        "branch lockfile should be written under branchLockfileDir",
    );

    drop((root, mock_instance));
}
