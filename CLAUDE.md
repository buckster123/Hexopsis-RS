# text2mesh (Hexopsis-RS) — Agent & Developer Guide

> Image or text → glTF 2.0 GLB with core PBR, local **or** remote, one `MeshJob`.
> Four-face workspace: core lib + MCP + CLI + HTTP.
> Standalone-first. ApexOS is a consumer, never the owner. Garden name **Hexopsis-RS**; crate prefix `text2mesh` until a sweep PR.

Bootstrapped 2026-08-19. House conventions come from `~/Projects/Launchpad-RS/`
— load a doc from there when you need the detail behind a rule below.

**Read `docs/CHARTER.md` before any non-trivial change — its decisions log (D1–Dn) is
binding.** Amend it with a dated entry when a decision changes, never silently. Where the
charter and this file disagree, the charter wins.

Siblings: Cadre-RS (analytic CAD), Imaginarium-RS (T2I keys), OmniOcular-RS (visualize 3D).
Clean-room: do **not** open AI_trellis2cpp / TRELLIS / Hunyuan / TripoSR source.

---

## What this is

A Rust product that turns a still image or a text prompt into a portable glTF 2.0 GLB.
The text path is **Lattice Router + typed View Contract + Hero-Orbit**, then the same
image-to-mesh plane. Dual compute: `LocalPlane` + `RemotePlane`; `auto` is a pure planner.
v1 (S0–S12) is **implemented** on `main` (`docs/STATUS.md`). Quality is a user sidecar
(`meshplane/1`) or a remote (Meshy/Tripo/colony), not an in-process DiT.

```
crates/
  text2mesh/         # core — ALL logic: types, store, planner, mock, director
  text2mesh-mcp/     # MCP stdio (agent face)
  text2mesh-cli/     # clap CLI (human/ops face)
  text2mesh-api/     # axum REST + optional HTMX
docs/design.md       # THE contract
BACKLOG.md           # slice ledger
```

---

## Locked decisions

The load-bearing summary; **`docs/CHARTER.md` D1–D30 is the binding long form.**
**Locked means locked.**

- **Language**: Rust — one Cargo workspace
- **License**: MIT OR Apache-2.0
- **MCP**: hand-rolled newline-delimited JSON-RPC over stdio, protocol `2024-11-05`, no SDK
- **Storage**: SQLite `jobs.sqlite` + `jobs/<id>/` artefacts
- **HTTP**: `reqwest` (rustls) out, `axum` in; bind `127.0.0.1:8796`
- **CI from commit 0**: fmt `--check` + clippy `-D warnings` + test + build
- **Nano-first**: default build has no ggml/CUDA/ONNX; job timeouts never < 30 s
- **Honesty**: mock/vertex-colour → `degraded`; grey → `failed`; no orphan `pending`
- **Spend gated**; no `XAI_API_KEY` in this process; Hunyuan never auto
- **Cerebro** agent id `HEXOPSIS`, tags `project:text2mesh`

---

## The playbook

Full rationale: `~/Projects/Launchpad-RS/docs/house-doctrine.md`.

1. **Contract first.** `docs/design.md` before code. Docs travel with code.
2. **Slices, not marathons.** One branch off `origin/main`. Never stacked PR bases.
3. **Honesty invariants.** No fake success. Stated degrades. Failures carry the real reason.
4. **Pure-fn tests.** Planner, classifier, compiler, gates, parsers. Handlers are glue.
5. **Field truth beats green CI.** Ledger ✅ only after a live job or stated degrade.
6. **Secrets hygiene.** Lengths/heads only. No credentials in CLAUDE.md.
7. **Cerebro is the thread.** `session_recall` in, `session_save` out. `agent_id="HEXOPSIS"`.
8. **Spend is gated.** Paid POST never auto-fires.
9. **Cost the failure.** Paid remote poll expiry → `waiting_upstream`, not silent fail.

---

## Git discipline

- Feature branch off freshly-fetched `origin/main`: `feat/…`, `fix/…`, `chore/…`, `docs/…`.
- Ship via PR. Merge when André says so **or** when he already granted merge-on-green.
- Commit format: imperative, lowercase. End with `Co-Authored-By` trailer.
- Never amend a pushed commit. Never force-push.
- Push after every commit.

---

## Cerebro session protocol

All Cerebro MCP calls use `agent_id="HEXOPSIS"` for this product (GROK may also write
`project:text2mesh` tags). Full menu: `~/Projects/Launchpad-RS/docs/cerebro-protocol.md`.

**START:** `session_recall(query="text2mesh Hexopsis-RS build status", agent_id="HEXOPSIS")`
**END:** `session_save` + `remember` for decisions.

---

## Dev commands

```bash
cargo test --workspace
cargo fmt --all && cargo clippy --workspace -- -D warnings
cargo build --release --workspace

TEXT2MESH_ALLOW_MOCK=1 cargo run -p text2mesh-cli -- generate --image ./fixtures/dot.png --compute local --provider local.mock --json
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}' | cargo run -p text2mesh-mcp
cargo run -p text2mesh-cli -- serve   # 127.0.0.1:8796
```

---

## Gotchas

Project invariants live in **`docs/gotchas.md`**. Grep it before modifying a subsystem.

- **MCP stdout is sacred.** All `tracing` → **stderr**.
- **Read the pinned crate's docs for the exact version.**
- **Do not clone PRD Appendix B URLs.** Writer provenance only.
- **Mock is never `succeeded`.** Vertex-colour GLB → `degraded`.

---

## Docs

| File | Load when |
|------|-----------|
| `docs/STATUS.md` | v1 close-out / field truth |
| `docs/USER.md` | How to run the product |
| `docs/CHARTER.md` | Binding D1–Dn |
| `docs/design.md` | Wire / job / MCP / HTTP |
| `docs/prd.md` | Product intent |
| `docs/gotchas.md` | Any subsystem change |
| `BACKLOG.md` | Slice ledger |

Keep this file under ~250 lines. Fat goes to `docs/`.
