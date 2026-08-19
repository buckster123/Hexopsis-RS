# text2mesh PRD write-up — 2026-08-19 (v0.4)

**Implemented.** v1 S0–S12 is on `main` (2026-08-19). This note is the PRD-write provenance, not the runbook — see `docs/STATUS.md` and `docs/USER.md`.

**v0.4 (2026-08-19).** André locked house OQ-1..7 in place (no pack rewrite): (1) garden name **Tessera-RS**, crates/binaries stay `text2mesh` until a crates.io + trademark sweep PR, Cerebro id **TESSERA** / tags `project:text2mesh`; (2) sidecar v1 + independent Rust from papers as horizon; (3) hybrid runtime; (4) extras-allowed — GLB+PBR still defines success; Gaussian/NeRF optional extra artefacts if an engine emits them, not a second metric, not first-class DCC; (5) 6 cameras `cardinal4_hero_top`; (6) bind `127.0.0.1:8796`; (7) defer print wrap. OQ-8/9/10 remain open. CHARTER D1/D9/D16/D21/D27/D28 amended; D21 stays. Wait clocks, gates, and prompt suffixes unchanged.

Clean-room pack after verify leftovers. No forbidden source opened. CHARTER D* win; freeze conflicts are bugs (D15). Draft **v0.3** addressed `docs/reviews/verify.md` Issues 1–7 in place (no pack rewrite). Draft **v0.4** locks OQ-1..7 only.

## What landed (v0.3)

| File | Role |
|---|---|
| `docs/prd.md` | **Draft v0.3** — one wait clock; one gate contract; frozen prompt strings; watchdog pid-live; no CUSTOM env; confirm in FR-FAC-13; eval files in PR plan |
| `docs/CHARTER.md` | D1–D30 unchanged; dated v0.3 freeze-leftover amendment |
| `docs/design.md` | **v0.3** — `prompt_suffix` on every preset row; class lock + negatives arrays; G0 text = identity_phrase else normalized; invariant 11 = G1–G4; schema-drift wait default |

Working name **`text2mesh`**. Garden name **Tessera-RS** (OQ-1 locked). Cerebro id **TESSERA**.

## v0.3 freeze leftovers (verify Issues 1–7)

1. **Wait default 1800.** MCP `timeout_s`, CLI `--timeout-s`, Route B `max_wall_s`: min 30, default 1800, max 86400. PR-01b asserts generated MCP default equals CLI default.
2. **One gate contract.** G0/G1/G4 vs `canonical_view_id` (preview `cardinal4` has no `hero`). FACE/BACK pair only. G0 text is a single string. Required G1–G4 miss after retries → do not call Image3dPlane.
3. **S5 goldens.** Frozen suffixes + class locks + Janus/other negatives JSON arrays. One expected JSON per prompt.
4. **Observability.** Emit progress when the engine does; director parent-heartbeat; watchdog must not treat missing lines as crash if pid is live. ≤5 s is a should cadence.
5. **No custom plane.** `TEXT2MESH_CUSTOM_*` deleted. `remote-http` = Meshy/Tripo/colony adapters (inert without keys).
6. **Confirm on the copied route list.** `POST /v1/jobs/{id}/confirm`; artifact `?kind=` includes `contract`.
7. **Fixture files in the PR plan.** PR-10 `prompts.json` + `identity.json`; PR-13 `classify.json` + `species.txt`; PR-11 may add `scores/`; PR-21 uses the same files.

## Honesty freeze (unchanged from v0.2)

- Manifest `ok=true` ⇔ `succeeded`. Degraded: `ok=false`, CLI **exit 1**.
- `system-check`: `report_complete` + `ready` (not `ok`).
- Mock + vertex-colour → `degraded`. Grey default material → `failed`.
- 202 has no `artifact_url`; GET artefact 409 until terminal.

## Field machine

Krackan: 512 MiB shared AMD iGPU. Local **standard** needs 24 GB VRAM + ≥16 GiB disk. v1 demo = mock + remote **or** stated degrade. Stages are progress names; no in-process DiT in S0–S11.

## House OQs (locked 2026-08-19)

1 Tessera-RS · 2 sidecar+horizon · 3 hybrid runtime · 4 extras-allowed (GLB success) · 5 six cameras · 6 port 8796 · 7 defer GPL wrap.

S0 scaffold can proceed. Independent re-verify of v0.4 before treating the pack as implementation-ready.
