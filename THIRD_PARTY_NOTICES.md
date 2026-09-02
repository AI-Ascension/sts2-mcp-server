# Third-party notices

This foundation snapshot contains no copied third-party implementation source, proprietary game file,
host assembly, save, credential, or runtime dependency. The policy tool uses only the Rust standard
library. The MCP artifact conformance test uses the locked `jsonschema` 0.48.5 development dependency
from crates.io (MIT), with default features disabled, only to validate the copied Draft 2020-12 schema and
fixtures; it is not a runtime package dependency.

When dependencies or fixtures are added, record their exact version, source, license, redistribution
status, and review owner here or in a generated notice tied to the committed lockfile. A dependency with
unknown provenance or incompatible terms blocks release until resolved.
