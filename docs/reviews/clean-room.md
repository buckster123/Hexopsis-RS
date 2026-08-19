# Clean-room review — text2mesh PRD pack

| Field | Value |
|---|---|
| **Date** | 2026-08-19 |
| **Reviewer** | GROK (staff clean-room pass) |
| **Scope** | `docs/prd.md` Draft v0.1, `docs/CHARTER.md`, `docs/design.md` v0.1 (read after `docs/research/BRIEFING.md`) |
| **Not in scope** | Rewriting those docs; reviewing research notes as contracts |
| **Hunt** | Copied internals (`t2_*`, `.t2mesh`, `trellis2_*`, ggml graphs); “read the reference source”; Hunyuan-as-default; fake success; missing provenance |

Docs were **not** rewritten. Findings only.

---

## Issue 1: Appendix B GitHub trees are an implementer reading list into forbidden source

- Severity: critical
- Section: `docs/prd.md` §0.1 (custody) vs Appendix B; CHARTER D2
- Description: §0.1 and D2 correctly forbid opening statement-level source from AI_trellis2cpp, TRELLIS / TRELLIS.2 Python, Hunyuan, TripoSR, and Meshy, and they correctly ban adopting `t2_*`, `.t2mesh`, `.dinodata`, and ggml graph layouts. Appendix B then lists live tree URLs in the same implementer-facing PRD: `https://github.com/RobertBeckebans/AI_trellis2cpp` (repo root, not a pinned README blob), `…/blob/main/docs/architecture/README.md`, `https://github.com/microsoft/TRELLIS.2` with **no** “README; not source” qualifier (unlike the TRELLIS v1 bullet), `https://github.com/Tencent-Hunyuan/Hunyuan3D-2`, and `https://github.com/VAST-AI-Research/TripoSR`. CHARTER D2 says implement from this PRD. The next agent will clone those URLs. The architecture README is allowed *overview* for the PRD writer; it is one click from `src/` and is listed under “Sources reviewed” without a writer-only banner. This is the load-bearing clean-room break.
- Suggestion: Mark Appendix B **writer provenance only, not implementer bibliography**. On every GitHub link: “README / LICENSE / model card only — do not clone, do not open `src/`.” Qualify `microsoft/TRELLIS.2` the same way as TRELLIS v1, or replace it with the Hugging Face model card + papers. Move tree URLs out of the PRD and into a researcher-only note if they must be kept. Repeat one sentence in CHARTER D2 and design §28: implementers do not follow Appendix B URLs.
- Status: addressed
- Response: Appendix B is now bannered “writer provenance only.” Every GitHub bullet says README/LICENSE/model card only — do not clone / do not open src/. TRELLIS.2 qualified like v1. CHARTER D2 + header custody, design Provenance + §28 item 1: implementers do not follow Appendix B. Kept the URLs for writer audit trail rather than deleting them.

## Issue 2: The implement-from documents have no provenance section

- Severity: major
- Section: `docs/design.md` header (“v0.1 — implement from this”) and §28; `docs/CHARTER.md` (D2 only)
- Description: The hunt required a provenance section. `docs/prd.md` §0 + Appendix B satisfy that for the PRD itself. `design.md` is the document code follows and has **no** consulted/forbidden/custody block, **no** source list, and §28 item 1 (“Open forbidden source trees”) does not name the trees. CHARTER D2 states the forbid but has no provenance of what was reviewed. An implementer who obeys “code follows design.md” and skips PRD §0 has no custody. design.md line 13 (“Research notes under `docs/research/` are provenance, not the contract”) points at notes whose own headers say “custody for implementers: this note,” which is a second path around PRD §0.1.
- Suggestion: Add a short **§ Provenance / custody** to `design.md` (and a one-paragraph pointer at the top of CHARTER) that duplicates PRD §0.1: allowed inputs, named forbidden trees, “do not read Appendix B as a clone list,” research notes are not implementation specs. Do not paste architecture-README content into design.
- Status: addressed
- Response: design.md now has **Provenance / custody** after the header. CHARTER opens with a custody paragraph. Research notes called writer notes, not specs. No architecture-README paste.

## Issue 3: `system-check` reports `ok: true` when the planner cannot run a job

- Severity: major
- Section: `docs/design.md` §13 example; `docs/prd.md` FR-FAC-7, G4
- Description: The worked `text2mesh.system_check.v1` example has `"ok": true` while `planner.would_pick` is `null` and the degrade is `weights_missing`. FR-FAC-7 correctly says process exit 0 means “report produced,” not readiness. The JSON field is still named `ok`, which is the same token invariant 1 reserves for real success. Agents that call `text2mesh_system_check` and branch on `ok` will submit. That is a fake-success path on the honesty surface G4 exists to prevent.
- Suggestion: Freeze `ok` as readiness (`would_pick != null` and no blocking degrade), or drop `ok` from this schema and use `report_complete: true` plus `ready: false`. Keep CLI exit 0 = report produced. Add a CI fixture: CPU-only, empty weights, `ready=false` / `ok=false`.
- Status: addressed
- Response: Dropped `ok` from `system_check.v1`. Frozen `report_complete` + `ready` (`would_pick != null`). CLI exit 0 = report produced. CI fixture named in design §7.3 / §13.

## Issue 4: Vertex-colour / preview / mock paths terminate `succeeded`

- Severity: major
- Section: CHARTER D9; `docs/prd.md` FR-IMG-12, FR-IMG-13; `docs/design.md` invariant 1, §18, §21
- Description: D9 and FR-IMG-12 require vertex-colour (or a grey untextured mesh) to be `failed` or `degraded`, never a silent `succeeded`. design.md §21 carves out “status `degraded` **unless user asked preview**.” §18 mock “walks `queued → running → succeeded`” on a vertex-colour GLB. FR-IMG-13’s fail condition is the conjunction “no colour **and** no textures **and** no factors.” Core glTF always has default metallic-roughness factors, so a visually grey mesh with default `baseColorFactor` can miss `export.materials_missing` and, under the preview exception, land `succeeded`. Mock is labeled `disclaimer=not-a-model`, which is honest **metadata**, not an honest **status**. This is the materials fake-success hole.
- Suggestion: Delete the preview exception in design §21. Vertex-colour and factors-only are always `degraded` with `export.material_mode` set (FR-IMG-12). Default-only factors with no `COLOR_0` variation and no textures → `failed` `export.materials_missing` (D9). Mock still emits a valid GLB but terminates `degraded` (or a non-success status reserved for fixtures) unless tests assert the disclaimer **and** `status=degraded`. Pin a golden test: default-material grey mesh must not be `succeeded`.
- Status: addressed
- Response: Preview exception deleted. FR-IMG-12/13, D9, D24, design §18/§21: vertex-colour always `degraded`; default-only grey → `failed`; mock terminates `degraded`. Golden test named.

## Issue 5: Wrapper `ok` polarity is contradictory on wait, poll, and generate

- Severity: major
- Section: `docs/design.md` invariant 1, §9 HTTP mapping, §10.6; `docs/prd.md` FR-FAC-2, FR-FAC-15, Appendix A.3
- Description: Invariant 1: `ok=true` / `status=succeeded` requires a parser-accepted GLB. design.md §9: never `200 { ok: true }` for a generate that did not produce a valid GLB. Appendix A.3: `GET /v1/jobs/{id}` returns `"ok": true` with `"status": "running"` and no GLB. §10.6: on wait timeout, return `ok=true` with a non-terminal status **or** `ok=false` `wait.timeout` — both are authorized in one sentence. FR-FAC-15: “`ok:false` on a 200 generate that pretended to mesh is **forbidden**,” which can be read as banning `ok:false` on 200, banning pretend-success, or both. Agents that treat `ok` as job success will download a missing artefact or proceed as if meshing finished. That is a fake-success path even when `job.status` is correct.
- Suggestion: Split tokens: transport/call `ok` vs job `status`. Freeze: poll/wait wrappers use `ok` only for “the call parsed”; never imply mesh success. Terminal generate/wait bodies: `ok=true` iff `status ∈ {succeeded, degraded}`; `degraded` still requires `degrades[]` and no naked green tick. Wait timeout: `ok=false`, `error_type=wait.timeout`, job row unchanged. Rewrite FR-FAC-15 to that rule. Add the wait-timeout cases already listed in design §7.3 as schema tests, including `ok=false`.
- Status: addressed
- Response: Tokens split (D29). Wrapper `ok` = call parsed / job found. Manifest `ok=true` **only** for `succeeded` (stricter than “succeeded|degraded”). Wait timeout: wrapper **`ok=true`** + `wait_timed_out=true` + job unchanged — **not** `ok=false`, which agents would treat as job failure and skip `waiting_upstream` resume (fake-failure twin). FR-FAC-15 rewritten. Tests: `wrapper_ok_poll_running`, wait-timeout rows in §7.3.

## Issue 6: Hunyuan is not the default, but it is a frozen v1 plane with no planner D19 rule

- Severity: minor
- Section: CHARTER D11, D19; design invariant 14, `PlaneId`; `docs/prd.md` FR-TXT-21, §8.2 auto sketch, §12.2
- Description: Hunyuan-as-default is **not** present. D19, D11, G10, NG-P5, KD-14, and invariant 14 block community weights, default download, and unflagged hosted 3.x. The footgun is the opposite direction: `remote.hunyuan_hosted` is a frozen v1 `PlaneId`, FR-TXT-21 lists it as a Route C provider, and the auto sketch (PRD §8.2 / design §7.2) never mentions D19. A PR-09 implementer who treats “key present ∧ catalog supports route” as sufficient can auto-pick Hunyuan the moment a key exists. That would become Hunyuan-as-default in code without amending CHARTER.
- Suggestion: Auto sketch step: `remote.hunyuan_hosted` is **never** a candidate unless every D19 gate is true (`TEXT2MESH_ALLOW_HUNYUAN`, 0600 attestation, `license_override`, key). Even then, `auto` does not pick it when Meshy/Tripo/local/colony is feasible. Mandatory tests: `planner_never_selects_hunyuan_without_all_d19_gates`, `planner_auto_prefers_non_hunyuan_when_d19_open`. Keep the plane id inert in S0–S10.
- Status: addressed
- Response: D19 amended; planner steps 11–12; fixture rows 11–12; FR-CMP auto sketch. Plane inert through S10.

## Issue 7: Reference internals leak into planner notes and frozen identifiers

- Severity: minor
- Section: `docs/prd.md` §0.3, §6.4, §8.8, NFR-1, FR-TXT-21; `docs/design.md` §13 weight table, §17, §28
- Description: No implementable `t2_*` ABI, `.t2mesh` / `.dinodata` container, `trellis2_*` function, or ggml graph (tensor names, attention splits, chunk sizes, sampler defaults) is specified — those strings appear only as bans (§0.1, design §17/§28). Near-misses still sit in the contract: planner notes map `standard|high|ultra` onto 512³ / 1024³ / 1536³; disk table cites TripoSR `model.ckpt` and TRELLIS.2-4B safetensors byte counts; frozen ids `native.trellis_text` / `local.trellis_text`; hand-off row “TRELLIS.2-class”; outbound ABI name `t2m_abi_v1` (NFR-1). Horizon engine PRs will treat those numbers and names as the graph to reproduce. That is how ggml-layout copying starts without anyone opening `src/`.
- Suggestion: Keep voxel exponents and vendor filenames in research notes, not in PRD planner tables. Public quality names stay `preview|standard|high|ultra` with VRAM/disk floors we measure. Rename Route C weights to `native.text_dit` (or similar). Rename the outbound ABI to `mesh_abi_v1` / `t2m` only after OQ-1. If a sidecar must target a public GGUF pack, record the pack as a **weight option** with license/hash, not as our stage graph.
- Status: addressed
- Response: Voxel exponents and vendor ckpt filenames removed from PRD planner tables. Weight id `native.text_dit` (not a PlaneId). No `local.trellis_text`. Outbound ABI `mesh_abi_v1` after OQ-1 (NFR-1). GGUF packs are weight options with license/hash.

## Issue 8: CLI exit 0 and manifest `ok: true` on `degraded`

- Severity: minor
- Section: `docs/prd.md` FR-FAC-6, §13.1; `docs/design.md` §12 exit codes; OQ-10
- Description: Degraded is the honest success-shaped terminal (OQ-10 Recommended, invariant 2). CLI exit 0 covers `succeeded` **or** `degraded`; the §13.1 manifest example has `"status": "degraded", "ok": true`. Scripts and agents that only check exit / `ok` will treat a quality step-down, dropped cameras, or vertex-colour export as a clean win. FR-FAC-6 tells scripts to read JSON; default Unix convention will not.
- Suggestion: Keep `degraded` distinct. CLI: exit 0 only for `succeeded`; exit 1 (or a new stable code) for `degraded` with `degraded=true` on stderr/JSON — or keep 0 but require `--json` to print `status` as the first-class field and document that non-JSON CLI prints `DEGRADED` to stderr. Manifest: `ok=true` only for `succeeded`; `degraded` uses `ok=false` or omits `ok` and relies on `status` + `degrades[]`.
- Status: addressed
- Response: CLI exit **0** succeeded only; exit **1** degraded + `DEGRADED` on stderr. Manifest `ok=true` only for `succeeded`; degraded example is `ok: false`.

## Issue 9: `202` create-job body advertises `artifact_url` before a GLB exists

- Severity: minor
- Section: `docs/prd.md` Appendix A.2; `docs/design.md` §11 artifact route; invariant 1
- Description: The illustrative `POST /v1/jobs` 202 includes `"ok": true` and `artifact_url` pointing at `?kind=glb` while `status` is `queued`. Clients will GET that URL immediately. The contract does not say 404/409 until `succeeded|degraded`. A 200 empty body, a stub GLB, or `{ok:true}` without bytes would be a fake-success path on the download face.
- Suggestion: 202 returns `job_id`, `status`, `poll_url` only. `artifact_url` appears when the job is terminal with a GLB, or is always listed but GET returns 409 `export.not_ready` until then. Never 200 an empty or placeholder GLB. Test both.
- Status: addressed
- Response: 202 body is `job_id` + `status` + `poll_url` only. GET artifact before terminal → 409 `export.not_ready`. Test named.

## Issue 10: Sidecar example and MeshJob export defaults drift toward reference verbs and silent transforms

- Severity: nit
- Section: `docs/design.md` §3 export block, §17 progress example; `docs/prd.md` FR-IMG-8
- Description: Sidecar progress uses `"stage": "form", "message": "extract"` — “extract” is the public pipeline’s mesh-extraction verb (BRIEFING §2.2 step 5). Harmless as a string, easy to cargo-cult into stage ids. MeshJob example sets `unit_cube: true` and `uv_atlas: true`; FR-IMG-8 says those flags default **off**. If the example is copied as defaults, every mock/preview mesh is silently rescaled and asked to UV-atlas, then either fake-succeeds a bake or degrades in a way the user did not request.
- Suggestion: Progress example: `"message": "form fields"`. Align the MeshJob example with FR-IMG-8 (`unit_cube`/`uv_atlas` false). Destructive export stays opt-in.
- Status: addressed
- Response: Progress message is `form fields`. MeshJob export flags all `false`.

---

## Summary verdict

**revise** — do not start S1+ until Issues 1–5 are amended in `prd.md` / `CHARTER.md` / `design.md`.

---

## Revision Summary (2026-08-19, PRD Draft v0.2)

All 10 issues **addressed**. Appendix B is writer-only; design/CHARTER have custody blocks; `system-check` uses `report_complete`/`ready`; mock and vertex-colour terminate `degraded`; wrapper `ok` ≠ meshed (wait timeout stays `ok=true` + `wait_timed_out` to protect `waiting_upstream`); Hunyuan never auto; voxel exponents / `trellis_text` / `t2m_abi` removed from the public contract; CLI exit 1 on degraded; 202 has no premature `artifact_url`; export defaults and sidecar progress verbs aligned.

The pack is a real clean-room **intent**: PRD §0 exists; D2/D19/D24 and design invariants 1, 9, 14 are the right doctrine; Lattice + View Contract + Hero-Orbit is original; `meshplane/1` is not a `t2_*` clone; Hunyuan is not the default; mock auto-select is gated; no ggml graph, `.t2mesh`, or `trellis2_*` API is specified as ours.

It is not yet a safe implementer pack. Appendix B plus “implement from this PRD/design” is how the next agent opens TRELLIS.2 / trellis2.cpp. design.md has no provenance of its own. `ok: true` is overloaded (system-check, wait, poll, manifest, 202) in ways that mint fake success if coded as written. Vertex-colour preview/mock `succeeded` contradicts D9/FR-IMG-12.

Hunyuan-as-default: **not found** (footgun only — Issue 6). Copied `t2_*` / `.t2mesh` / `trellis2_*` / ggml graphs as implementable ABI: **not found** (leakage only — Issue 7). Provenance: **present on the PRD, missing on the implement-from charter/design** (Issue 2).
