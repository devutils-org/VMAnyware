# VMAnyware — instructions for coding agents

Read `STATE.md` first, then the relevant section of `VMAnyware.md` and
`docs/phase-0.md`. The roadmap remains the product specification.

- Write code, comments and repository documents in English. Conversation may be French.
- Work sequentially: one agent, one small vertical slice, one reviewable diff.
  Do not start parallel agents or worktrees unless the user requests them.
- Before a slice, state its scope and acceptance criteria. Stop expanding scope
  when those criteria are met; record the next task in `STATE.md`.
- Keep the workspace small. Add crates, runtime dependencies and abstractions
  only when a working use case needs them. Record consequential choices in
  `docs/decisions/`.
- Select tests and checks from the current slice's diff. Run the smallest checks
  that cover changed behavior and affected callers; do not rerun unrelated suites
  or previously passing checks without new changes or evidence of risk. In a dirty
  workspace, distinguish this slice's changes from earlier verified changes.
  Documentation-only edits need diff/format review, not application tests.
  Broaden verification only for demonstrated cross-component impact, shared
  contracts, dependency/build changes or unresolved failures; state the reason.
  `make check` is the complete local check, not the default for every slice.
  Use targeted test filters first; `make check-server`, `make check-client` and
  `make check-rust` are available when the diff warrants their broader scope.
- Never mark a roadmap item complete without evidence. Distinguish source
  preparation, compilation, application launch and real infrastructure testing.
- Do not inspect local credential files, commit secrets, disable TLS/SSH host
  verification, or use private SSH keys in examples.
- No VM mutations, production infrastructure changes or credential changes
  without explicit user approval of the concrete target and operation.
- Keep real Proxmox tests opt-in and confined to an explicitly configured
  disposable target. Default checks must work without Proxmox or credentials.
- Finish by reporting changes, actual verification, remaining limitations and
  one next task. Update `STATE.md` so a new session can resume cheaply.

Borrow conventions from sibling projects, not their product-specific machinery.
Do not modify sibling repositories.
