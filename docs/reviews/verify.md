# Independent verification — text2mesh PRD pack v0.2

| Field | Value |
|---|---|
| **Date** | 2026-08-19 |
| **Reviewer** | GROK (independent skeptic; not the revision author) |
| **Read** | `docs/prd.md` Draft v0.2, `docs/CHARTER.md` (D1–D30), `docs/design.md` v0.2, `docs/reviews/clean-room.md`, `completeness.md`, `feasibility.md` |
| **Not in scope** | Rewriting the pack; treating research notes as the contract |
| **Fail-closed** | Missing review file or empty `Response` ⇒ open. Rubber-stamp of `Status: addressed` without a doc check ⇒ not allowed. |

Three review files are present. Every former issue has `Status: addressed` and a non-empty `Response`. Those Responses were checked against the revised docs, not against the revision summaries.

---

## Process gate

| Check | Result |
|---|---|
| `docs/reviews/clean-room.md` | present |
| `docs/reviews/completeness.md` | present |
| `docs/reviews/feasibility.md` | present |
| Former issues with `Status` | 10 + 15 + 8 = **33 / 33** |
| Former issues with non-empty `Response` | **33 / 33** |
| `Status: open` remaining in the three files | **0** (all marked addressed; several leftovers are still live — see NEW) |

No process fail. The pack is **not** therefore ready.

---

## Former issues — independent check

Legend: **holds** = the Response’s claim is in the revised docs. **leftover** = original hole mostly closed; a residue is filed as NEW.

### clean-room.md (10)

| # | Claimed fix | In the docs? |
|---|---|---|
| 1 | Appendix B writer-only; GitHub bullets README/LICENSE only; D2 + design Provenance + §28 | **holds.** Banner at PRD Appendix B; CHARTER D2; design Provenance + §28.1. URLs remain on purpose (Response says so). |
| 2 | design Provenance; CHARTER custody paragraph; research notes ≠ specs | **holds.** |
| 3 | Drop `ok` from `system_check.v1`; `report_complete` + `ready` | **holds.** design §13 example has no `ok`. FR-FAC-7 / D29. |
| 4 | No preview exception; vertex-colour `degraded`; grey `failed`; mock `degraded` | **holds.** D9/D24; FR-IMG-12/13; design §18/§21. |
| 5 | `ok` split (D29); wait timeout wrapper `ok=true` + `wait_timed_out` | **holds.** Deliberate divergence from the review’s `ok=false` suggestion, with a real reason (fake-failure twin). |
| 6 | D19 planner never auto-Hunyuan; fixture rows 11–12 | **holds.** design §7.2 steps 11–12 + table rows 11–12. |
| 7 | Voxel exponents / vendor ckpts / `local.trellis_text` / `t2m_abi` out of public contract | **holds** on those items. Residual marketing string “TRELLIS.2-class” still in PRD §7.6 hand-off — not an ABI. |
| 8 | CLI exit 0 only `succeeded`; exit 1 `degraded`; manifest `ok` only `succeeded` | **holds.** FR-FAC-6; design §12; PRD §13.1 example `ok: false`. |
| 9 | 202 has no `artifact_url`; GET artefact 409 until terminal | **holds.** FR-FAC-14; design §9/§11. |
| 10 | Progress `form fields`; export defaults false | **holds.** design §17/§3. |

### completeness.md (15)

| # | Claimed fix | In the docs? |
|---|---|---|
| 1 | Export defaults all false; export on `JobSubmit` | **holds.** |
| 2 | 32 MiB compressed / 4096 / 64 MiB uncompressed; no auto-scale | **holds.** FR-IMG-4 = design §20. |
| 3 | Store root + sqlite at root + `jobs/<id>/` | **holds.** NFR-9; PRD §11.4; design §16. |
| 4 | Mock `degraded` + vertex_color; CLI exit **1** | **holds** (stronger than the review’s “keep exit 0”). |
| 5 | Compiler/gate algorithms in design §4–6 | **partial.** The eleven listed holes are specified in design. PRD gate table, G2 FACE strings, fail-closed vs ladder, and unfrozen prompt strings still fork — NEW 2–3. |
| 6 | `JobSubmit` §3.4; HTTP image JSON or multipart; confirm on three faces | **partial.** Type exists; design §11 has confirm. PRD FR-FAC-13 route list still omits it — NEW 6. |
| 7 | PR-01b + `schema_drift_cli_mcp_openapi` | **holds.** |
| 8 | Remote order colony→tripo→meshy; 12-row I/O table; FR-CMP-6 → design §7.3 | **holds.** Row 6 still allows two outcomes (`vram_short` **or** `spend.gated`) — not reopened. |
| 9 | Imaginarium/Cadre public wires §19.1/§19.2 | **holds** at the level requested (routes, probe, errors, Starlark path). Bodies stay “as their public estimate accepts.” |
| 10 | `prompts.json` in PR-10; idle unload PR-16/22; `local.trellis_text` struck; PR-04 pins mock | **holds.** New fixture files named only in design — NEW 7. |
| 11 | `confirm_ttl` 24 h; idempotency = `recover_ttl` | **holds.** |
| 12 | `child_jobs[]` not MeshJobs; wait wrapper `ok=true` | **holds.** |
| 13 | Key `present`/`len`/`head`; config.toml 1:1 env; drop `remote.custom` | **partial.** PlaneId dropped. PRD env + cargo table still advertise custom — NEW 5. |
| 14 | §18.1 Risks | **holds.** |
| 15 | View Contract example abridged + overcast | **holds.** |

### feasibility.md (8)

| # | Claimed fix | In the docs? |
|---|---|---|
| 1 | One VRAM/disk number per pick; Krackan 512 MiB shared → remote or degrade | **holds.** Floors 6144 / 24576 / 16384. system-check example is honest. |
| 2 | Stages = progress names (D28); `quality-*` unscheduled; S11 live GLB = fixture \| LIVE remote \| stated degrade | **holds.** |
| 3 | Crash only if pid dead; `TEXT2MESH_HB_S=300`; director heartbeats | **partial.** design §8.1 / FR-CMP-26 hold. PRD §13.2 still says silence → watchdog — NEW 4. |
| 4 | Route B `max_wall_s=1800`; Nano 180 s = mock/preview only | **partial.** Job wall is 1800. MCP wait default in the PRD tools table is still **600** — NEW 1. |
| 5 | PR-07 split; do-not-stack on 04/05/06; PR-13→01, 15→08, 16→03 | **holds.** |
| 6 | Four crates only; D30; “no I/O glue” retracted | **holds.** |
| 7 | Probe vs job timeout tables; D14/NFR-4 name the probe exception | **holds.** |
| 8 | Export flags default off | **holds.** |

Former issues are **not** reopened. Leftovers that still mint implementer forks are NEW.

---

## NEW issues (revision leftovers)

### Issue 1: MCP wait default is still 600 s; the rest of the clock is 1800 s

- Severity: major
- Section: PRD §9.3 tools table `text2mesh_wait`; design §10.6; design §3.4 `max_wall_s`; FR-TXT-19; PRD §13.2 “wait default **1800 s**”
- Description: Feasibility Issue 4’s job wall was raised to 1800 s (MeshJob, JobSubmit, CLI `wait` / `generate`, Route B). The **PRD MCP tool table** still says `text2mesh_wait` default **600**. Design §10.6 freezes default **1800**. PRD §13.2 also says wait default 1800. That is an intra-PRD freeze conflict plus a PRD/design freeze conflict. Hero-Orbit Default-6 with retries was the reason 600 s died: ~6×30 s T2I + 5 min mesh ≈ 570–750 s. An MCP agent that omits `timeout_s` will still cut the job the revision claimed to un-kill. Faces do not share one wait default (G5 / D18).
- Suggestion: One number. Frozen: `timeout_s` min 30, default **1800**, max 86400 on MCP, CLI, and Route B `max_wall_s`. Delete 600 from the PRD tools table. Add a schema-drift / fixture line that the generated MCP tool default equals CLI `--timeout-s` default.
- Status: addressed
- Response: PRD §9.3 `text2mesh_wait` default **1800** (min 30, max 86400). Same bounds in FR-FAC-10, FR-TXT-19, KD-34, design §3.4 / §10.6 / §12 / §24. PR-01b + design §7.3 `schema_drift_cli_mcp_openapi`: generated MCP `timeout_s` default equals CLI `--timeout-s` default (1800). Remaining “600 s” in design is CPU-preview catalog latency, not a wait default.

### Issue 2: Gate tables are still two contracts

- Severity: major
- Section: PRD §7.7 G0–G4 + FR-TXT-15; design invariant 11; design §4.1 `canonical_view_id`; design §6
- Description: Completeness Issue 5 item 6 was “preview `cardinal4` has no `hero`; compiler can fail its own gates.” Design §4.1 / §6 now score G0/G1/G4 against **`canonical_view_id`** (`hero` if present, else `front`). The **PRD gate table and the sentence under it still say `hero`**. Preview 4-view coded from the PRD will look up a missing camera. Separately:
  1. **G2 FACE strings disagree.** PRD: difference vs `"a face, two eyes, front of a head"`, **and** back closer to BACK than to **`"a face looking at camera"`** (a third string). Design: back closer to BACK than to **FACE** (the first string only). M3 / Janus is not one test.
  2. **Fail-closed vs ladder disagree on G2/G4.** FR-TXT-15 and design invariant 11: skip 3D only if a **required** view still fails **G1 or G3**. Design §6 ladder includes G2/G3/G4 and ends `fail-closed view.consistency`. After a G4-only (or G2-only) required miss, one reading spends 3D, the other does not. `error_type` is `view.janus_face` vs `view.consistency` vs “continue.”
  3. **G0 text side is not an algorithm.** `identity ∪ normalized` is not concat / max / both-must-pass. When camera-stripping changes the phrase, implementers invent the CLIP text.
- Suggestion: PRD §7.7 must use `canonical_view_id`, not `hero`. Pick **one** G2 FACE/BACK pair (Recommended: design’s two strings) and delete the third. Amend invariant 11 / FR-TXT-15 to match the ladder: required view failing **any of G1–G4** after retries → do not call Image3dPlane; keep specific `error_type` on the gate that failed, wrap with `view.consistency` on exhaust. Freeze G0 text as `identity_phrase` if non-empty else `normalized` (single string).
- Status: addressed
- Response: PRD §7.7 G0/G1/G4 now score vs `canonical_view_id`. G2 FACE/BACK pair only; third string deleted. G0 text = `identity_phrase` if non-empty else `prompt.normalized` (not union/concat/max). Sentence under the table: G1 vs `canonical_view_id`, plus G2 for faces; preview `cardinal4` has no `hero`. FR-TXT-15 + design invariant 11 + §6 ladder + §8.2: required view failing any of G1–G4 after retries → do not call Image3dPlane; keep specific `error_type`; wrap `view.consistency` on exhaust. Design §6 G0 line aligned.

### Issue 3: S5 goldens still have no frozen prompt strings

- Severity: major
- Section: design §4.1 class table; §4.3 preset tables; §4.4 assembly; PRD §7.4 example
- Description: Completeness Issue 5’s punchline was “S5 goldens will not have a single expected JSON.” The revision froze identity remainder, NFC, class lists, cameras, `lighting.rig`, and `background.mode`. It did **not** freeze the strings the contract hashes:
  - `cameras[].prompt_suffix` — §4.3 has az/el/required/role only. Only the PRD **hero** example has a suffix.
  - `lighting.prompt_lock`, `background.prompt_lock`, `style_lock.prompt_lock`, `style_lock.medium` — one creature example (`photoreal_product` on a fox); no table by class. Product/architecture/vehicle/prop/unknown are invented.
  - `negatives[]` — example list in PRD; design only says “Janus negatives” for creature/character.
  Two PR-10 implementers will emit different `contract_hash` values for the same 24 prompts. `evals/text2/identity.json` does not save this — it does not contain those fields.
- Suggestion: Add a `prompt_suffix` column to design §4.3 (every preset row). Add a class→`{lighting.prompt_lock, background.prompt_lock, style_lock.medium, style_lock.prompt_lock, negatives[]}` table in §4.1. Creature/character negatives must be the Janus set, frozen as an exact JSON array. S5 goldens are then a function of those tables, not of taste.
- Status: addressed
- Response: design §4.3 every preset row has frozen `prompt_suffix` (front/right/back/left/hero/top/qne/qnw as specified). design §4.1 class lock table + exact Janus/other `negatives[]` JSON arrays. PRD §7.4 example uses those exact strings; §7.5 tables carry the same suffixes. S5 goldens are a function of those tables.

### Issue 4: Observability still teaches “silence → watchdog”

- Severity: minor
- Section: PRD §13.2 vs FR-CMP-26 / design §8.1 / `TEXT2MESH_HB_S=300`
- Description: Feasibility Issue 3 closed the 30 s generate-kill in the **watchdog table** (pid-live = alive, HB 300 s). PRD §13.2 still: “Heartbeats … every ≤5 s while `running`. Silence → watchdog.” An S1 implementer of observability who never re-reads FR-CMP-26 will fail local `running` on a quiet sidecar. The binding table and the timing section now fight.
- Suggestion: Replace §13.2 with: emit progress when the engine does; director parent-heartbeat while children run; watchdog may **not** treat missing lines as crash if `pid` is live. Point at design §8.1. Keep “≤5 s” only as a **should** cadence, not a kill.
- Status: addressed
- Response: PRD §13.2 rewritten: emit progress when the engine does; director parent-heartbeat while children run; watchdog may **not** treat missing lines as crash if `pid` is live (design §8.1). ≤5 s is a should cadence, not a kill. “Silence → watchdog.” deleted.

### Issue 5: `remote.custom` was dropped from PlaneId and left in the env/cargo surface

- Severity: minor
- Section: PRD §12.3 `TEXT2MESH_CUSTOM_BASE_URL` / `TEXT2MESH_CUSTOM_KEY`; PRD §10.1 `remote-http` “Meshy/Tripo/**custom** adapters”; design §22 (no such keys); FR-CMP-21 / design §2
- Description: Completeness Issue 13 / 8 Responses say `remote.custom` is gone in v1. Frozen `PlaneId` agrees. The PRD env table still documents a custom adapter key pair that design §22 does not list. The cargo blurb still says “custom adapters.” That is how the dropped plane comes back in S10 without a CHARTER amendment.
- Suggestion: Delete `TEXT2MESH_CUSTOM_*` from PRD §12.3. Change the feature blurb to “Meshy/Tripo/colony adapters (inert without keys).” If a custom plane is wanted later, it is a dated D* + PlaneId bump.
- Status: addressed
- Response: Deleted `TEXT2MESH_CUSTOM_BASE_URL` / `TEXT2MESH_CUSTOM_KEY` from PRD §12.3. Cargo `remote-http` blurb is “Meshy/Tripo/colony adapters (inert without keys)” (PRD §10.1; design §23 aligned). No `remote.custom` in v1. design §22 had no custom keys — left as-is (still states the plane is dropped).

### Issue 6: FR-FAC-13 route list still omits confirm

- Severity: minor
- Section: PRD FR-FAC-13 vs FR-FAC-14 vs design §11
- Description: Completeness Issue 6 required confirm on all three faces. design §3.4 / §11 and FR-FAC-14 have `POST /v1/jobs/{id}/confirm`. The FR-FAC-13 **route block** (the thing a face implementer copies) still ends at cancel + artifact + events. HTTP confirm is specified twice and missing once.
- Suggestion: Put `POST /v1/jobs/{id}/confirm` in the FR-FAC-13 list. Optionally `?kind=` include `contract` (design §11 has it; the PRD list does not).
- Status: addressed
- Response: PRD FR-FAC-13 route block now includes `POST /v1/jobs/{id}/confirm`. Artifact `GET` `?kind=` includes `contract` (`glb|manifest|contract|view`), matching design §11.

### Issue 7: Design now requires fixture files the PR Plan does not check in

- Severity: minor
- Section: design §4.1 `evals/text2/identity.json`; §5 `classify.json`, `species.txt`; §6 `evals/text2/scores/`; PRD §19 PR-10 / PR-13 / PR-21
- Description: Completeness Issue 10 was “goldens and eval will fork prompt sets” — `prompts.json` moved to PR-10. The v0.2 compiler now also names `identity.json`, `classify.json`, `species.txt`, and `scores/`. PR-10 still only checks in `prompts.json`. PR-13 says “classifier table” without the filename. Two slices can invent two species lists (inline tokens in §5 vs a file that “extends only with a design amendment”).
- Suggestion: PR-10 checks in `prompts.json` **and** `identity.json`. PR-13 checks in `classify.json` + `species.txt` (species file = the closed list; inline tokens must match it, not a second list). PR-11 may add `evals/text2/scores/` fixtures. Same files as PR-21.
- Status: addressed
- Response: PRD §19: PR-10 checks in `prompts.json` **and** `identity.json`. PR-13 checks in `classify.json` + `species.txt` (file = closed list; inline tokens must match it). PR-11 may add `evals/text2/scores/`. PR-21 uses those same files. design §5 / §27 name the same set.

---

## What the revision did get right (do not reopen)

- Clean-room custody is now on PRD + CHARTER + design; Appendix B is writer-only; no implementable `t2_*` / `.t2mesh` / ggml graph.
- Honesty: mock/vertex-colour `degraded`; grey `failed`; CLI exit 1; manifest `ok` only `succeeded`; system-check `report_complete`/`ready`; 202 without `artifact_url`; 409 `export.not_ready`; wait timeout does not fake-fail the job row.
- Dual path: 12-row planner table, remote order, Hunyuan never auto, VRAM = device VRAM + `shared`, Krackan 512 MiB example is honest.
- v1 quality is sidecar/remote/degrade (D28); `quality-candle`/`quality-ggml` unscheduled; four crates only (D30).
- Job clocks in the **job** tables are 1800 s / HB 300 / pid-live (except Issue 1 and Issue 4 leftovers).
- `JobSubmit`, PR-01b, confirm/wait face-only ops, Imaginarium/Cadre public routes, `confirm_ttl`, child_jobs.

Those are real edits. They are not a complete freeze.

---

## Summary

**Verdict: not-ready.** `open_count` = **7** (3 major, 4 minor).

Do not present this pack to André as implementation-ready. Majors 1–3 still let S5 goldens, S6 gates, and MCP Hero-Orbit fork. Minors 4–7 are leftover sentences from the same pass (heartbeat, custom env, confirm route, eval files) and should die in the same revision.

Former 33: Responses exist and mostly hold. Completeness 5/6/13 and feasibility 3/4 were marked addressed while residues stayed in the PRD — filed as NEW rather than silently reopened.

Ready for André only when Issues 1–3 are one contract in **both** `prd.md` and `design.md` (CHARTER D15: freeze conflicts are bugs).

---

## Revision Summary (Draft v0.3 writer pass, 2026-08-19)

PRD writer addressed Issues 1–7 in place (no pack rewrite). Status → **Draft v0.3**. CHARTER D1–D30 unchanged; freeze leftovers recorded as a dated amendment.

| # | Fix |
|---|---|
| 1 | One wait clock: min 30 / default 1800 / max 86400 on MCP `timeout_s`, CLI `--timeout-s`, Route B `max_wall_s`. Schema-drift asserts MCP default == CLI default. |
| 2 | One gate contract: G0/G1/G4 vs `canonical_view_id`; G2 two FACE/BACK strings; G0 text = identity_phrase else normalized; required G1–G4 miss after retries does not call Image3dPlane. |
| 3 | Frozen `prompt_suffix` on every preset row; class lock + negatives JSON arrays; PRD §7.4 example uses those strings. |
| 4 | §13.2: engine-driven progress; parent heartbeat; pid-live ≠ crash; ≤5 s should-not-kill. |
| 5 | Dropped `TEXT2MESH_CUSTOM_*`; `remote-http` = Meshy/Tripo/colony (inert without keys). |
| 6 | FR-FAC-13 includes confirm; artifact `?kind=` includes `contract`. |
| 7 | PR-10/11/13/21 share `prompts.json`, `identity.json`, `classify.json`, `species.txt`, `scores/`. |

Independent re-verify of v0.3 is still required before presenting the pack as implementation-ready.
