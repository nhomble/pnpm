use crate::{
    ImporterDepVersion, Lockfile, ProjectSnapshot, ResolvedDependencyMap, ResolvedDependencySpec,
    SnapshotDepRef, SnapshotEntry,
};
use node_semver::Version;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    str::FromStr,
};

/// Merges two lockfiles that diverged from a common ancestor — the
/// counterpart read applies when `mergeGitBranchLockfiles` folds every
/// git-branch-named lockfile into the wanted one.
///
/// Mirrors pnpm's `mergeLockfileChanges`
/// (`lockfile/merger/src/index.ts`): per-importer dependency maps merge
/// key-by-key, preferring the higher semver version (or `theirs` for
/// non-semver refs like `link:`/`file:`); `packages`/`snapshots` entries
/// merge the same way for their own dependency maps, with `theirs`
/// winning wholesale on every other field. Like the TS implementation,
/// the result carries only `importers`, `packages`, `snapshots`,
/// `lockfileVersion`, `pnpmfileChecksum`, and `ignoredOptionalDependencies`
/// — `settings`, `catalogs`, `overrides`, and `patchedDependencies` are
/// not merged and come back empty, matching upstream.
#[must_use]
pub fn merge_lockfile_changes(ours: &Lockfile, theirs: &Lockfile) -> Lockfile {
    let lockfile_version = if theirs.lockfile_version.minor > ours.lockfile_version.minor {
        theirs.lockfile_version
    } else {
        ours.lockfile_version
    };

    let pnpmfile_checksum =
        ours.pnpmfile_checksum.clone().or_else(|| theirs.pnpmfile_checksum.clone());

    let ignored_optional_dependencies = merge_string_sets(
        ours.ignored_optional_dependencies.as_deref(),
        theirs.ignored_optional_dependencies.as_deref(),
    );

    let importer_ids = union_keys(&ours.importers, &theirs.importers);
    let importers = importer_ids
        .into_iter()
        .map(|importer_id| {
            let our_project = ours.importers.get(&importer_id);
            let their_project = theirs.importers.get(&importer_id);
            (importer_id, merge_project_snapshot(our_project, their_project))
        })
        .collect();

    let packages = merge_wholesale_maps(ours.packages.as_ref(), theirs.packages.as_ref());

    let snapshots =
        merge_maps(ours.snapshots.as_ref(), theirs.snapshots.as_ref(), merge_snapshot_entry);

    Lockfile {
        lockfile_version,
        settings: None,
        catalogs: None,
        overrides: None,
        package_extensions_checksum: None,
        pnpmfile_checksum,
        patched_dependencies: None,
        ignored_optional_dependencies,
        importers,
        packages,
        snapshots,
    }
}

fn union_keys<Key: Clone + Eq + Hash, Value>(
    ours: &HashMap<Key, Value>,
    theirs: &HashMap<Key, Value>,
) -> HashSet<Key> {
    ours.keys().chain(theirs.keys()).cloned().collect()
}

/// `theirs`'s entry wins wholesale wherever both sides have one; falls
/// back to whichever side has it when only one does. Used for
/// `packages:` (metadata with no dependency map of its own to merge).
fn merge_wholesale_maps<Key: Clone + Eq + Hash, Value: Clone>(
    ours: Option<&HashMap<Key, Value>>,
    theirs: Option<&HashMap<Key, Value>>,
) -> Option<HashMap<Key, Value>> {
    let empty = HashMap::new();
    let ours = ours.unwrap_or(&empty);
    let theirs = theirs.unwrap_or(&empty);
    let merged: HashMap<Key, Value> = union_keys(ours, theirs)
        .into_iter()
        .filter_map(|key| {
            let value = theirs.get(&key).or_else(|| ours.get(&key))?.clone();
            Some((key, value))
        })
        .collect();
    (!merged.is_empty()).then_some(merged)
}

fn merge_maps<Key: Clone + Eq + Hash, Value>(
    ours: Option<&HashMap<Key, Value>>,
    theirs: Option<&HashMap<Key, Value>>,
    merge_value: impl Fn(Option<&Value>, Option<&Value>) -> Value,
) -> Option<HashMap<Key, Value>> {
    let empty = HashMap::new();
    let ours = ours.unwrap_or(&empty);
    let theirs = theirs.unwrap_or(&empty);
    let merged: HashMap<Key, Value> = union_keys(ours, theirs)
        .into_iter()
        .map(|key| {
            let value = merge_value(ours.get(&key), theirs.get(&key));
            (key, value)
        })
        .collect();
    (!merged.is_empty()).then_some(merged)
}

fn merge_project_snapshot(
    ours: Option<&ProjectSnapshot>,
    theirs: Option<&ProjectSnapshot>,
) -> ProjectSnapshot {
    let default = ProjectSnapshot::default();
    let ours = ours.unwrap_or(&default);
    let theirs = theirs.unwrap_or(&default);
    ProjectSnapshot {
        specifiers: merge_maps(
            ours.specifiers.as_ref(),
            theirs.specifiers.as_ref(),
            |our_value, their_value| {
                take_changed_string(our_value.map(String::as_str), their_value.map(String::as_str))
                    .unwrap_or_default()
            },
        ),
        dependencies: merge_dependency_map(
            ours.dependencies.as_ref(),
            theirs.dependencies.as_ref(),
        ),
        dev_dependencies: merge_dependency_map(
            ours.dev_dependencies.as_ref(),
            theirs.dev_dependencies.as_ref(),
        ),
        optional_dependencies: merge_dependency_map(
            ours.optional_dependencies.as_ref(),
            theirs.optional_dependencies.as_ref(),
        ),
        dependencies_meta: theirs
            .dependencies_meta
            .clone()
            .or_else(|| ours.dependencies_meta.clone()),
        publish_directory: theirs
            .publish_directory
            .clone()
            .or_else(|| ours.publish_directory.clone()),
    }
}

fn merge_dependency_map(
    ours: Option<&ResolvedDependencyMap>,
    theirs: Option<&ResolvedDependencyMap>,
) -> Option<ResolvedDependencyMap> {
    merge_maps(ours, theirs, |our_spec, their_spec| ResolvedDependencySpec {
        specifier: take_changed_string(
            our_spec.map(|spec| spec.specifier.as_str()),
            their_spec.map(|spec| spec.specifier.as_str()),
        )
        .unwrap_or_default(),
        version: merge_ref::<ImporterDepVersion>(
            our_spec.map(|spec| &spec.version),
            their_spec.map(|spec| &spec.version),
        ),
    })
}

fn merge_snapshot_entry(
    ours: Option<&SnapshotEntry>,
    theirs: Option<&SnapshotEntry>,
) -> SnapshotEntry {
    let mut merged = theirs.or(ours).cloned().unwrap_or_default();
    merged.dependencies = merge_maps(
        ours.and_then(|entry| entry.dependencies.as_ref()),
        theirs.and_then(|entry| entry.dependencies.as_ref()),
        merge_ref::<SnapshotDepRef>,
    );
    merged.optional_dependencies = merge_maps(
        ours.and_then(|entry| entry.optional_dependencies.as_ref()),
        theirs.and_then(|entry| entry.optional_dependencies.as_ref()),
        merge_ref::<SnapshotDepRef>,
    );
    merged
}

/// Merges one dependency reference, preferring the higher semver
/// version. Mirrors pnpm's `mergeVersions`: equal or missing `theirs`
/// keeps `ours`; missing `ours` takes `theirs`; otherwise the version
/// before any `(peer@suffix)` is compared as semver, falling back to
/// `theirs` when either side doesn't parse (`link:` / `file:` refs and
/// the like).
fn merge_ref<Ref>(ours: Option<&Ref>, theirs: Option<&Ref>) -> Ref
where
    Ref: Clone + PartialEq + FromStr + Into<String>,
{
    let (ours, theirs) = match (ours, theirs) {
        (Some(ours), Some(theirs)) => (ours, theirs),
        (ours, theirs) => {
            return theirs
                .or(ours)
                .cloned()
                .expect("at least one side is present at every merged key");
        }
    };
    if ours == theirs {
        return ours.clone();
    }
    let our_string: String = ours.clone().into();
    let their_string: String = theirs.clone().into();
    let our_version = our_string.split('(').next().unwrap_or(&our_string);
    let their_version = their_string.split('(').next().unwrap_or(&their_string);
    let merged_string = match (Version::from_str(our_version), Version::from_str(their_version)) {
        (Ok(our_semver), Ok(their_semver)) => {
            if our_semver > their_semver {
                our_string
            } else {
                their_string
            }
        }
        _ => their_string,
    };
    Ref::from_str(&merged_string).unwrap_or_else(|_| theirs.clone())
}

fn take_changed_string(ours: Option<&str>, theirs: Option<&str>) -> Option<String> {
    match (ours, theirs) {
        (_, Some(theirs)) if ours != Some(theirs) => Some(theirs.to_owned()),
        (Some(ours), _) => Some(ours.to_owned()),
        (None, Some(theirs)) => Some(theirs.to_owned()),
        (None, None) => None,
    }
}

fn merge_string_sets(ours: Option<&[String]>, theirs: Option<&[String]>) -> Option<Vec<String>> {
    let mut merged: Vec<String> = ours.iter().flat_map(|slice| slice.iter()).cloned().collect();
    for dep in theirs.iter().flat_map(|slice| slice.iter()) {
        if !merged.contains(dep) {
            merged.push(dep.clone());
        }
    }
    (!merged.is_empty()).then_some(merged)
}

#[cfg(test)]
mod tests;
