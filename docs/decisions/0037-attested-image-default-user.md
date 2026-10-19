# 0037 — Explicit image-default user declaration

Status: accepted for local provisioning transport; real-template proof pending.

Cloud-Init 25.1.4 flags Proxmox's scalar user form as deprecated. The strict
first-boot requirement does not accept that recoverable error. ADR 0033's offline
reference experiment established an omitted-user alternative only when the image
default is known and matches the requested account.

Add optional imageDefaultUser to the existing server mapping. Its name and
revision must match guestUser/templateVersion; it declares a non-root default
account with a locked password and no inherited authorized keys. Treat it as an
operator declaration, never as observed evidence. It is included in plan hash
material. Omitting it preserves legacy serialized mapping/hash inputs.

For opted-in mappings, configuration deletes ciuser instead of setting it.
Independent observation requires its absence. Generated seed inspection permits
omitted user only in this mode, retaining strict users/default, exact key material,
network and revision checks. Explicit null and other user forms are rejected.
Existing mappings keep explicit ciuser and named/scalar classification.

Generated HTTPS fixtures cover both modes and rejection paths; no real provider
was contacted. No new dependency, database migration or generic provisioner is
needed. This does not finish attached-seed regeneration, boot, guest readiness,
production worker/API integration or mobile feasibility.
