// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const SUPPORTED_POLICY_VERSION: i64 = 1;

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

        if version != Some(SUPPORTED_POLICY_VERSION) {
            return Err(format!(
                "policy_version must be {SUPPORTED_POLICY_VERSION}, found {version:?}"
            ));
        }
        let required_files = required_files.ok_or("project.required_files is missing")?;
        let ignored_directories =
            ignored_directories.ok_or("project.ignored_directories is missing")?;
        let ignored_path_prefixes =
            ignored_path_prefixes.ok_or("project.ignored_path_prefixes is missing")?;
        let limits = parse_limits(&limit_values)?;
        Ok(Self {
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

fn parse_section(line: &str) -> Result<Section, String> {
    match line {
        "[project]" => Ok(Section::Project),
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
mod tests {
    use super::{Policy, SizeCategory};

    const LIMITS: &str = "rust_production_preferred = 10\nrust_production_max = 20\n\
rust_test_preferred = 11\nrust_test_max = 21\ncsharp_production_preferred = 12\n\
csharp_production_max = 22\ncsharp_test_preferred = 13\ncsharp_test_max = 23\n\
workflow_preferred = 14\nworkflow_max = 24\nmarkdown_preferred = 15\nmarkdown_max = 25";

    #[test]
    fn parses_complete_policy() -> Result<(), String> {
        let text = format!(
            "policy_version = 1\n[project]\nrequired_files = [\"README.md\"]\n\
             ignored_directories = [\"target\"]\nignored_path_prefixes = []\n\
             [limits]\n{LIMITS}\n[exemptions]\n"
        );
        let policy = Policy::parse(&text)?;
        assert_eq!(policy.required_files, ["README.md"]);
        assert_eq!(policy.budget(SizeCategory::RustProduction).maximum, 20);
        Ok(())
    }

    #[test]
    fn rejects_inverted_budget() {
        let text = format!(
            "policy_version = 1\n[project]\nrequired_files = []\n\
             ignored_directories = []\nignored_path_prefixes = []\n[limits]\n{}\
             \n[exemptions]\n",
            LIMITS.replace("rust_production_max = 20", "rust_production_max = 5")
        );
        assert!(Policy::parse(&text).is_err());
    }
}
