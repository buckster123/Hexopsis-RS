# Independent verification — text2mesh PRD pack v0.3

| Field | Value |
|---|---|
| **Date** | 2026-08-19 |
| **Reviewer** | GROK (independent skeptic; not the revision author) |
| **Read** | `docs/prd.md` Draft v0.3, `docs/CHARTER.md` (D1–D30 + dated v0.3 amendment), `docs/design.md` v0.3, `docs/reviews/verify.md` Issues 1–7 |
| **Not in scope** | Rewriting the pack; treating research notes as the contract |
| **Fail-closed** | Empty `Response` in `verify.md` ⇒ open. Rubber-stamp of `Status: addressed` without a doc check ⇒ not allowed. Residue of Issues 1–7 ⇒ re-list `Status: open`. NEW issues only if **this** revision introduced a freeze conflict. |
| **Grep** | `default 600`, `Silence → watchdog`, `TEXT2MESH_CUSTOM`, `a face looking at camera` in `prd.md` / `design.md` / `CHARTER.md` |

`verify.md` marks Issues 1–7 `Status: addressed` with non-empty Responses. Those Responses were checked against Draft v0.3 `prd.md` and `design.md`, not against the writer’s Revision Summary.

---

## Process gate

| Check | Result |
|---|---|
| `docs/reviews/clean-room.md` | present |
| `docs/reviews/completeness.md` | present |
| `docs/reviews/feasibility.md` | present |
| `docs/reviews/verify.md` | present; Issues 1–7 have `Status` + non-empty `Response` |
| Pack status | PRD **Draft v0.3**; design **v0.3**; CHARTER D1–D30 unchanged + dated freeze amendment |
| `Status: open` remaining in Issues 1–7 after this check | **0** |

No process fail.

---

## Issues 1–7 — independent check

Legend: **holds** = the Response’s claim is in the revised docs. Residue ⇒ re-list `Status: open`.

### Issue 1: MCP wait default is still 600 s; the rest of the clock is 1800 s

- Severity: major
- Section: PRD §9.3 tools table `text2mesh_wait`; FR-FAC-10; FR-TXT-19; KD-34; PR-01b; design §3.4 / §10.6 / §12 / §24
- Description (from v0.2): PRD MCP tool table still said default **600** while design wait/wall was **1800**.
- Suggestion: One number: min 30 / default **1800** / max 86400 on MCP `timeout_s`, CLI `--timeout-s`, Route B `max_wall_s`. Schema-drift: generated MCP default equals CLI default.
- **In the docs?** **holds.** One clock in **both** contracts:
  - PRD tools table: `text2mesh_wait` min 30, default **1800**, max 86400 (FR-FAC-10 same bounds).
  - FR-TXT-19, KD-34, Appendix A `max_wall_s` default 1800, CLI example `--timeout-s 1800`.
  - design JobSubmit `max_wall_s` **1800** (min 30, max 86400); §10.6 / §12 / §24 `job_wait` and Route B wall **1800**.
  - PR-01b + design §7.3 `schema_drift_cli_mcp_openapi`: generated MCP `timeout_s` default **equals** CLI `--timeout-s` default **1800**.
- Grep: no wait-default **600** in `prd.md` / `design.md` / `CHARTER.md`. Remaining “600 s” in design §14 is **CPU-preview catalog latency** (`seconds_p50`), not a wait/wall default. `0600` is env-file mode.
- Status: addressed
- Response: Confirmed in the docs, not in the writer summary.

### Issue 2: Gate tables are still two contracts

- Severity: major
- Section: PRD §7.7 G0–G4 + FR-TXT-15; design invariant 11; design §4.1 `canonical_view_id`; design §6
- Description (from v0.2): PRD scored G0/G1/G4 vs missing `hero`; G2 had a third FACE string; fail-closed was G1/G3 vs ladder G1–G4; G0 text was not an algorithm.
- Suggestion: `canonical_view_id`; one FACE/BACK pair; G0 text = identity_phrase else normalized; required any of G1–G4 after retries → do not call Image3dPlane.
- **In the docs?** **holds.** One gate contract in **both** files:
  1. G0/G1/G4 score vs `canonical_view_id` (`hero` if present else `front`). Preview `cardinal4` has no `hero`; canonical is `front`. Same sentence under the PRD table and in design §4.1 / §6.
  2. FACE = `"a face, two eyes, front of a head"`. BACK = `"the back of a head, no face"`. **No third FACE string** in PRD or design. Grep `a face looking at camera` is gone from the contract trio (research note only — not the contract).
  3. FR-TXT-15 = design invariant 11 = §6 ladder = §8.2: required view failing **any of G1–G4** after retries → **do not** call Image3dPlane; keep specific `error_type`; wrap `view.consistency` on exhaust.
  4. G0 text: `T = identity_phrase` if non-empty else `prompt.normalized` (not union, concat, or max) — identical in PRD §7.7 and design §6.
- Leftover naming (not a fork): fail `error_type` remains `view.hero_text_mismatch` in **both** files; manifest example still has `g1_vs_hero`. Scoring rule is canonical. Do not reopen.
- Status: addressed
- Response: Confirmed in the docs.

### Issue 3: S5 goldens still have no frozen prompt strings

- Severity: major
- Section: design §4.1 class lock table + negatives JSON; §4.3 preset `prompt_suffix`; PRD §7.4 example; PRD §7.5 tables
- Description (from v0.2): suffixes, class locks, and `negatives[]` were invented per implementer → forked `contract_hash`.
- Suggestion: `prompt_suffix` column on every preset row; class lock table; exact Janus/other JSON arrays; PRD example uses those strings.
- **In the docs?** **holds.** One golden function in **both** contracts:
  - design §4.3: every row of `cardinal4` / `cardinal4_hero_top` / `cardinal4_hero_top_quarters` has frozen `prompt_suffix`.
  - PRD §7.5 tables carry the **same** suffixes (4-view full; 6-view full; 8-view = default 6 plus qne/qnw).
  - design §4.1 class lock table covers creature/character, product/prop, architecture, vehicle, unknown. Janus `negatives[]` and other-class `negatives[]` are exact JSON arrays.
  - PRD §7.4 creature example uses those exact lock strings and the Janus array; field rules point at design §4.1 / §4.3. S5 goldens are a function of those tables (PRD note + design §27).
- Status: addressed
- Response: Confirmed in the docs.

### Issue 4: Observability still teaches “silence → watchdog”

- Severity: minor
- Section: PRD §13.2 vs FR-CMP-26 / design §8.1 / `TEXT2MESH_HB_S=300`
- Description (from v0.2): §13.2 still said “Silence → watchdog.”
- **In the docs?** **holds.** Leftover sentence **gone**.
  - PRD §13.2: emit progress when the engine does; ≤5 s is a **should**, **not** a kill; director parent-heartbeats; watchdog may **not** treat missing lines as crash if `pid` is live (design §8.1).
  - Grep `Silence → watchdog` / `Silence -> watchdog` is absent from `prd.md` / `design.md` / `CHARTER.md`.
  - FR-CMP-26 and design §8.1 still: pid-live + silent progress → **alive**; `TEXT2MESH_HB_S` default **300**.
- Nit (not a freeze, not reopened): state table still says `running` … “heartbeat required.” Binding kill rule is pid-dead. Director must still parent-heartbeat.
- Status: addressed
- Response: Confirmed in the docs.

### Issue 5: `remote.custom` was dropped from PlaneId and left in the env/cargo surface

- Severity: minor
- Section: PRD §12.3; PRD §10.1 `remote-http`; design §22 / §23; FR-CMP-21
- Description (from v0.2): `TEXT2MESH_CUSTOM_*` and “custom adapters” cargo blurb survived the PlaneId drop.
- **In the docs?** **holds.** Leftover sentences **gone**.
  - PRD §12.3 has no `TEXT2MESH_CUSTOM_BASE_URL` / `TEXT2MESH_CUSTOM_KEY`.
  - PRD §10.1 and design §23: `remote-http` = “Meshy/Tripo/colony adapters (inert without keys)”.
  - Frozen PlaneId has no `remote.custom`. FR-CMP-21 and design §22 say dropped.
  - Grep `TEXT2MESH_CUSTOM` in the contract trio is only the CHARTER dated amendment recording the drop.
- Status: addressed
- Response: Confirmed in the docs.

### Issue 6: FR-FAC-13 route list still omits confirm

- Severity: minor
- Section: PRD FR-FAC-13 vs FR-FAC-14 vs design §11
- Description (from v0.2): the copied HTTP route block ended at cancel + artifact; confirm lived only in FR-FAC-14 / design.
- **In the docs?** **holds.** Leftover omission **gone**.
  - FR-FAC-13 includes `POST /v1/jobs/{id}/confirm`.
  - Artifact `GET` `?kind=glb|manifest|contract|view` matches design §11 (`glb\|manifest\|contract\|view`).
- Status: addressed
- Response: Confirmed in the docs.

### Issue 7: Design now requires fixture files the PR Plan does not check in

- Severity: minor
- Section: design §4.1 / §5 / §6 / §27; PRD §19 PR-10 / PR-11 / PR-13 / PR-21; CHARTER S5 / S7
- Description (from v0.2): `identity.json`, `classify.json`, `species.txt`, `scores/` named in design only.
- **In the docs?** **holds.** Leftover split **gone**.
  - PR-10: `prompts.json` **and** `identity.json`.
  - PR-13: `classify.json` + `species.txt` (file = closed list; inline tokens must match it).
  - PR-11 may add `evals/text2/scores/`.
  - PR-21 uses those same files.
  - design §5 / §27 and CHARTER S5/S7 name the same set.
- Status: addressed
- Response: Confirmed in the docs.

---

## NEW issues (revision leftovers)

None. This v0.3 pass did not introduce a freeze conflict between `prd.md` and `design.md`. CHARTER D1–D30 were not silently rewritten; leftovers are a dated amendment (D15 process).

Research notes (`docs/research/text2-layer.md`, `compute-plane.md`) still carry old 600 s walls, G1-or-G3 fail-closed, and the third FACE string. They are **writer notes**, not the contract (design Provenance; CHARTER custody).

---

## What still holds (do not reopen)

- Clean-room custody on PRD + CHARTER + design; Appendix B writer-only.
- Honesty: mock/vertex-colour `degraded`; grey `failed`; CLI exit 1; manifest `ok` only `succeeded`; system-check `report_complete`/`ready`; 202 without `artifact_url`; 409 `export.not_ready`; wait timeout does not fake-fail the job row.
- Dual path: 12-row planner, remote order colony→tripo→meshy, Hunyuan never auto, VRAM = device VRAM + `shared`.
- v1 quality sidecar/remote/degrade (D28); four crates only (D30).
- Job clocks: wait/wall min 30 / default **1800** / max 86400; HB 300; pid-live ≠ crash.
- Confirm on three faces; artifact `kind` includes `contract`; eval fixtures owned by named PRs.

Nits that are **not** freeze conflicts (do not reopen, do not mint NEW):

- G0 fail token is still `view.hero_text_mismatch` while the scored view is `canonical_view_id` — same token in both files.
- PRD §13.1 example field `g1_vs_hero` is an illustration name, not a second G1 rule.
- PRD §7.4 example `t2i.quality_tier: "preview"` on a `cardinal4_hero_top` excerpt; design §4.1 algorithm is `preview` iff `job.quality=preview`, else `quality`. Goldens follow the algorithm + lock tables, not the abridged illustration.
- design §14 “CPU preview 600 s” is estimate catalog latency, not MCP/CLI wait.

---

## Summary

**Verdict: ready.** `open_count` = **0**.

Issues 1–3 are one contract in **both** `prd.md` and `design.md`. Issues 4–7 leftover sentences are gone. No NEW freeze from this revision.

Former 33 from clean-room / completeness / feasibility stay addressed; this pass did not silently reopen them.

The pack may be presented to André as implementation-ready (CHARTER D15: remaining research-note drift is not a second spec).
