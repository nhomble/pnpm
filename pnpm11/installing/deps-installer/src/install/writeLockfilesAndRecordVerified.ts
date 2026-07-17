import path from 'node:path'

import { WANTED_LOCKFILE } from '@pnpm/constants'
import { getWantedLockfileName, type LockfileObject, writeLockfiles, type WriteLockfilesResult } from '@pnpm/lockfile.fs'
import type { ResolutionVerifier } from '@pnpm/resolving.resolver-base'

import { recordLockfileVerified } from './recordLockfileVerified.js'

export interface WriteLockfilesAndRecordVerifiedOptions {
  wantedLockfile: LockfileObject
  wantedLockfileDir: string
  currentLockfile: LockfileObject
  currentLockfileDir: string
  useGitBranchLockfile?: boolean
  branchLockfileDir?: string
  mergeGitBranchLockfiles?: boolean
  cacheDir?: string
  resolutionVerifiers: readonly ResolutionVerifier[] | undefined
}

/** Plural counterpart of {@link writeWantedLockfileAndRecordVerified}. */
export async function writeLockfilesAndRecordVerified (
  opts: WriteLockfilesAndRecordVerifiedOptions
): Promise<WriteLockfilesResult> {
  const cacheActive = opts.cacheDir != null && (opts.resolutionVerifiers?.length ?? 0) > 0
  const wantedLockfileName = cacheActive
    ? await getWantedLockfileName({
      useGitBranchLockfile: opts.useGitBranchLockfile,
      mergeGitBranchLockfiles: opts.mergeGitBranchLockfiles,
    })
    : undefined
  const written = await writeLockfiles({
    wantedLockfile: opts.wantedLockfile,
    wantedLockfileDir: opts.wantedLockfileDir,
    currentLockfile: opts.currentLockfile,
    currentLockfileDir: opts.currentLockfileDir,
    useGitBranchLockfile: opts.useGitBranchLockfile,
    mergeGitBranchLockfiles: opts.mergeGitBranchLockfiles,
    branchLockfileDir: opts.branchLockfileDir,
    wantedLockfileName,
  })
  if (cacheActive) {
    const writtenLockfileDir = wantedLockfileName !== WANTED_LOCKFILE && opts.branchLockfileDir ? opts.branchLockfileDir : opts.wantedLockfileDir
    recordLockfileVerified({
      cacheDir: opts.cacheDir,
      lockfilePath: path.resolve(writtenLockfileDir, wantedLockfileName!),
      lockfile: written.wantedLockfile,
      resolutionVerifiers: opts.resolutionVerifiers,
    })
  }
  return written
}
