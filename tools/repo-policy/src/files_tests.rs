// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)] // Disposable fixture setup must fail the test on error.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{collect, ignored_directory, language_findings, relative_text, size_findings};
use crate::config::Policy;
use crate::license;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("sts2-mcp-policy-{}-{unique}", std::process::id()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // This exact directory was created exclusively by the fixture constructor.
        assert!(fs::remove_dir_all(&self.0).is_ok());
    }
}

fn policy() -> Policy {
    Policy::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../policy.toml")).unwrap()
}

#[test]
fn source_bin_is_collected_but_generated_bin_and_explicit_ignores_stay_ignored() {
    let fixture = Fixture::new();
    for path in [
        "src/bin/entry.rs",
        "crates/adapter/src/bin/entry.rs",
        "crates/adapter/src/bin/support/helper.rs",
        "crates/adapter/src/bin/support/bin/nested.rs",
        "crates/adapter/bin/Debug/generated.rs",
        "bin/generated.rs",
        "target/debug/source.rs",
        "src/bin/vendor/generated.rs",
    ] {
        fixture.write(path, "// SPDX-License-Identifier: MIT\n");
    }
    let mut policy = policy();
    let files = collect(&fixture.0, &policy).unwrap();
    let names: Vec<_> = files
        .iter()
        .map(|path| relative_text(&fixture.0, path))
        .collect();
    assert_eq!(
        names,
        [
            "crates/adapter/src/bin/entry.rs",
            "crates/adapter/src/bin/support/bin/nested.rs",
            "crates/adapter/src/bin/support/helper.rs",
            "src/bin/entry.rs",
        ]
    );
    policy.ignored_path_prefixes.insert(String::from("src/bin"));
    assert!(ignored_directory(std::path::Path::new("src/bin"), &policy));
}

#[test]
fn binary_source_reaches_spdx_language_and_size_enforcement() {
    let fixture = Fixture::new();
    fixture.write("LICENSE", "MIT License\n");
    let production = "crates/adapter/src/bin/oversized.rs";
    fixture.write(production, &"// missing license\n".repeat(401));
    fixture.write("crates/adapter/src/bin/support/forbidden.py", "pass\n");
    fixture.write(
        "crates/adapter/src/bin/support/allowed_tests.rs",
        &format!(
            "// SPDX-License-Identifier: MIT\n{}",
            "// test line\n".repeat(350)
        ),
    );
    let policy = policy();
    let files = collect(&fixture.0, &policy).unwrap();
    let license_findings = license::findings(&fixture.0, &files);
    assert!(
        license_findings
            .iter()
            .any(|finding| finding.rule == "LIC002" && finding.path == production)
    );
    let language = language_findings(&fixture.0, &files);
    assert_eq!(language.len(), 1);
    assert_eq!(language[0].rule, "LANG001");
    let (checked, sizes) = size_findings(&fixture.0, &files, &policy);
    assert_eq!(checked, 2);
    assert_eq!(sizes.len(), 1);
    assert_eq!(sizes[0].path, production);
    assert_eq!(sizes[0].rule, "SIZE001");
    assert!(sizes[0].message.contains("hard maximum 400"));
}
