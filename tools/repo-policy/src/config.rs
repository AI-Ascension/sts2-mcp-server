// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[path = "config_paths.rs"]
mod paths;
use paths::{
    validate_exact_path, validate_exact_paths, validate_ignored_directories,
    validate_ignored_path_prefixes,
};

const LEGACY_POLICY_VERSION: i64 = 1;
const CURRENT_POLICY_VERSION: i64 = 2;
const MANAGED_STANDARDS_PATH: &str = "standards/tools/standards-sync";
const KNOWN_RULES: &[&str] = &[
    "BOUND001", "CFG001", "DOC001", "DOC002", "DOC003", "EXC001", "LANG001", "LIC001", "LIC002",
    "LIC003", "RUST001", "RUST002", "RUST003", "RUST004", "RUST005", "SIZE001", "WF001", "WF002",
    "WF003", "WF004", "WF005",
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum SizeCategory {
    RustProduction,
    RustTest,
    CsharpProduction,
    CsharpTest,
    Workflow,
    Markdown,
}

impl SizeCategory {
    fn key(self) -> &'static str {
        match self {
            Self::RustProduction => "rust_production",
            Self::RustTest => "rust_test",
            Self::CsharpProduction => "csharp_production",
            Self::CsharpTest => "csharp_test",
            Self::Workflow => "workflow",
            Self::Markdown => "markdown",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Budget {
    pub(crate) preferred: usize,
    pub(crate) maximum: usize,
}

#[derive(Debug)]
pub(crate) struct Policy {
    pub(crate) policy_version: i64,
    pub(crate) advisory_rules: BTreeSet<String>,
    pub(crate) required_files: Vec<String>,
    pub(crate) ignored_directories: BTreeSet<String>,
    pub(crate) ignored_path_prefixes: BTreeSet<String>,
    pub(crate) exemptions: BTreeMap<String, String>,
    limits: BTreeMap<SizeCategory, Budget>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Section {
    Root,
    Project,
    Severity,
    Limits,
    Exemptions,
}

impl Policy {
    pub(crate) fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        Self::parse(&text)
    }

    fn parse(text: &str) -> Result<Self, String> {
        let mut section = Section::Root;
        let mut version = None;
        let mut required_files = None;
        let mut ignored_directories = None;
        let mut ignored_path_prefixes = None;
        let mut mandatory_rules = None;
        let mut advisory_rules = None;
        let mut limit_values = BTreeMap::new();
        let mut exemptions = BTreeMap::new();

        for (line_number, raw_line) in text.lines().enumerate() {
            let line = strip_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') {
                section = parse_section(line)
                    .map_err(|error| format!("line {}: {error}", line_number + 1))?;
                continue;
            }
            let (raw_key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("line {}: expected key = value", line_number + 1))?;
            let key = raw_key.trim();
            let value = value.trim();
            match section {
                Section::Root if key == "policy_version" => {
                    version = Some(parse_integer(value, key)?);
                }
                Section::Project => match key {
                    "required_files" => required_files = Some(parse_string_array(value, key)?),
                    "ignored_directories" => {
                        ignored_directories = Some(parse_string_array(value, key)?);
                    }
                    "ignored_path_prefixes" => {
                        ignored_path_prefixes = Some(parse_string_array(value, key)?);
                    }
                    _ => return Err(format!("unknown project key: {key}")),
                },
                Section::Severity => match key {
                    "mandatory" => {
                        mandatory_rules = Some(parse_string_array(value, key)?);
                    }
                    "advisory" => {
                        advisory_rules = Some(parse_string_array(value, key)?);
                    }
                    _ => return Err(format!("unknown severity key: {key}")),
                },
                Section::Limits => {
                    limit_values.insert(key.to_owned(), parse_integer(value, key)?);
                }
                Section::Exemptions => {
                    exemptions.insert(
                        parse_string(key, "exemption path")?,
                        parse_string(value, key)?,
                    );
                }
                Section::Root => return Err(format!("unknown root key: {key}")),
            }
        }

        let policy_version =
            version.ok_or_else(|| "policy_version must be an integer".to_owned())?;
        if !matches!(
            policy_version,
            LEGACY_POLICY_VERSION | CURRENT_POLICY_VERSION
        ) {
            return Err(format!(
                "policy_version must be {LEGACY_POLICY_VERSION} or {CURRENT_POLICY_VERSION}, found {policy_version}"
            ));
        }
        let advisory_rules = if policy_version == LEGACY_POLICY_VERSION {
            BTreeSet::new()
        } else {
            let mandatory = mandatory_rules.ok_or("severity.mandatory is missing")?;
            validate_rule_list(&mandatory, "severity.mandatory", true)?;
            if !mandatory.iter().any(|rule| rule == "*") {
                return Err(
                    "severity.mandatory must include \"*\" as the default classification"
                        .to_owned(),
                );
            }
            let advisory_values = advisory_rules.ok_or("severity.advisory is missing")?;
            validate_rule_list(&advisory_values, "severity.advisory", false)?;
            let advisory: BTreeSet<String> = advisory_values.into_iter().collect();
            if advisory.is_empty() {
                return Err("severity.advisory must name at least one rule".to_owned());
            }
            if advisory
                .iter()
                .any(|rule| mandatory.iter().any(|item| item == rule))
            {
                return Err("severity rules cannot be both mandatory and advisory".to_owned());
            }
            advisory
        };
        let required_files = required_files.ok_or("project.required_files is missing")?;
        validate_exact_paths(&required_files, "required_files")?;
        let ignored_directories =
            ignored_directories.ok_or("project.ignored_directories is missing")?;
        let ignored_directories = validate_ignored_directories(&ignored_directories)?;
        let ignored_path_prefixes =
            ignored_path_prefixes.ok_or("project.ignored_path_prefixes is missing")?;
        let ignored_path_prefixes = validate_ignored_path_prefixes(&ignored_path_prefixes)?;
        let limits = parse_limits(&limit_values)?;
        for path in exemptions.keys() {
            validate_exact_path(path, "exemptions")?;
        }
        Ok(Self {
            policy_version,
            advisory_rules,
            required_files,
            ignored_directories: ignored_directories.into_iter().collect(),
            ignored_path_prefixes: ignored_path_prefixes.into_iter().collect(),
            exemptions,
            limits,
        })
    }

    pub(crate) fn budget(&self, category: SizeCategory) -> Budget {
        self.limits.get(&category).copied().unwrap_or_default()
    }
}

fn validate_rule_list(values: &[String], key: &str, allow_default: bool) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for rule in values {
        if !seen.insert(rule.as_str()) {
            return Err(format!("{key} contains duplicate rule {rule}"));
        }
        if rule == "*" {
            if !allow_default {
                return Err(format!("{key} cannot contain the default wildcard"));
            }
        } else if !KNOWN_RULES.contains(&rule.as_str()) {
            return Err(format!("{key} contains unknown rule {rule}"));
        } else if !allow_default && rule != "SIZE001" {
            return Err(format!("{key} cannot demote mandatory rule {rule}"));
        }
    }
    Ok(())
}

fn parse_section(line: &str) -> Result<Section, String> {
    match line {
        "[project]" => Ok(Section::Project),
        "[severity]" => Ok(Section::Severity),
        "[limits]" => Ok(Section::Limits),
        "[exemptions]" => Ok(Section::Exemptions),
        _ => Err(format!("unsupported section: {line}")),
    }
}

fn parse_limits(values: &BTreeMap<String, i64>) -> Result<BTreeMap<SizeCategory, Budget>, String> {
    let mut limits = BTreeMap::new();
    for category in [
        SizeCategory::RustProduction,
        SizeCategory::RustTest,
        SizeCategory::CsharpProduction,
        SizeCategory::CsharpTest,
        SizeCategory::Workflow,
        SizeCategory::Markdown,
    ] {
        let key = category.key();
        let preferred = positive_limit(values, &format!("{key}_preferred"))?;
        let maximum = positive_limit(values, &format!("{key}_max"))?;
        if preferred > maximum {
            return Err(format!("{key}_preferred cannot exceed {key}_max"));
        }
        limits.insert(category, Budget { preferred, maximum });
    }
    Ok(limits)
}

fn positive_limit(values: &BTreeMap<String, i64>, key: &str) -> Result<usize, String> {
    let value = values
        .get(key)
        .copied()
        .ok_or_else(|| format!("{key} limit is missing"))?;
    usize::try_from(value)
        .ok()
        .filter(|limit| *limit > 0)
        .ok_or_else(|| format!("{key} limit must be positive"))
}

fn parse_integer(value: &str, key: &str) -> Result<i64, String> {
    value
        .parse::<i64>()
        .map_err(|error| format!("{key} must be an integer: {error}"))
}

fn parse_string_array(value: &str, key: &str) -> Result<Vec<String>, String> {
    let value = value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err(format!("{key} must be an array"));
    }
    let body = &value[1..value.len() - 1];
    let mut values = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in body.char_indices() {
        match character {
            '"' if !escaped => quoted = !quoted,
            ',' if !quoted => {
                values.push(parse_string(&body[start..index], key)?);
                start = index + character.len_utf8();
            }
            _ => {}
        }
        escaped = character == '\\' && !escaped;
        if character != '\\' {
            escaped = false;
        }
    }
    if quoted {
        return Err(format!("{key} contains an unterminated string"));
    }
    let last = body[start..].trim();
    if !last.is_empty() {
        values.push(parse_string(last, key)?);
    }
    Ok(values)
}

fn parse_string(value: &str, key: &str) -> Result<String, String> {
    let value = value.trim();
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(format!("{key} must be a double-quoted string"));
    }
    let mut result = String::new();
    let mut escaped = false;
    for character in value[1..value.len() - 1].chars() {
        if escaped {
            result.push(match character {
                '"' | '\\' => character,
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                _ => return Err(format!("{key} contains an unsupported escape")),
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            result.push(character);
        }
    }
    if escaped {
        return Err(format!("{key} contains an unfinished escape"));
    }
    Ok(result)
}

fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        match character {
            '"' if !escaped => quoted = !quoted,
            '#' if !quoted => return &line[..index],
            _ => {}
        }
        escaped = character == '\\' && !escaped;
        if character != '\\' {
            escaped = false;
        }
    }
    line
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
