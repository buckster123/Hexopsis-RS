# text2mesh (Tessera-RS) backlog — slice ledger

A row gets its ✅ when the slice is **merged, deployed, and verified live** — not when tests
pass (house doctrine #5). Notes carry the date and the evidence.

## v1 (image or text → honest GLB)

- [x] **S0 — bootstrap**: repo, CLAUDE.md, design contract, four crates, CI, dual LICENSE, banner — 2026-08-19 local `cargo test --workspace` 57/57; banner job `01M0DESM366XFHP7RMCBTQZTF8`. Field/Krackan still open.
- [x] **S1 — types + store**: MeshJob, errors, SQLite, state machine, watchdog units — 2026-08-19 local green.
- [x] **S2 — mock + faces**: deterministic vertex-colour GLB `degraded`; CLI/MCP/HTTP against mock — 2026-08-19 CLI smoke exit 1 + `status=degraded`. Field import in Blender still open.
- [x] **S3 — honesty surfaces**: system-check, estimate, spend gate — 2026-08-19 `report_complete`/`ready`, no `ok` field.
- [x] **S4 — planner dual-path**: 12-row fixture table; HTTP mock `/v1/jobs` round-trip — 2026-08-19 `planner_row_01..12` + HTTP tests green.
- [x] **S5 — View Contract compiler**: goldens for `evals/text2/prompts.json` + `identity.json` — 2026-08-19 local; `compile --json` emits frozen locks.
- [x] **S6 — gates + Hero-Orbit**: G3/G4 pure; mock T2I views; ungated skip G0–G2 — 2026-08-19 `TEXT2MESH_ALLOW_UNGATED=1` text→degraded GLB. CLIP G0–G2 still `feature_off` without the flag.
- [x] **S7 — Lattice + Cadre**: classifier; Route A refuse-if-absent — 2026-08-19 `box 20x10x5 mm` → `analytic.unavailable` without Cadre. Compose still S14.
- [ ] **S8 — Imaginarium T2I**: estimate-then-fire; no xAI key here — coded 2026-08-19; ledger ✅ after merge + live estimate
- [ ] **S9 — sidecar `meshplane/1`**: handshake, confinement, cancel — coded 2026-08-19; fixture e2e; ledger ✅ after merge
- [ ] **S10 — remote adapters**: Meshy + Tripo fixtures
- [ ] **S11 — export + WebUI**: glTF validate; HTMX; amber degrade
- [ ] **S12 — weights + idle unload**: Hunyuan refuse; Krackan system-check honesty

## Post-v1 parking

- In-process quality DiT from papers (OQ-2/OQ-3 horizon)
- Crates.io + trademark sweep → `tessera` prefix
- Pure-Rust print wrap (OQ-7)
- Native/Slint viewer
- Gaussian/NeRF extras if an engine emits them (success remains GLB)
