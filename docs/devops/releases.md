# Releases

## Version policy

The platform uses Semantic Versioning for coordinated releases. A release tag
must match `vMAJOR.MINOR.PATCH`, for example `v0.1.0`. The tag is the release
source of truth; workspace package versions remain component metadata and must
not be silently rewritten by CI.

- Major: incompatible public API, shared contract, or operational behavior.
- Minor: backward-compatible capability.
- Patch: backward-compatible fix or security update.

Event schema and shared infrastructure changes require explicit compatibility
review even when the platform release remains backward compatible.

## Trusted release flow

1. Review and merge the change through the protected default branch.
2. Create and push a version tag, for example `git tag v0.1.0 && git push origin v0.1.0`.
3. The release workflow validates formatting, Clippy, tests, documentation
   tests, and a release build.
4. It packages the source archive, supported Rust binaries, Cargo metadata, and
   SHA-256 checksums.
5. It creates a GitHub Release with generated notes.

The workflow also supports manual packaging of an existing `vMAJOR.MINOR.PATCH`
tag. It does not deploy services or publish container images.

## Integrity and recovery

Checksums are generated after the final files are assembled and are uploaded
with the same release. Cargo metadata records the dependency inventory used by
the Rust package. This is not a complete SBOM or a proof of code safety.

Release publication is not configured for signing, registry publishing, or
provenance attestations because no signing identity, registry, or approved
attestation policy exists. Do not add private keys to the repository. A failed
release must be investigated before retrying; never replace an immutable
artifact under an existing version with different contents.

Release notes are generated from GitHub history and must be reviewed for
accuracy. Known limitations and breaking changes belong in the release notes
before publication.
