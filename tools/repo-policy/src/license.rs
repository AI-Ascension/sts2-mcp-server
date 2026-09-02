// SPDX-License-Identifier: MIT

use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::Finding;
use crate::files::relative_text;

const SPDX: &str = "SPDX-License-Identifier: MIT";

pub(crate) fn findings(root: &Path, files: &[PathBuf]) -> Vec<Finding> {
    let mut findings = Vec::new();
    check_root_license(root, &mut findings);
    for path in files {
        if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("rs" | "cs")
        ) {
            check_source_header(root, path, &mut findings);
        }
        if path.file_name().and_then(|value| value.to_str()) == Some("Cargo.toml") {
            check_manifest(root, path, &mut findings);
        }
    }
    findings
}

fn check_root_license(root: &Path, findings: &mut Vec<Finding>) {
    let path = root.join("LICENSE");
    match fs::read_to_string(&path) {
        Ok(text) if text.starts_with("MIT License\n") => {}
        Ok(_) => findings.push(Finding::error(
            "LIC001",
            "LICENSE",
            "root license must use the MIT text",
        )),
        Err(error) => findings.push(Finding::error(
            "LIC001",
            "LICENSE",
            format!("cannot read root license: {error}"),
        )),
    }
}

fn check_source_header(root: &Path, path: &Path, findings: &mut Vec<Finding>) {
    let relative = relative_text(root, path);
    match fs::read_to_string(path) {
        Ok(text) if text.lines().take(8).any(|line| line.contains(SPDX)) => {}
        Ok(_) => findings.push(Finding::error(
            "LIC002",
            &relative,
            format!("missing {SPDX} in first eight lines"),
        )),
        Err(error) => findings.push(Finding::error(
            "LIC002",
            &relative,
            format!("cannot read source: {error}"),
        )),
    }
}

fn check_manifest(root: &Path, path: &Path, findings: &mut Vec<Finding>) {
    let relative = relative_text(root, path);
    match fs::read_to_string(path) {
        Ok(text) if valid_manifest_license(root, path, &text) => {}
        Ok(_) => findings.push(Finding::error(
            "LIC003",
            &relative,
            "Cargo package license must be MIT or inherited from the MIT workspace",
        )),
        Err(error) => findings.push(Finding::error(
            "LIC003",
            &relative,
            format!("cannot read Cargo manifest: {error}"),
        )),
    }
}

fn valid_manifest_license(root: &Path, path: &Path, text: &str) -> bool {
    if path == root.join("Cargo.toml") {
        return section_has_setting(text, "[workspace.package]", "license = \"MIT\"");
    }
    text.lines().any(|line| {
        matches!(
            line.trim(),
            "license.workspace = true" | "license = \"MIT\""
        )
    })
}

fn section_has_setting(text: &str, section: &str, expected: &str) -> bool {
    let mut active = false;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            active = line == section;
            continue;
        }
        if active && line == expected {
            return true;
        }
    }
    false
}
