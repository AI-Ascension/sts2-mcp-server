# Licensing

Original repository code and documentation are released under MIT. This file is engineering guidance,
not legal advice.

The project license does not grant rights to STS2 binaries, host assemblies, game data, art, music,
trademarks, personal saves, platform components, or external services. No such material belongs in this
target. A local host installation may be inspected by an authorized owner in a disposable environment,
but its files must not be committed, vendored, or packaged here.

New Rust and managed source files carry an `SPDX-License-Identifier: MIT` header. Dependencies and
fixtures require an exact version/source, license review, redistribution status, and notice entry before
they are used. The current workspace packages use only the Rust standard library, so no runtime
dependency notice is required beyond [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).

Do not copy or transliterate a reference implementation. Compatibility requirements must be expressed
as project-owned contracts and tests, with first-party or authorized evidence recorded separately.
