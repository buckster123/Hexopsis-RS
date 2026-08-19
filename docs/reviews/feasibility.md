# text2mesh — feasibility & house-fit review

| Field | Value |
|---|---|
| **Status** | addressed (v0.2) |
| **Date** | 2026-08-19 |
| **Lens** | Feasibility + Launchpad house-fit. Docs only; no rewrite. |
| **Read** | `docs/prd.md` Draft v0.1, `docs/CHARTER.md`, `docs/design.md`, Launchpad `docs/stack.md` + `docs/house-doctrine.md`, research notes as needed |
| **Field probe** | Krackan 2026-08-19: AMD Radeon 840M (gfx1152, `1002:1114`), **512 MiB** reported VRAM, **22 GiB** host RAM, **no NVIDIA** |

Do not treat research notes as the contract. CHARTER D* win.

---

## Hunt results

| Hunt | Result |
|---|---|
| v1 secretly requires a from-scratch TRELLIS.2 reimplementation before any demo | **Partial.** S2 mock + S9 sidecar + OQ-2(c) correctly keep a 4B DiT **out** of the first demo. FR-IMG-20 + `quality-*` features + S11 “live GLB” still slope implementers into an in-process TRELLIS-class graph. → Issue 2 |
| Unrealistic VRAM claims | **Hit.** Planner floors are 16 / 24 GB NVIDIA-class; the field box is a 512 MiB AMD iGPU. Disk floor 12 GB < 14–17 GB quality stack. → Issue 1 |
| Stacked-PR traps | **Hit.** Plan *says* fresh `origin/main`; PR-07 is a three-face join and several deps are over-serial. → Issue 5 |
| GPL pulled in by default | **Clear.** D11 / D21 / OQ-7(a); no `print-cgal` in the cargo table; Slint / CGAL / Hunyuan kept off default configure. |
| Crate map vs four-face | **Partial.** S0 members match Launchpad four-face. Mid-v1 splits + a `text2mesh-slint` row invite a sixth crate before the four-face is boring. → Issue 6 |
| Timeouts &lt; 30 s | **Partial.** Wait / handshake / generate **floors** are 30 s. Probe 5 s / 20 s and estimate-refresh 10 s are sub-30 client budgets. Heartbeat stale at 30 s is a de-facto job kill. → Issues 3, 7 |
| ApexOS ownership | **Clear.** D5: ApexOS-RS is a consumer, never the owner. No Apex-only protocol. Port 8787 avoided. Zero-sibling mock path exists. |
| Missing mock engine for CI | **Clear.** D24 / FR-CMP-13 / design §18 / PR-03: always compiled, deterministic GLB, `TEXT2MESH_ALLOW_MOCK` for auto, HTTP mock in S4, mock T2I in S6. |
| PR Plan that is not sliced | **Mostly clear.** §19 is 23 titled PRs with files + deps, not one marathon branch. The failure mode is **join slices**, not a missing plan. → Issue 5 |

---

## Issues

### Issue 1: Planner VRAM/disk floors are not the field machine

- Severity: major
- Section: PRD §6.4, §8.8, FR-IMG-18; design §7.2 floors + §14 `seconds_p50`; CHARTER D14; PRD manifest example §13.1
- Description: Official TRELLIS.2-4B is **≥24 GB NVIDIA**. TRELLIS v1 is **≥16 GB NVIDIA**. TripoSR-class preview is **~6 GB**. The planner table still uses `local standard` = 16 GB warn / 24 GB comfortable, disk **≥12 GB**, and FR-IMG-18 / FR-IMG-5 make **standard** the default local pick on “modest VRAM.” Design §14 even catalogs `standard 16 GB → 180 s`. Krackan today: **AMD Radeon 840M, 512 MiB `mem_info_vram_total`, 22 GiB RAM, no `nvidia-smi`.** Community 8 GB / 6 GB offload is already labelled unofficial — good — but the **normative floor is still 16 GB**, and the example manifest shows `gpu.vulkan` / `AMD Radeon` as the device that actually ran a quality job. Two concrete traps: (1) a probe that treats **shared host RAM** as VRAM will select `standard` and OOM the box; (2) a probe that reports the **512 MiB carve-out** will `vram_short` every local quality pick, which is honest — but then S11 / M10 / S12 “Krackan field truth” cannot be a local TRELLIS-class mesh. Disk **12 GB** is also below the same notes’ 14–17 GB (safetensors stages ≈ 16.2 GB). D14 says never assume 16 GB; the planner still does.
- Suggestion: Freeze **one number** per pick (`need_mb`), not “16 warn / 24 comfortable.” Default `auto` quality on a shared iGPU / `<6 GB` / `shared=true` is **preview-or-remote-or-degrade**, never silent standard. Count **device VRAM**, not host RAM; record `shared`. Raise the quality-stack disk floor to **≥16 GB** (or “sum of named files × 1.1”). Replace the AMD-Vulkan quality example with an honest Krackan row (`vram_mb≈512`, `would_pick=remote|degrade`, `slow=true` if CPU). State in S11/M10 that Krackan live quality is **remote or stated degrade** unless a sidecar actually fits.
- Status: addressed
- Response: One number per pick: preview 6144 / standard+high 24576 VRAM; disk standard **16384**. Shared iGPU never silent standard. system-check example is 512 MiB shared Radeon with `ready=false`. S11/M10/S12 Krackan = remote or degrade. D14 amended.

### Issue 2: v1 still slopes into an in-process TRELLIS.2 graph

- Severity: major
- Section: PRD FR-IMG-20 / §6.5 / §10.1 / §11.5 / S11; CHARTER OQ-2, S9, horizon note; design §23 `quality-candle` / `quality-ggml`
- Description: The **first demo is not blocked** on a from-scratch 4B DiT — S2 mock and S9 fixture sidecar are the right house move, and NG-D1 / OQ-2(c) say so. The v1 **quality** story still is. FR-IMG-20 requires a graph of named stages we **own** (`condition / occupy / form / refine / shade / export`), each with a typed artefact that “can run alone in tests.” §11.5 draws that graph as the local sidecar interior. Cargo features for v1 already name `quality-candle` and `quality-ggml`. S11’s done-when is “mock **+ one live GLB**.” There is **no first-party engine** that can produce that live GLB on Krackan (Issue 1). Path of least resistance: implement the owned stages in-process, or turn `quality-ggml` on and ingest a community GGUF graph — i.e. a TRELLIS.2 reimplementation (or a wrap) **before** a quality demo. Sidecar-as-product-boundary only works if v1 quality is allowed to be **user binary or paid remote**, and the owned-stage graph is **progress vocabulary**, not a core implementation mandate.
- Suggestion: Amend FR-IMG-20 / §6.5: stage ids are **manifest / meshplane progress names**; in-process implementations are **horizon** (OQ-2/OQ-3), not S0–S11. Strike `quality-candle` / `quality-ggml` from the v1 feature table (keep them in a “horizon, do not schedule” note). Define S11 “live GLB” as **one of**: fixture `meshplane/1` child, paid remote with `TEXT2MESH_LIVE=1`, or a **stated** `not_configured` / `vram_short` on Krackan. Do not accept an in-process occupy/form/refine slice as a v1 dependency.
- Status: addressed
- Response: FR-IMG-20/D28: stages are progress names. `quality-candle`/`quality-ggml` struck from v1 Cargo table (horizon_unscheduled). S11 live GLB = fixture sidecar | LIVE remote | stated degrade.

### Issue 3: 30 s heartbeat stale is a de-facto generate timeout

- Severity: major
- Section: PRD FR-CMP-26, §13.2; design §8.1, §22 `TEXT2MESH_HB_S=30`, §24 “sidecar generate: heartbeat, no short timeout”; CHARTER D14; Launchpad stack “no timeout shorter than 30s”
- Description: D14 / NFR-4 forbid **client** timeouts &lt; 30 s on wait / handshake / generate. The watchdog still flips local `running` → `failed` `engine.crash` when the last heartbeat is **older than 30 s**, and §13.2 demands a line every **≤5 s**. Sidecar generate is described as “no short timeout,” but a TRELLIS-class (or even preview) kernel often goes **silent for minutes**. Nested Route B is worse: parent stays `running` while T2I children work (design §8.2); if the director does not emit parent heartbeats, the parent dies at 30 s of silence. That is Nano-first in name only — it is a 30 s generate kill unless every engine is chatty.
- Suggestion: Heartbeat stale must be **≫ generate floor** (minutes, not 30 s), or the watchdog must treat “child process still alive + no new line” as **alive**. Require director heartbeats while waiting on T2I / sidecar children. Keep 30 s only for **handshake** and **client wait minimum**. Document that missing progress lines are not a crash if `pid` is live.
- Status: addressed
- Response: Local crash only if **pid dead**. `TEXT2MESH_HB_S` default **300**. Director heartbeats while waiting on children. 30 s remains handshake + wait floor only.

### Issue 4: Nano 180 s / default 600 s walls cannot finish Hero-Orbit

- Severity: major
- Section: PRD FR-TXT-19, §13.2 latency table; design §3 `max_wall_s: 600`, §24 Nano Route B 180 s / default 600 s
- Description: The same docs say Hero-Orbit Default-6 hosted T2I is **minutes-class before 3D**, hosted T2I is 5–30 s **per view**, remote mesh poll is 30 s–5 min, CPU quality is **hours-class**. Retry ladder adds up to 2 hero resamples + 3 orbit edits + 1 reseed. Nano wall **180 s** cannot cover 1 hero + 5 I2I + gates + any 3D. Default **600 s** is tight even on the happy path (≈6×20 s + 90 s mesh ≈ 210 s) and loses on vendor slowness or retries (6+3 images × 30 s + 5 min mesh ≈ 570–750 s). There is no spec for **how Nano is detected** so the 180 s cap applies. Offline appliance persona (§5.4) plus Route B as the visual default will produce `wait.timeout` / wall fails that look like product bugs.
- Suggestion: Nano 180 s applies to **preview mock / single-image preview**, not Default-6 Route B. Route B `max_wall_s` default **≥ 1800 s** (or no wall except `recover_ttl` on paid remotes). Say how Nano is detected (compile features / `system-check` tier / env), or drop the 180 s number. Estimate `seconds_p50` must include N T2I + reserved retries + mesh, not mesh alone.
- Status: addressed
- Response: Route B default `max_wall_s=1800`. Nano 180 s = mock/single-image preview only. Nano = `system-check` tier (no sidecar feature, no quality weights, vram missing/`<6144`/`shared`). Estimate `seconds_p50` includes T2I+retries+mesh.

### Issue 5: PR-07 is a stacked-PR join; several deps are fake serial

- Severity: major
- Section: PRD §19 PR Plan; CHARTER D26; house doctrine #2
- Description: The plan correctly says deps are merge order, not stacked bases, and it **calls out** PR-17/PR-18. The dangerous junction is **PR-04 · 05 · 06 → PR-07**. PR-07 is “core + all faces,” deps `PR-04..06`. That is the garden’s stacked-PR failure mode: an integration branch that never reaches `main` under squash-merge, or a marathon André cannot review. PR-13 **Lattice** depends on PR-12 Hero-Orbit even though `classify` is a pure function over PR-01 types. PR-15 Imaginarium depends on PR-12; the `T2iProvider` trait does not. PR-16 sidecar depends on PR-09 planner; handshake + fixture child can follow PR-03. Twenty-three PRs **are** slices; the join edges are not written as “merge to main, then branch again.”
- Suggestion: Split PR-07 into core `system-check` (after PR-01/PR-03) + thin per-face wiring PRs that each depend only on **merged** core. Add the same “do not stack” sentence to PR-04/05/06 as PR-17/18. Repoint PR-13 → PR-01 (or PR-03). Repoint PR-15 → T2I trait + PR-08 spend. Repoint PR-16 → PR-03 + process spawn. Keep a one-line rule in §19: after every merge, `git fetch && git checkout -b … origin/main`.
- Status: addressed
- Response: PR-07 split into 07a + 07b/c/d. PR-04/05/06 do-not-stack. PR-13 → PR-01; PR-15 → PR-08; PR-16 → PR-03. §19 fetch/checkout rule.

### Issue 6: Mid-v1 crate splits fight four-face

- Severity: minor
- Section: PRD §11.3 crate map; CHARTER D3 / D17; Launchpad `docs/stack.md` four-face
- Description: S0 members `text2mesh` / `-mcp` / `-cli` / `-api` match the house. D17 then schedules `text2mesh-provider` at the second adapter (PR-18), `text2mesh-engine` at the first local-engine slice (PR-16), optional `-io`, and a **`text2mesh-slint` row** in the same table. Stack.md is four faces; extra crates are a deliberate deviate. Splitting engine/provider in the same v1 that is still proving the four-face will produce a six-crate workspace before CLI/MCP/HTTP are boringly thin. Core is also described as “**No I/O glue** beyond traits” while S1–S9 put rusqlite, process spawn, and HTTP adapters in that crate — fine for four-face, confusing next to a promised `-engine` / `-provider` split.
- Suggestion: Lock v1 workspace to the **four S0 members**. Move provider/engine/io/slint to “post-v1 split, CHARTER amendment.” Delete or footnote the slint row so it is not a crate to scaffold. If core contains store + mock, say “four-face core owns storage and traits; faces stay thin” and drop “no I/O glue.”
- Status: addressed
- Response: D3/D17/D30: four crates only. Slint/provider/engine/io post-v1. “No I/O glue” retracted.

### Issue 7: Probe and estimate still use client timeouts under 30 s

- Severity: minor
- Section: NFR-4; design §7 probe, §24 table; Launchpad stack.md “no timeout shorter than 30s”
- Description: NFR-4 carves **wait / handshake / generate**. House stack wording does not. Remaining sub-30 client budgets: device probe **5 s each / 20 s total**, estimate remote refresh **10 s**, `system-check` **&lt; 20 s**. Those are reasonable **probe** kills (missing key must not wait 30 s — D13). They are still timeouts &lt; 30 s and will be copied onto generate/poll by accident if the table is the only thing implementers read. Combined with Issue 3, the 30 s number is overloaded (handshake floor, wait minimum, heartbeat kill, probe cap).
- Suggestion: Split the table into **probe budgets** (5/20 s, kill → `unavailable`) vs **job budgets** (≥30 s). One sentence: probe/estimate are the only sub-30 s timers; they must not be reused for sidecar generate or vendor poll. Align NFR-4 with stack.md by naming the probe exception in CHARTER D14.
- Status: addressed
- Response: design §24 split probe vs job. D14 + NFR-4 name the probe exception.

### Issue 8: Design silently defaults destructive export flags on

- Severity: minor
- Section: PRD FR-IMG-8 (all export flags default **off**); design §3 MeshJob `unit_cube: true`, `uv_atlas: true`
- Description: FR-IMG-8 says destructive cleanup is **opt-in** (`unit_cube`, `uv_atlas`, `keep_largest_component`, `force_opaque` all default off). The frozen MeshJob example turns `unit_cube` and `uv_atlas` **on**. That is a silent recenter/rescale and a texture-atlas demand on every job, including mock and Cadre GLB. House honesty: do not silently transform what you could refuse or gate. Feasibility: mock and Route A should not fail or degrade because an implicit `uv_atlas=true` was not honoured.
- Suggestion: Make the design example match FR-IMG-8 (`false` / off). If `unit_cube` is required for viewer convenience, it is a **stated** default with a CHARTER line, not a silent disagreement.
- Status: addressed
- Response: Design MeshJob `unit_cube`/`uv_atlas` false. Destructive export stays opt-in. No silent CHARTER default-on.

---

## Cleared (do not reopen without new evidence)

- **GPL default:** MIT OR Apache-2.0 core; no default CGAL feature; `print_wrap` fail-closed; do not link `imaginarium-slint`; Hunyuan blocked (UK). OQ-7(b) remains a named footgun — keep it out of `Cargo.toml`.
- **ApexOS ownership:** D5 + sibling table. MCP registration in `~/Projects/.mcp.json` is garden convention, not assimilation.
- **Mock for CI:** Always compiled; golden hash; auto-select forbidden without `TEXT2MESH_ALLOW_MOCK`; dual-path CI is local mock + HTTP mock (S4), not live weights.
- **First GLB demo:** S2/PR-03 is enough for a Blender-importable **mock**. Do not move that done-when onto an in-process DiT.

---

## Summary

House shape is mostly right: four-face S0, sidecar-not-port, mock always on, spend gated, ApexOS not owner, GPL/Hunyuan off the default path, PR plan actually sliced. Feasibility on **this** host is not. Krackan is a **512 MiB AMD iGPU**; the planner still talks 16–24 GB NVIDIA standard. v1 will demo on **mock + remote (or degrade)** or it will slide into a TRELLIS.2-shaped in-process engine to satisfy “live GLB.” Watchdog 30 s and Nano 180 s will kill the jobs the text path is proud of. Fix floors, stage ownership, job clocks, and the PR-07 join before coding S1.

**Verdict:** `needs-revision`

---

## Revision Summary (2026-08-19, PRD Draft v0.2)

All 8 issues **addressed**. Planner floors are one number (24 GB VRAM / 16 GiB disk for standard); Krackan 512 MiB shared iGPU → remote or degrade. Stages are progress names; `quality-*` unscheduled; S11 live GLB is fixture/remote/degrade. Heartbeat is pid-live + 300 s. Route B wall 1800 s. PR-07 split; four crates only; probe vs job timeout tables; export flags default off.
