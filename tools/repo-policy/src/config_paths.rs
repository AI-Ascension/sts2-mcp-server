// SPDX-License-Identifier: MIT

use super::MANAGED_STANDARDS_PATH;
use std::collections::BTreeSet;
use std::path::{Component, Path};

pub(super) fn validate_exact_paths(values: &[String], key: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values {
        validate_exact_path(value, key)?;
        if !seen.insert(value) {
            return Err(format!("{key} contains duplicate path {value}"));
        }
    }
    Ok(())
}

pub(super) fn validate_ignored_directories(values: &[String]) -> Result<BTreeSet<String>, String> {
    let mut directories = BTreeSet::new();
    for directory in values {
        let path = Path::new(directory);
        let safe = !directory.is_empty()
            && !directory.contains('\\')
            && !directory.contains('/')
            && !directory.contains('*')
            && !directory.contains('?')
            && path.components().count() == 1
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
            && !directory.eq_ignore_ascii_case("standards");
        if !safe {
            return Err(format!(
                "ignored_directories must contain one safe directory name and cannot hide standards: {directory}"
            ));
        }
        if !directories.insert(directory.clone()) {
            return Err(format!(
                "ignored_directories contains duplicate directory {directory}"
            ));
        }
    }
    Ok(directories)
}

pub(super) fn validate_exact_path(value: &str, key: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains('*')
        || value.contains('?')
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "{key} must contain exact repository-relative paths: {value}"
        ));
    }
    Ok(())
}

pub(super) fn validate_ignored_path_prefixes(
    values: &[String],
) -> Result<BTreeSet<String>, String> {
    let mut prefixes = BTreeSet::new();
    for prefix in values {
        let path = Path::new(prefix);
        let safe = !prefix.is_empty()
            && !prefix.contains('\\')
            && !prefix.starts_with('/')
            && !prefix.ends_with('/')
            && !prefix.contains('*')
            && !prefix.contains('?')
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !safe {
            return Err(format!(
                "ignored_path_prefixes must contain exact repository-relative paths: {prefix}"
            ));
        }
        if prefix == "standards"
            || prefix == "standards/tools"
            || MANAGED_STANDARDS_PATH.starts_with(&format!("{prefix}/"))
        {
            return Err(format!(
                "ignored_path_prefixes cannot hide the standards root; use the exact managed path {MANAGED_STANDARDS_PATH}"
            ));
        }
        if !prefixes.insert(prefix.clone()) {
            return Err(format!(
                "ignored_path_prefixes contains duplicate path {prefix}"
            ));
        }
    }
    Ok(prefixes)
}
