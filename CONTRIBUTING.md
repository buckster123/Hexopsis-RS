# Contributing

Garden name **Tessera-RS**. Working crates stay `text2mesh` until a rename sweep (CHARTER D1).

1. Read [`docs/CHARTER.md`](docs/CHARTER.md) — D1–D30 bind. Amend with a dated entry; never silently.
2. [`docs/design.md`](docs/design.md) is the wire. Behaviour changes update it in the **same commit**.
3. [`docs/gotchas.md`](docs/gotchas.md) before touching a subsystem.
4. Playbook: [`CLAUDE.md`](CLAUDE.md).

## Clean-room

Implement from the charter, the design, Khronos glTF 2.0, the GGUF spec, and crates.io. Do **not** open or paraphrase statement-level source from `AI_trellis2cpp`, Microsoft TRELLIS / TRELLIS.2 Python, Hunyuan3D, TripoSR, or Meshy trees. Do not follow PRD Appendix B URLs (writer provenance only).

## PRs

One branch off fresh `origin/main`. `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`. No stacked feature bases. No `XAI_API_KEY` in this process. Mock/vertex-colour is `degraded`, never a fake success.
