# Security incident guidance

1. Disable the affected API key or identity immediately.
2. Preserve correlation IDs, safe audit events, deployment configuration, and
   relevant timestamps without copying secrets or message bodies.
3. Determine affected tenants, resources, credentials, and delivery activity.
4. Rotate compromised credentials and invalidate old verification material.
5. Apply an abuse or delivery kill switch when message sending may be affected.
6. Restore service only after authorization, secret access, and audit paths are
   verified.
7. Record the timeline, impact, containment, recovery, and follow-up controls.

Production incidents require an approved identity provider, durable audit sink,
secret manager, and key-rotation runbook; the current local adapter does not
claim those guarantees.
