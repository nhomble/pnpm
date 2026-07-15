use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

/// Every git-branch-named lockfile starts with this prefix and ends with
/// [`FILE_NAME_SUFFIX`], with a non-empty branch slug in between.
const FILE_NAME_PREFIX: &str = "pnpm-lock.";
const FILE_NAME_SUFFIX: &str = ".yaml";

/// Builds the git-branch-named lockfile file name for `branch`.
///
/// Every character outside `[A-Za-z0-9_.-]` becomes `!` (branch names may
/// contain `/`, which isn't valid in a file name) and the result is
/// lowercased (the file system may be case-insensitive). Mirrors pnpm's
/// `stringifyBranchName` (`lockfile/fs/src/lockfileName.ts`).
#[must_use]
pub fn git_branch_lockfile_name(branch: &str) -> String {
    let slug: String = branch
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') { c } else { '!' })
        .collect::<String>()
        .to_lowercase();
    format!("{FILE_NAME_PREFIX}{slug}{FILE_NAME_SUFFIX}")
}

/// Reads the current git branch name by running `git symbolic-ref --short
/// HEAD` in `dir`. `None` when `dir` isn't a git repository or `HEAD` is
/// detached. Mirrors pnpm's `getCurrentBranch`
/// (`network/git-utils/src/index.ts`) — pacquet always takes the
/// `symbolic-ref` path rather than parsing `.git/HEAD` directly.
#[must_use]
pub fn current_branch(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|branch| branch.trim().to_owned())
        .filter(|branch| !branch.is_empty())
}

/// `true` when `file_name` is a git-branch-named lockfile: it starts with
/// `pnpm-lock.`, ends with `.yaml`, and the segment in between is
/// non-empty. Excludes the plain `pnpm-lock.yaml`. Mirrors
/// pnpm's `GIT_BRANCH_LOCKFILE_NAME` regex
/// (`lockfile/fs/src/gitBranchLockfile.ts`).
#[must_use]
pub fn is_git_branch_lockfile_name(file_name: &str) -> bool {
    file_name
        .strip_prefix(FILE_NAME_PREFIX)
        .and_then(|rest| rest.strip_suffix(FILE_NAME_SUFFIX))
        .is_some_and(|slug| !slug.is_empty())
}

/// Lists every git-branch-named lockfile in `dir`. Returns an empty list
/// when `dir` doesn't exist. Mirrors pnpm's `getGitBranchLockfileNames`
/// (`lockfile/fs/src/gitBranchLockfile.ts`).
pub fn list_git_branch_lockfiles(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let read_dir = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut matches = Vec::new();
    for entry in read_dir {
        let path = entry?.path();
        let is_branch_lockfile = path
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .is_some_and(is_git_branch_lockfile_name);
        if is_branch_lockfile {
            matches.push(path);
        }
    }
    Ok(matches)
}

/// Deletes every git-branch-named lockfile in `dir`. Mirrors pnpm's
/// `cleanGitBranchLockfiles` (`lockfile/fs/src/gitBranchLockfile.ts`).
pub fn clean_git_branch_lockfiles(dir: &Path) -> io::Result<()> {
    list_git_branch_lockfiles(dir)?.into_iter().try_for_each(fs::remove_file)
}

#[cfg(test)]
mod tests;
