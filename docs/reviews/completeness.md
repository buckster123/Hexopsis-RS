# text2mesh — completeness review

| Field | Value |
|---|---|
| **Status** | addressed (v0.2) |
| **Date** | 2026-08-19 |
| **Reviewer** | GROK (staff completeness, adversarial) |
| **Scope** | `docs/prd.md` Draft v0.1 · `docs/CHARTER.md` · `docs/design.md` v0.1 |
| **Docs rewritten?** | No |
| **Verdict** | not-ready |

Checklist (mandatory pack). Pass = section exists and is specific enough to implement without inventing product behaviour.

| Item | Result |
|---|---|
| Mandatory PRD sections (writer §0–20 + App A/B) | **Pass** (Risks heading absent — nit) |
| Key Decisions numbered + rationale | **Pass** (KD-1..30; CHARTER D1–D27 binds a subset) |
| PR Plan ordered and mergeable off `main` | **Partial** (order is real; coverage holes — Issue 10) |
| text2 invention: typed fields, not slogans | **Partial** (schema is typed; compiler/gates still guess — Issue 5) |
| Dual compute + Auto planner | **Partial** (trait + 9-step sketch exist; tie-break + I/O table missing — Issue 8) |
| MCP / CLI / HTTP parity | **Partial** (tool/route lists exist; ingest/confirm/export/native flags diverge — Issue 6–7) |
| Job state machine never-`pending` | **Pass** with TTL holes (Issue 11) |
| Nano feature gates | **Pass** |
| Sibling compose | **Partial** (policy locked; HTTP wire not typed — Issue 9) |
| Measurable success metrics | **Pass** (M1–M10; M3 numeric) |
| Alternatives (≥3, why not) | **Pass** (§16.1–16.4) |

Freeze conflicts between PRD and design are treated as completeness failures: implementers do not have one contract.

---

## Issue 1: Export flag defaults disagree (PRD vs design)

- Severity: major
- Section: PRD §6.2 FR-IMG-8 · design §3 MeshJob `export`
- Description: PRD requires `keep_largest_component`, `force_opaque`, `unit_cube`, `uv_atlas` all default **off** (destructive cleanup opt-in). Design’s normative MeshJob example freezes `unit_cube: true` and `uv_atlas: true`. CHARTER D15 says design is the wire contract; D9/FR-IMG-8 say opt-in. An S1 serde `#[serde(default)]` implementer will pick one and silently violate the other. Faces (CLI/MCP/HTTP) also never expose the export object (see Issue 6), so the default *is* the product.
- Suggestion: Pick one default set in design §3 and PRD FR-IMG-8 in the same sentence. Recommended: keep PRD opt-in (all false) so mock/preview meshes are not auto-rescaled; `uv_atlas` remains a preference the exporter honours when the engine can bake, still default false. Add the export object to the shared submit schema.
- Status: addressed
- Response: All export flags default **false** in design MeshJob + JobSubmit. Export object on the shared submit schema (CLI/MCP/HTTP).

---

## Issue 2: Image size limit is two different limits

- Severity: major
- Section: PRD FR-IMG-4 · design §20 Image preprocess
- Description: PRD: max **32 MiB compressed**, 4096 px long edge after decode; larger → `spec.rejected`; do not silently downscale without `image.scaled=true`. Design: reject if **pixels > 32 MiB uncompressed** or long edge > 4096, with Recommended reject-over-4096 and no silent clamp. A 4096² RGBA buffer is 64 MiB uncompressed; a 10 MB JPEG of that image passes PRD and fails design. `image.scaled` is therefore undefined (when would it ever be true if both reject?).
- Suggestion: Freeze one triple in design §20: compressed cap, uncompressed/pixel cap, long-edge cap, and whether scale-down exists. Recommended: 32 MiB **compressed** upload, 4096 long edge, uncompressed decode cap (e.g. 64 MiB or 4096²×4), **no** auto-scale in v1 (`image.scaled` only if a later amendment adds it). Delete the contradictory clause.
- Status: addressed
- Response: Frozen: 32 MiB compressed, 4096 long edge, 64 MiB uncompressed. No auto-scale in v1. FR-IMG-4 matches design §20.

---

## Issue 3: Job store layout is three layouts

- Severity: major
- Section: PRD NFR-9 · PRD §11.4 · design §16
- Description: Three incompatible trees:
  1. NFR-9 default store = `~/.local/share/text2mesh/jobs`
  2. PRD §11.4: `$TEXT2MESH_STORE/jobs.sqlite` and `$TEXT2MESH_STORE/<job_id>/…`
  3. design §16: `$TEXT2MESH_STORE` default `$XDG_DATA_HOME/text2mesh`, sqlite at store root, artefacts under `jobs/<job_id>/`
  Extra files disagree too (`artifact.glb.sha256` in PRD only; `log.stderr.txt` / `artifact.step` in design only). MCP `text2mesh_artifact` returns a path; if faces and the sidecar disagree on the directory, Colony/HTTP mock and local jobs diverge.
- Suggestion: Freeze design §16 as the only tree. Recommended: store root `~/.local/share/text2mesh`, sqlite `jobs.sqlite` at root, artefacts `jobs/<job_id>/`. Amend NFR-9 to name the **root**, not a `jobs` suffix. List the full per-job file set once (glb, glb sha, step, log, views, scratch).
- Status: addressed
- Response: NFR-9 + PRD §11.4 now match design §16 (root, sqlite at root, `jobs/<id>/`, full file set including sha, step, log, analytic star).

---

## Issue 4: Mock engine terminal status vs vertex-colour honesty

- Severity: major
- Section: PRD FR-IMG-12 · design §18 Mock engine · design §21.4 Export
- Description: FR-IMG-12: vertex colour / factors-only → status **`degraded`**, never a green `succeeded` with a grey-ish mesh. Design §18: mock walks `queued → running → succeeded` and emits vertex colours only. Design §21.4 adds a **preview exception** (COLOR_0 → degraded *unless* user asked preview). Mock is always `quality=preview`. PRD has no preview exception. S2 “mock hash pinned” + S11 “grey mesh → failed / vertex_color → degraded” will fight in CI: either the golden mock is `succeeded` (PRD-illegal) or every Nano generate is `degraded` (CLI exit 0 still, but G3/M4 “round-trip succeeded” tests will be wrong).
- Suggestion: State the mock outcome in one line in both docs. Recommended: mock is `degraded` + `export.material_mode=vertex_color` + `disclaimer=not-a-model`, still parser-valid GLB; CLI exit 0 (already true for degraded). Drop the preview exception or confine it to “preview **and** user accepted vertex colour via quality=preview”, written in FR-IMG-12.
- Status: addressed
- Response: Mock is `degraded` + vertex_color + `not-a-model` in PRD/CHARTER/design. Preview exception dropped. CLI exit for degraded is **1** (clean-room Issue 8), not 0 — stronger honesty than this issue’s “keep 0.” M4/S2 tests assert `degraded`.

---

## Issue 5: View Contract / Lattice / gates are typed but not implementable

- Severity: major
- Section: PRD §7.2–7.8 · design §4–6
- Description: The invention is **not** a slogan: schema id, JSON fields, presets 4/6/8, Hero-Orbit steps, RetryPolicy numbers, G0–G4 thresholds, and `gate_version=g0_v0` are real. These holes still force implementers to invent product behaviour:
  1. **`extract_noun_phrase`**: “keep material, colour, garment, species; strip camera words” — camera-word list exists; noun-phrase algorithm does not (whole remainder? first clause? POS?). Example `identity_phrase` equals `prompt.raw`.
  2. **`prompt.normalized`**: undefined (case, whitespace, Unicode, language).
  3. **`subject_lock.attributes[]`**: no extractor.
  4. **`language`**: no detector; no “v1 English-only” lock.
  5. **Classifier lists**: creature “species list” is unnamed; `character` vs `creature` has no rule (G2 applies to both, but class still leaks into lighting/negatives).
  6. **No-hero preset:** quality `preview` → `cardinal4` has no `hero`. `canonical_view_id`, G0, G1-vs-hero, and hand-off “hero else front” are not compiled together. Compiler can emit a contract that fails its own gates.
  7. **G1 “adjacent cardinals ≥ 0.70”:** adjacency graph undefined once `hero` / `top` / `qne` exist.
  8. **G3/G4 geometry:** “non-background gray/white cluster or alpha”, “bbox not glued to two opposite edges”, gray-world ratios — no algorithm, kernel, or pixel tolerance. G3 fail is fail-closed (FR-TXT-15) but the retry ladder identifies worst view by **G1/G2**, so G3-only failures have no repair step.
  9. **`t2i.quality_tier`:** example is `"preview"` on a standard job; mapping from MeshJob `quality` → T2I tier/model is missing (default model `grok-imagine-image-2.0` is example-only).
  10. **`orbit_seed_mode=family_plus_view_index`:** view index order unspecified (preset table order? required-only?).
  11. **Class lighting vs example:** design §4.1 creature → overcast+gray; PRD §7.4 example creature uses `studio_three_point` and `compile_notes` that do not match `lighting.rig`. S5 goldens will not have a single expected JSON.
- Suggestion: Add a compiler appendix (design §4) with: identity = camera-stripped remainder (fixture table of (prompt, identity_phrase, class)); v1 `language="en"` no detect; `attributes` = empty or a closed adjective list; `character` iff person/humanoid tokens else `creature`; `canonical_view_id = hero` if present else `front`; adjacency = ring order of cardinals `{front,right,back,left}` only; G3 = luminance/chroma threshold vs `background.hex` **or** alpha, bbox margin ≥ N px from two opposite edges; G3/G4 failures enter the same worst-view ladder; `t2i.quality_tier` = preview if job quality=preview else quality; `view_index` = index in `cameras[]`. Make the PRD JSON example match the compiler (overcast for creature, six cameras or a truncated-but-labelled excerpt).
- Status: addressed
- Response: design §4.1 / §5 / §6 now specify identity remainder, NFC normalize, language=en, attributes=[], humanoid vs creature lists, canonical_view_id, cardinal adjacency, G3/G4 pixel algorithms, G3/G4 on the retry ladder, t2i.quality_tier, view_index. PRD example: overcast, attributes=[], abridged cameras labelled.

---

## Issue 6: MCP / CLI / HTTP do not share one submit surface

- Severity: major
- Section: PRD §9 FR-FAC-1..15 · design §3, §10–12 · PRD Appendix A.1
- Description: G5 / D18 / FR-FAC-1 require one type layer. The published faces are not a projection of MeshJob:
  | Field / op | MeshJob / design | CLI | MCP A.1 | HTTP |
  |---|---|---|---|---|
  | Image bytes | `input.kind=image` | `--image PATH` | `image_path` | **unspecified** (no multipart, path, or URL; POST example is prompt-only) |
  | `allow_native_text` | field, default false | **missing** from job flags | **missing** | **missing** |
  | `license_override` | field (D19 Hunyuan) | **missing** | **missing** | **missing** |
  | `export.*` | object | **missing** | **missing** | **missing** |
  | `budget.max_wall_s` / credits | object | `--max-usd` only | `max_usd` only | `max_usd` only |
  | Confirm spend | `needs_confirm` | no `confirm`; `generate --allow-spend` only | resubmit `job_id`+`allow_spend` | **`POST /v1/jobs/{id}/confirm`** |
  | Wait | director | `wait` | `text2mesh_wait` (min 30) | **no wait**; poll/SSE only |
  | `generate` blocking | — | **unspecified** (human maker persona needs a one-shot GLB) | submit non-blocking (correct) | 202 (correct) |

  HTTP image-to-mesh (G1/M1) cannot be called as specified. Route C and Hunyuan attestation cannot be set from a face. Confirm is a third protocol. `tools/list` vs OpenAPI vs clap will drift immediately because A.1 is a subset and design §10.4 says “see PRD Appendix A.1” instead of freezing the type.
- Suggestion: Freeze **one** `JobSubmit` struct in design (every MeshJob field that a caller may set). Generate clap/MCP/OpenAPI from it (Issue 7). Document face-only ops: HTTP has no blocking wait; MCP/CLI have no SSE; CLI `generate` = submit + wait (floor 30 s); MCP `submit` never blocks; confirm is `POST /confirm` **and** `submit{job_id, allow_spend}` **and** `text2mesh confirm JOB`. Specify HTTP image as raw body `Content-Type: image/png|jpeg` **or** JSON `{ "image_path" }` **or** multipart — pick one, plus max bytes from Issue 2. Add `allow_native_text` and export flags to the shared struct (even if rarely used).
- Status: addressed
- Response: `JobSubmit` frozen in design §3.4 with all caller fields including export, `allow_native_text`, `license_override`, `max_wall_s`. HTTP image = JSON path **or** multipart (`image`+`spec`). Confirm on all three faces. Face-only ops documented.

---

## Issue 7: Schema-drift CI has no owner in the PR Plan

- Severity: major
- Section: PRD G5, FR-FAC-1, KD-17, NFR-15 · CHARTER D18 · design invariant 15 · PR Plan PR-01/05/06
- Description: Drift = CI fail is load-bearing. PR-01 is “serde types + JSON roundtrip”. PR-05 MCP tools/list. PR-06 “OpenAPI from types”. No PR adds: generated MCP tool JSON, generated OpenAPI, clap dump, and a CI job that diffs them. Without that slice, G5 is a slogan and faces will hand-write schemas (Issue 6 already started).
- Suggestion: Add **PR-01b** or extend PR-01: `schemars`/`utoipa` (or garden equivalent) from the same structs; `tests/schema_drift.rs` compares `mcp.tools.json`, `openapi.json`, `cli --help-json` if any. PR-05/06 consume the artefact, they do not invent fields. Name the test in design §7.3 next to the planner table.
- Status: addressed
- Response: PR-01b added; `schema_drift_cli_mcp_openapi` in design §7.3. Faces consume the artefact.

---

## Issue 8: Auto planner cannot choose among feasible remotes (and has no I/O unit table)

- Severity: major
- Section: PRD §8.2 FR-CMP-5..8 · design §7.2–7.3 · research compute-plane §4.2 / §12.1
- Description: Dual-path architecture is present: one trait, Local+Remote, `auto` is not a plane, mock never auto, `ultra` never auto, first-reason degrade order frozen, local preferred when feasible. Two implementability holes:
  1. **Tie-break:** if Meshy, Tripo, and `remote.colony` are all feasible, auto has no order. Same for sidecar vs preview when both pass (order is stated: sidecar then preview) — remotes have nothing analogous. Catalog “supports route” is undefined per provider (image-only vs text vs multiview).
  2. **Mandatory CI table:** FR-CMP-6 says the unit table lives in “design.md / compute-plane §12.1”. compute-plane §12.1 is a **list of test names**, not `(spec, ProbeSnapshot, SpendPolicy) → PlaneChoice` rows. design §7.3 copies the names. Implementers can pass vacuously.
- Suggestion: Freeze auto remote order in design §7.2, e.g. `remote.colony` (usd=0) → `remote.tripo` → `remote.meshy` → `remote.custom` → `remote.hunyuan_hosted` (still gated). Add a 8–12 row fixture table (CPU-only/no weights/no key; weights+VRAM; weights missing+key+gate open; local mode + no weights; user pinned CUDA missing; usd>0 gate closed; allow-mock; ultra rewrite). Point FR-CMP-6 at **design §7.3 table**, not the research note.
- Status: addressed
- Response: Remote order colony → tripo → meshy. `remote.custom` dropped from v1. Hunyuan never auto if others feasible. 12-row I/O table in design §7.3. FR-CMP-6 points there.

---

## Issue 9: Sibling compose is policy-complete and wire-empty

- Severity: major
- Section: PRD §7.3, §7.10 · CHARTER D5 · design §5.1, §19
- Description: Isolation rules are complete (no `XAI_API_KEY`, no `imaginarium-slint`, no OCCT, refuse-if-absent, INSTALLED≠ACTIVE). Implementers of S8/S14 still cannot type a request:
  - **Imaginarium:** design says HTTP `TEXT2MESH_IMAGINARIUM_URL` default `:8791` and “call **their** estimate”. No method/path, no body, no mapping from View Contract → generate vs edit, no where bytes land (`content_url` vs job dir). Sibling public surface is `POST /v1/estimate`, `POST /v1/images/generations`, `POST /v1/images/edits` (1–3 sources — already cited as Imagine cap). None of those strings appear in PRD/design.
  - **Cadre:** design lists `cadre write-source --http`, `cadre build`, `cadre export --format glb|step`. Cadre HTTP is `/v1/build`, `/v1/export`, jobs. No JSON body, no where Starlark is written (job dir?), no poll vs sync, no how we detect “Cadre live” (health? version?).
  - **OmniOcular:** “may visualize” — fine for v1 (no slice).
- Suggestion: Add design §19.1 / §5.1 wire tables: method, path, request fields we send, response fields we read, timeout, error map onto `t2i.*` / `analytic.*`. Do not copy sibling internals; cite their public routes. State Starlark lands in `jobs/<id>/analytic/source.star` (or similar) that Cadre is allowed to read. Probe = `GET /v1/health` with the existing 5 s/20 s budget.
- Status: addressed
- Response: design §19.1 Imaginarium and §19.2 Cadre public routes, bodies, errors, probe GET /v1/health. Starlark at `jobs/<id>/analytic/source.star`.

---

## Issue 10: PR Plan is ordered, but not a complete cover of the contract

- Severity: major
- Section: PRD §19 · CHARTER Phases S0–S12 · PRD §14
- Description: Merge rules are correct (fresh `main`, PR-17/PR-18 not stacked, PR-04/05/06 parallel after PR-03). Coverage gaps vs the rest of the pack:
  1. **Idle unload** (FR-IMG-23, CHARTER S12, PRD S12) is not in any PR. PR-22 is weights + licenses + Hunyuan only.
  2. **S5 goldens need `evals/text2/prompts.json` at PR-10**; PR-21 (eval harness) depends on PR-12 and is the first PR that names that file. Goldens and eval will fork prompt sets.
  3. **FR-TXT-21 `local.trellis_text`** is a Route C provider. Frozen `PlaneId` (design §2, PRD §8.1) has no such id. Either add the plane + a PR, or strike it from v1 (sidecar/native passthrough only).
  4. **HTTP image ingest, confirm parity, export flags** (Issue 6) have no slice.
  5. **Schema-drift CI** (Issue 7) has no slice.
  6. PR-04 “CLI generate against mock” lands **before** planner PR-09. Description does not freeze `--compute local --provider local.mock` (or equivalent) so S2 does not silently call auto.
- Suggestion: Extend PR-10 to check in `evals/text2/prompts.json` (compile goldens share it). Add idle unload to PR-22 (or PR-16 sidecar). Delete or enum-lock `local.trellis_text`. Add a line on PR-04: force mock provider; auto still forbidden. Point Issues 6–7 at named PRs. Keep mergeability: do not stack PR-17/18.
- Status: addressed
- Response: PR-10 checks in `prompts.json`. Idle unload on PR-16/22. `local.trellis_text` struck. PR-04 forces `--provider local.mock`. PR-01b/JobSubmit cover 6–7. Stacking called out for 04/05/06 and 17/18.

---

## Issue 11: `needs_confirm` and idempotency can sit forever

- Severity: minor
- Section: PRD §8.6 FR-CMP-22..26 · design §8.1 Watchdog · FR-IMG-2
- Description: Orphan `pending` is banned; states and mermaid are implementable; local crash vs remote `waiting_upstream` is resolved. Watchdog table covers `queued`, local `running`, remote heartbeat, `recover_ttl`. It does **not** mention `needs_confirm` (spend gate waiting on a human/agent). Idempotency is “same key inside the store TTL” but **store TTL is unnamed** (hours? forever?). A confirm row or a duplicate key can live unbounded; that is not `pending`, but it is a job that never reaches a terminal state without operator action — unstated.
- Suggestion: Document `needs_confirm` as user-owned: no watchdog fail, optional `confirm_ttl` (Recommended: 24 h → `failed` `spend.gated` or `cancelled`, no POST). Freeze idempotency window (Recommended: same as `recover_ttl` 24 h, or store-lifetime while the job dir exists). Add one watchdog row so S1 tests exist.
- Status: addressed
- Response: `confirm_ttl` 24 h → `failed` `spend.gated`. Idempotency = `recover_ttl` (24 h) or job-dir lifetime, whichever shorter. Watchdog row + `watchdog_needs_confirm_ttl` test.

---

## Issue 12: Nested T2I children and MCP wait `ok` are underspecified

- Severity: minor
- Section: design §8.2 · design §10.6 · PRD FR-TXT-14
- Description: Parent stays `running` while T2I children run / `waiting_upstream` — good. Child ids go in `artifacts.views[]` and `manifest.child_jobs`. It is not said whether children are `MeshJob`s (`input.kind=?`), a distinct `T2iJob`, or just files. Faces listing jobs will either leak internals or hide spend. Separately, `text2mesh_wait` allows **both** `ok=true` + non-terminal status **and** `ok=false` `wait.timeout` while the job is `waiting_upstream`/`running`. Agents cannot implement a correct wait loop from the text.
- Suggestion: Children are not MeshJobs in v1; they are `child_jobs[]` of `{id, kind:t2i, provider, upstream_id, status, usd}` and do not appear in `text2mesh_list_jobs` unless `include_children`. Freeze wait: call `ok=true` always when the RPC succeeds; job `status` and `error_type=wait.timeout` on the snapshot communicate timeout; JSON-RPC/`isError` only for unknown `job_id` / protocol. Align CLI exit 8 with that snapshot.
- Status: addressed
- Response: Children are `child_jobs[]` structs, not MeshJobs; hidden from list unless `include_children`. Wait wrapper `ok=true` + `wait_timed_out`; CLI exit 8 inspects snapshot.

---

## Issue 13: Honesty extras not frozen (key length/head, config.toml, custom mapping)

- Severity: minor
- Section: PRD §13.3 · design §13 · NFR-10 · FR-CMP-21
- Description: `system-check` must report keys as length/head. Design example rows are `{id, present}` only (`XAI_API_KEY` present=false always is specified). Config file `~/.config/text2mesh/config.toml` is “later” while env is complete — S3 estimate/caps will hard-code env-only. `remote.custom` “small mapping file” has no schema (quality name → vendor knob). None block S0; they will fork before S10.
- Suggestion: Extend `text2mesh.system_check.v1` key row: `present`, `len`, `head` (2–4 chars), never `XAI` head. Sketch config.toml keys as a 1:1 of design §22 env (env wins). Custom mapping: JSON `{ "quality": {"preview": "…", "standard": "…"}, "route": {"image": "POST /…"} }` or drop `remote.custom` from v1 PlaneId (Meshy/Tripo/colony only).
- Status: addressed
- Response: Key rows have `present`/`len`/`head`; never XAI head. config.toml is 1:1 of non-secret env (env wins). `remote.custom` dropped from v1.

---

## Issue 14: DocSmith “Open Questions & Risks” heading missing

- Severity: nit
- Section: house-and-siblings §7 template vs PRD §18
- Description: Writer-mandatory §0–20 + appendices are all present (including Key Decisions and PR Plan, which DocSmith’s short ToC does not even name). The house template’s §13 title is “Open Questions & Risks”. PRD §18 is Open Questions only. License/territory/spend risks live in §12 and OQ-1..7; they are not gathered as risks with owners.
- Suggestion: Add a short §18.1 Risks table (CLIP uncalibrated, sidecar GPL handshake, Imaginarium down, I2I billing, sidecar OOM killing MCP if isolation fails) pointing at existing D*/OQ. Do not invent new product forks.
- Status: addressed
- Response: PRD §18.1 Risks table added; owners point at D*/OQ. No new forks.

---

## Issue 15: Illustrative View Contract JSON is not a valid Default-6 contract

- Severity: nit
- Section: PRD §7.4 example
- Description: Example has `"count": 6` and `preset: cardinal4_hero_top` but `cameras` contains only `hero`. `compile_notes` says `lighting=overcast+gray` while `lighting.rig` is `studio_three_point`. Fine as a sketch if labelled truncated; as written it will be copied into S5 goldens (Issue 5).
- Suggestion: Label the block “abridged; cameras[1:] omitted” **or** paste the full six-row table. Align `compile_notes` with `lighting.rig` after Issue 5’s class defaults.
- Status: addressed
- Response: Example labelled abridged; lighting.rig=`overcast`; compile_notes match; attributes=[].

---

## Summary

**Verdict: not-ready** (`open_count` = 15).

The pack is *structurally* complete: every writer-mandatory PRD section exists; CHARTER D1–D27 are dated and binding; design invariants + MeshJob/View Contract schemas + mermaid state machine are real; Nano feature gates, mock-always, spend-closed, no-orphan-`pending`, M1–M10 (M3 is numeric), and alternatives (a)(b)(c) all land. Dual compute is not a slogan (`ComputePlane` + Local/Remote + auto-as-planner). Lattice + View Contract + Hero-Orbit is a typed invention, not “call a T2I API”.

It is not *implementation*-complete. PRD and design disagree on defaults and on-disk layout (Issues 1–4). The text2 compiler/gates still hide product algorithms behind names (Issue 5). Faces do not project one submit type (Issues 6–7). Auto cannot break remote ties and has test names instead of a unit table (Issue 8). Cadre/Imaginarium compose is refuse-if-absent without URLs (Issue 9). The PR Plan is mergeable but misses idle-unload, prompt-file ordering, and `local.trellis_text` (Issue 10).

Do not start S1 types until Issues 1–4 and the `JobSubmit` field list are frozen in design. S0 scaffold can proceed.

---

## Revision Summary (2026-08-19, PRD Draft v0.2)

All 15 issues **addressed**. Export defaults, image caps, and store layout now agree. Mock is `degraded` (CLI exit **1**, not 0 — honesty over this review’s “keep 0”). Compiler/gates have algorithms. `JobSubmit` + PR-01b own schema drift. Planner has remote order + 12-row I/O table. Imaginarium/Cadre wires typed. PR plan covers idle-unload, prompts.json, no `local.trellis_text`. `confirm_ttl` / idempotency frozen. Children are not MeshJobs. Keys have len/head; `remote.custom` dropped. §18.1 Risks added. View Contract example abridged + overcast.

| Severity | Open |
|---|---|
| critical | 0 |
| major | 10 |
| minor | 3 |
| nit | 2 |
| **total** | **15** |
