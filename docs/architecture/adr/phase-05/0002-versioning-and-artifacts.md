# ADR-0502: Coordinated versioning and artifacts

- Status: Accepted
- Date: 2026-10-28

## Decision

Use coordinated platform releases with Semantic Versioning and immutable
`vMAJOR.MINOR.PATCH` tags. Release artifacts include a source archive, actual
Rust binaries produced by the workspace, Cargo metadata, and SHA-256
checksums. Cargo metadata is an inventory aid, not a complete SBOM.

Container images, signing, registry publication, and provenance attestations
remain deferred until an approved image set, registry, identity, and trust
policy exist.

## Consequences

Consumers receive traceable source and executable artifacts without inventing
images for services that have no Dockerfiles. Component manifest versions may
remain independently useful, but release tags define the coordinated platform
version.
