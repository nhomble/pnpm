use super::merge_lockfile_changes;
use crate::{
    ImporterDepVersion, Lockfile, LockfileVersion, PkgName, ProjectSnapshot, ResolvedDependencyMap,
    ResolvedDependencySpec,
};
use std::{collections::HashMap, str::FromStr};

fn empty_lockfile() -> Lockfile {
    Lockfile {
        lockfile_version: LockfileVersion::try_from(crate::ComVer::new(9, 0)).unwrap(),
        settings: None,
        catalogs: None,
        overrides: None,
        package_extensions_checksum: None,
        pnpmfile_checksum: None,
        patched_dependencies: None,
        ignored_optional_dependencies: None,
        importers: HashMap::new(),
        packages: None,
        snapshots: None,
    }
}

fn dep(alias: &str, version: &str) -> (PkgName, ResolvedDependencySpec) {
    (
        PkgName::from_str(alias).unwrap(),
        ResolvedDependencySpec {
            specifier: format!("^{version}"),
            version: ImporterDepVersion::from_str(version).unwrap(),
        },
    )
}

#[test]
fn merges_root_importer_dependencies_from_both_sides() {
    let mut ours = empty_lockfile();
    let mut our_deps: ResolvedDependencyMap = HashMap::new();
    our_deps.insert(dep("is-positive", "1.0.0").0, dep("is-positive", "1.0.0").1);
    ours.importers.insert(
        Lockfile::ROOT_IMPORTER_KEY.to_owned(),
        ProjectSnapshot { dependencies: Some(our_deps), ..Default::default() },
    );

    let mut theirs = empty_lockfile();
    let mut their_deps: ResolvedDependencyMap = HashMap::new();
    their_deps.insert(dep("is-negative", "2.0.0").0, dep("is-negative", "2.0.0").1);
    theirs.importers.insert(
        Lockfile::ROOT_IMPORTER_KEY.to_owned(),
        ProjectSnapshot { dependencies: Some(their_deps), ..Default::default() },
    );

    let merged = merge_lockfile_changes(&ours, &theirs);
    let root = merged.importers.get(Lockfile::ROOT_IMPORTER_KEY).unwrap();
    let deps = root.dependencies.as_ref().unwrap();
    assert_eq!(deps.len(), 2);
    assert!(deps.contains_key(&PkgName::from_str("is-positive").unwrap()));
    assert!(deps.contains_key(&PkgName::from_str("is-negative").unwrap()));
}

#[test]
fn prefers_the_higher_semver_version_on_conflict() {
    let mut ours = empty_lockfile();
    let mut our_deps: ResolvedDependencyMap = HashMap::new();
    our_deps.insert(dep("is-positive", "1.0.0").0, dep("is-positive", "1.0.0").1);
    ours.importers.insert(
        Lockfile::ROOT_IMPORTER_KEY.to_owned(),
        ProjectSnapshot { dependencies: Some(our_deps), ..Default::default() },
    );

    let mut theirs = empty_lockfile();
    let mut their_deps: ResolvedDependencyMap = HashMap::new();
    their_deps.insert(dep("is-positive", "2.0.0").0, dep("is-positive", "2.0.0").1);
    theirs.importers.insert(
        Lockfile::ROOT_IMPORTER_KEY.to_owned(),
        ProjectSnapshot { dependencies: Some(their_deps), ..Default::default() },
    );

    let merged = merge_lockfile_changes(&ours, &theirs);
    let root = merged.importers.get(Lockfile::ROOT_IMPORTER_KEY).unwrap();
    let deps = root.dependencies.as_ref().unwrap();
    let is_positive = deps.get(&PkgName::from_str("is-positive").unwrap()).unwrap();
    assert_eq!(is_positive.version, ImporterDepVersion::from_str("2.0.0").unwrap());
}

#[test]
fn ignored_optional_dependencies_union_without_duplicates() {
    let mut ours = empty_lockfile();
    ours.ignored_optional_dependencies = Some(vec!["fsevents".to_owned()]);
    let mut theirs = empty_lockfile();
    theirs.ignored_optional_dependencies =
        Some(vec!["fsevents".to_owned(), "bufferutil".to_owned()]);

    let merged = merge_lockfile_changes(&ours, &theirs);
    let mut merged_list = merged.ignored_optional_dependencies.unwrap();
    merged_list.sort();
    assert_eq!(merged_list, vec!["bufferutil".to_owned(), "fsevents".to_owned()]);
}
