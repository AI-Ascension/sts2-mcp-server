// SPDX-License-Identifier: MIT

use std::fs;
use std::path::Path;

use crate::diagnostic::Finding;

const REQUIRED_FILES: &[&str] = &["Cargo.lock", "rust-toolchain.toml"];

pub(crate) fn findings(root: &Path) -> Vec<Finding> {
    let manifest_path = root.join("Cargo.toml");
    if !manifest_path.is_file() {
        return Vec::new();
    }
    let mut findings = Vec::new();
    for relative in REQUIRED_FILES {
        if !root.join(relative).is_file() {
            findings.push(Finding::error(
                "RUST001",
                relative,
                "required when a Cargo workspace exists",
            ));
        }
    }
    let manifest = read(&manifest_path, "Cargo.toml", &mut findings);
    let toolchain = read(
        &root.join("rust-toolchain.toml"),
        "rust-toolchain.toml",
        &mut findings,
    );
    if let Some(manifest) = manifest {
        check_workspace(&manifest, &mut findings);
        if let Some(toolchain) = toolchain {
            check_toolchain_match(&manifest, &toolchain, &mut findings);
        }
    }
    findings
}

fn read(path: &Path, relative: &str, findings: &mut Vec<Finding>) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) => {
            findings.push(Finding::error(
                "RUST001",
                relative,
                format!("cannot read Rust configuration: {error}"),
            ));
            None
        }
    }
}

fn check_workspace(manifest: &str, findings: &mut Vec<Finding>) {
    for key in ["edition", "rust-version", "license"] {
        if !section_has_key(manifest, "[workspace.package]", key) {
            findings.push(Finding::error(
                "RUST001",
                "Cargo.toml",
                format!("workspace.package.{key} must be declared"),
            ));
        }
    }
    if !section_has_key(manifest, "[workspace.lints.rust]", "unsafe_code")
        && !section_has_key(manifest, "[workspace.lints.clippy]", "all")
    {
        findings.push(Finding::error(
            "RUST001",
            "Cargo.toml",
            "workspace lints must define inherited lint policy",
        ));
    }
}

fn check_toolchain_match(manifest: &str, toolchain: &str, findings: &mut Vec<Finding>) {
    let rust_version = setting_in_section(manifest, "[workspace.package]", "rust-version");
    let channel = setting_in_section(toolchain, "[toolchain]", "channel");
    if rust_version != channel {
        findings.push(Finding::error(
            "RUST001",
            "rust-toolchain.toml",
            format!("toolchain {channel:?} does not match workspace rust-version {rust_version:?}"),
        ));
    }
}

fn section_has_key(text: &str, section: &str, key: &str) -> bool {
    setting_in_section(text, section, key).is_some()
}

fn setting_in_section(text: &str, section: &str, key: &str) -> Option<String> {
    let mut active = false;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            active = line == section;
            continue;
        }
        if active {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            if name.trim() == key {
                return Some(value.trim().trim_matches('"').to_owned());
            }
        }
    }
    None
}
