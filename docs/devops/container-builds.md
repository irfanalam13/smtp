# Container validation

The current repository has no service Dockerfiles or application container
images. The local [Compose file](../../compose.yaml) provisions PostgreSQL
only, so Phase 05 validates Compose syntax and prevents tracked environment
files instead of creating placeholder production images.

```bash
docker compose -f compose.yaml config --quiet
```

When a service receives an approved Dockerfile, add a focused build and smoke
test to [container.yml](../../.github/workflows/container.yml). The image must
use a maintained pinned base, exclude `.env` and credentials, run as a
non-root user where practical, and include OCI source/revision/version labels.
The image must be scanned before publication. No registry or container
publishing credentials are configured today.

Do not treat a successful Compose configuration check as an image build or
security scan.
