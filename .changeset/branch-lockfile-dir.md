---
"@pnpm/config.reader": minor
"@pnpm/lockfile.fs": minor
"@pnpm/installing.context": minor
"@pnpm/installing.deps-installer": minor
"@pnpm/installing.deps-restorer": minor
"@pnpm/installing.commands": minor
"@pnpm/deps.status": minor
"pnpm": minor
---

Added a `branch-lockfile-dir` setting that controls where git-branch-named lockfiles (`pnpm-lock.<branch>.yaml`, created with `use-git-branch-lockfile`) are written. Defaults to `lockfile-dir` (today's behavior, the project root), so existing setups are unaffected. Set it to a subdirectory like `.pnpm/lockfiles` to keep the project root free of one lockfile per branch.
