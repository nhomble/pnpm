import fs from 'node:fs'
import path from 'node:path'

import { WANTED_LOCKFILE } from '@pnpm/constants'

import { getWantedLockfileName } from './lockfileName.js'

interface ExistsNonEmptyWantedLockfileOptions {
  useGitBranchLockfile?: boolean
  mergeGitBranchLockfiles?: boolean
  /** Directory to look for the git-branch-named lockfile in, if it differs from `pkgPath`. */
  branchLockfileDir?: string
}

export async function existsNonEmptyWantedLockfile (pkgPath: string, opts: ExistsNonEmptyWantedLockfileOptions = {
  useGitBranchLockfile: false,
  mergeGitBranchLockfiles: false,
}): Promise<boolean> {
  const wantedLockfile: string = await getWantedLockfileName(opts)
  const lockfileDir = wantedLockfile !== WANTED_LOCKFILE ? (opts.branchLockfileDir ?? pkgPath) : pkgPath
  return new Promise<boolean>((resolve, reject) => {
    fs.access(path.join(lockfileDir, wantedLockfile), (err) => {
      if (err == null) {
        resolve(true)
        return
      }
      if (err.code === 'ENOENT') {
        resolve(false)
        return
      }
      reject(err)
    })
  })
}
