# House doctrine & sibling compose — extract for text2mesh PRD

**Date:** 2026-08-19  
**Role:** clean-room research extract (not a CHARTER; not a PRD).  
**Allowed inputs:** Launchpad-RS `docs/house-doctrine.md` + `docs/stack.md`; OmniOcular-RS / Cadre-RS `docs/CHARTER.md`; DocSmith PRD **§0 + ToC only**; workspace `docs/research/BRIEFING.md`.  
**Forbidden:** reference-project source (trellis2.cpp, TRELLIS Python, Hunyuan/Tripo/Meshy implementations). Prefrontal confirmed **no existing garden text2mesh / generative-mesh project**.

Items marked **[suggested]** are recommendations for CHARTER/PRD writers, not locked decisions. House rules below are **binding unless CHARTER amends them**.

---

## 1. Four-face shape

Canonical workspace (Launchpad `docs/stack.md`; OmniOcular D3):

```
crates/
  <name>/         # core lib — ALL logic: types, storage, engines. No I/O glue.
  <name>-mcp/     # MCP-over-stdio binary — the agent face
  <name>-cli/     # clap CLI — the human/ops face
  <name>-api/     # axum REST + optional dashboard — the management face
```

**One capability, several faces.** The lib owns logic; each face is a thin adapter. That is what makes standalone-first work: MCP / CLI / REST consumers adopt with **zero colony dependency**.

Smaller shapes exist in the garden (one binary with subcommands; two crates; single crate) but this product has three co-equal callers plus jobs + optional WebUI, so **four-face is the preferred S0 shape**. Optional WebUI lives **inside `-api`** (lean HTMX; OmniOcular D12), not as a fifth required crate. A native/Slint viewer is **not** v1 unless CHARTER argues it; Cadre already has an orbit mesh viewer, OmniOcular owns `visualize`.

**Standalone-first (OmniOcular D5, stack.md):** ApexOS-RS is the richest consumer, **never the owner**. Core capabilities work with **zero siblings**, via thin providers and honest degrades.

---

## 2. MCP pin

House MCP (stack.md; OmniOcular D7; Cadre D17):

| Pin | Value |
|-----|--------|
| Protocol | `"2024-11-05"` |
| Transport | Hand-rolled newline-delimited JSON-RPC over **stdio** |
| SDK | **No official MCP SDK** until a dated CHARTER amendment re-opens it (Cadre closed OQ-7: stay hand-rolled) |
| stdout | **Sacred** — JSON-RPC only. All `tracing` → **stderr** |
| Notifications | `notifications/*` → skip response entirely; echo request `id` exactly |
| Frames | Per-frame parse isolation + size cap (Cerebro precedent: 32 MiB) |
| Tool failure | MCP `isError` **result** with helpful text. JSON-RPC *errors* = protocol breakage only |
| Honesty | Unimplemented surface → honest "not yet", never fake success |
| Registration | `~/Projects/.mcp.json` points at **`target/release/…`** — rebuild after CLI/MCP surface changes |
| Cadre extra | Tool-description budget ≤ 4,000 tokens (D12); deep docs as resources. Consider for this product if the tool list grows |

Streamable HTTP MCP is **optional** (Cadre keeps stdio + streamable HTTP in `-mcp` without the SDK). v1 can be stdio-only if HTTP is the REST face.

---

## 3. Nano-first

Inherited from ApexOS-RS; stack.md tiers; OmniOcular D9.

**Build UI and defaults for the smallest tier.** Faster tiers get the same behaviour, just faster.

| Rule | Concrete |
|------|----------|
| No timeout **< 30s** | Local CPU mesh jobs and remote polls must not use impatient client timeouts |
| Never assume keys | Missing Imagine / Meshy / Tripo key is a **stated degrade**, not a hang |
| Heavy models **feature-gated** | Default `cargo build` excludes ONNX / ggml / CUDA / GGUF runtimes (Occipital pattern: opt-in cargo feature, not only a runtime toggle) |
| Embeddings off on Nano | If used at all, empty model id → off; FTS5 fallback |
| System deps probed | ffmpeg, GPU runtimes, sibling binaries — `system-check` tells the truth |
| CPU is allowed and honest | Slow is valid; do not pretend GPU |
| Capability query, not `#ifdef` product forks | CUDA / ROCm / Vulkan / Metal via probe |

Inference runtime (candle / burn / ggml-via-FFI / onnxruntime) is a CHARTER open question; **Nano must still build without it**.

---

## 4. Spend is gated (doctrine #8 + #9)

Paid operations — API credits, GPU rental, image/video generation, hosted mesh APIs — **never auto-fire from a default flow**.

- Tests mock upstream. Live-fire is explicit, counted, and André's call.
- Free preflight (`check_credits`, `estimate`) runs **before** paid fire.
- OmniOcular D8: opt-in via config flag, CLI confirm, and/or preflight estimate tool.
- **INSTALLED ≠ ACTIVE** (Imaginarium key-isolation invariant): a provisioned integration stays inert until its key is present, and says so.
- Sibling keys live **only** in that sibling's env file (0600). This product's daemon/UI/MCP **never** see `XAI_API_KEY` when composing Imaginarium.

**Doctrine #9 — cost the failure, not the happy path:**

- A **paid** job that outlives its poll window stays **`pending` with a recoverable task id**, not silently `failed`. Never orphan spend.
- That does **not** license fake pending: missing key, missing weights, and refused licenses fail **closed** immediately (`failed` + structured reason). Briefing: missing key ≠ timeout; jobs must not wedge as `pending` on paths that never fired.

---

## 5. Honesty invariants (doctrine #3)

- Never report a fake success. `{ok: false}` with a real reason beats a 200 that means nothing.
- **Degrades are stated, not masked.**
- Check the **body**, not just HTTP status. Delivery confirmed by `ok`, not by `200`.
- Never silently clamp a value you could honestly reject.
- A job is never stuck `pending` on a local/error path — every failure path flips to `failed` with the reason (paid-poll exception in §4).
- Field truth beats green CI (doctrine #5): a slice is done when a real job produces a real GLB (or a stated degrade), not when tests pass.
- `system-check` reports engines, keys (length/head only), VRAM/RAM, licenses — distinct structured errors for missing weights / missing key / CPU-only.

Secrets (doctrine #6): lengths and heads only; never in repo/transcript/CLAUDE.md; 0600 env files (`/etc/<name>/env` when daemonized).

---

## 6. Sibling compose boundaries

**Rule (OmniOcular D5 + briefing §6):** compose, do not reimplement. Prefer a live sibling and advertise it in `system-check`. When absent, standalone provider **or** stated gap — never a hard dependency.

| Sibling | Owns | This product may | This product must not |
|---------|------|------------------|------------------------|
| **Cadre-RS** | Agent CAD: hermetic Starlark → B-rep (OCCT) → numeric inspect → STEP/GLB; orbit mesh *viewer*; slicer orchestration. Cerebro id `CADRE`. Viewer port **7411**, API **7410**. | **Analytic route:** dimensioned / mechanical / CAD-shaped prompts → compose Cadre (if present) to STEP/GLB. Optional later inspect/remesh. Honest refuse if Cadre absent. Reuse *ideas* from `write_gltf_json` + orbit viewer, **not** crate-level coupling in v1. | Not a CAD kernel. Not Starlark authoring. Not OCCT (Cadre's sanctioned FFI). Not mesh sculpting as modeling medium (Cadre NG4). Not organic generation. |
| **Imaginarium-RS** | xAI Imagine gateway: image/video gen, **key isolation**, library, jobs, studio. Bind **8791**. Spend-gated. | **View-contract T2I/I2I** when Imaginarium is live (submit views, poll artifacts). Standalone T2I provider if absent. Treat Imaginarium as a `ComputePlane`/`ImageProvider`, not a special case. | Not an Imagine client fork. Never hold `XAI_API_KEY`. Never auto-fire paid Imagine from a default mesh flow. Not a general image studio. |
| **OmniOcular-RS** | Multimodal *tools* (vision, audio, video, documents, **visualize 3D files**, search glue). Does **not** generate 3D. Bind **8795**. Compose *out* to Imaginarium for gen. | May hand a finished GLB to OmniOcular `visualize` when present. May use OmniOcular vision tools as optional inspect helpers. | **Do not steal `visualize`.** Not a multimodal toolkit. Not OCR/transcribe/PDF. Generation orchestration stays here; OmniOcular may *call* us later, we do not absorb its tool catalog. |
| **CerebroCortex-RS** | Colony memory. API **8765**. | Optional `session_recall` / `session_save` / `remember`. | Never a hard dep. Not a second brain. |
| **ApexOS-RS** | Agent runtime / soul. Gateway **8787**. | Consumer of our MCP. | Never owner. No Apex-only protocol. |
| **Limen-RS / Quest** | Spatial UI | Downstream mesh consumers. | Out of v1. |
| **Puerperium-RS** | Training / fine-tune | — | Not a training stack. |
| **Occipital-RS / Sonus-RS** | Web fetch / music | Irrelevant to v1 mesh. | Do not reimplement. |
| **Callosum-RS** | Colony mesh. Bind **8788**. | LAN sibling as a **provider** (same job schema), not a special case. | Do not take 8788. |
| **Prefrontal-RS** | Garden search / dashboard. Bind **7320**. | Ask Prefrontal before inventing tools that may exist. | Do not take 7320. |

**This product owns:** text and image → **3D mesh** (textured GLB + provenance). Dual compute (local onboard + networked) behind one `ComputePlane` / one `MeshJob` schema. View Contract compiler + consistency loop. Planner `auto` with stated degrade. MCP + CLI + HTTP sharing that schema.

**v1 non-goals (from briefing; CHARTER may tighten):** not CAD, not training, not DCC, not wrapping trellis2.cpp as the product, not Hunyuan-community default weights, not animation/rigging, not multi-tenant SaaS.

---

## 7. DocSmith PRD section list (methodology template)

DocSmith (`Scriptum-RS/DocSmith-PRD-Iterated-Opus-123/docsmith-prd.md`) is the garden clean-room PRD shape. **Use the section list; do not copy DocSmith requirements.**

| § | Title |
|---|--------|
| 0 | Clean-Room Methodology & Provenance |
| 1 | Summary |
| 2 | Background & Problem Statement |
| 3 | Goals |
| 4 | Non-Goals |
| 5 | Personas |
| 6 | Functional Requirements — *(engine; here: mesh job plane / local+remote compute / View Contract)* |
| 7 | Functional Requirements — Agent Tool Surfaces (MCP / CLI / API) |
| 8 | Non-Functional Requirements |
| 9 | Proposed Architecture *(original design)* |
| 10 | Security & Trust Model |
| 11 | Milestones |
| 12 | Success Metrics |
| 13 | Open Questions & Risks |
| 14 | Glossary |
| App. A | Example Tool Schemas *(illustrative, original)* |
| App. B | Sources Reviewed *(public URLs only)* |

### §0 rules to copy as *practice*, not prose

- Consult public README / architecture overviews / papers / LICENSE / model cards / hosted-API docs.
- Do **not** open, copy, or paraphrase statement-level source from the reference trees.
- Requirements restate **capabilities**, then §9 invents an independent mechanism.
- Implementers' custody: this PRD + public format specs (glTF 2.0, GGUF) + standard crates — not C++/Python reference trees.
- Depth: public self-description, not black-box binary probing. Mark product judgment **[inferred]**.
- Working name distinct from the reference; trademarks un-cleared until §13.

---

## 8. Suggested crate map **[suggested]**

Crate map is a Cadre D16-class **requirement** (named members); internal module design stays free. S0 keeps a thin facade so the workspace resolves from commit 0; slices split into named crates rather than growing a monolith.

**Crate prefix:** `text2mesh` until the D1 sweep. Garden name closed **Hexopsis-RS** (2026-08-19); Tessera-RS vacated. Rename of crates is a dated amendment, not silent drift.

| Crate | Role | S0? |
|-------|------|-----|
| `text2mesh` | Core lib: `MeshJob`, planner/`auto`, View Contract types, `ComputePlane` trait, provenance/license flags, structured errors. No I/O glue. | **yes** |
| `text2mesh-mcp` | Stdio MCP face (`2024-11-05`) | **yes** |
| `text2mesh-cli` | clap CLI; `--json`; `system-check`; `mcp` / `api` launchers | **yes** |
| `text2mesh-api` | axum REST + optional HTMX job UI; OpenAPI from the same types | **yes** |
| `text2mesh-provider` | Networked adapters (Meshy/Tripo/OpenAPI-ish, LAN sibling). Trait impls only. | split when the second adapter lands |
| `text2mesh-engine` | Local/onboard inference. **Cargo-feature gated**; default workspace tests do not link it. Sanctioned FFI named in CHARTER if ggml-class. | split at first local-engine slice |
| `text2mesh-io` | glTF/GLB write + sidecar JSON; no generator math | optional split if I/O grows |
| `text2mesh-slint` | Native viewer | **not v1** — compose OmniOcular/Cadre |

Shared schema rule (Cadre D13): CLI JSON, MCP tool schemas, and OpenAPI **generated from one Rust type layer**; drift is a CI failure.

Workspace scaffold (stack.md): `CLAUDE.md`, `README.md`, `BACKLOG.md`, dual `LICENSE`, `.gitignore`, `.github/workflows/ci.yml` (fmt + clippy `-D warnings` + test + build) **from commit 0**, rustfmt-clean baseline, `docs/CHARTER.md`, `docs/design.md`, `docs/gotchas.md`, optional `deploy/<name>.service`, optional banner.

Crate defaults to reuse, not rediscover: `tokio`, `reqwest` (rustls), `axum` 0.8, `clap` derive, `serde`+`serde_json` with `#[serde(default)]` caution, `tracing`→stderr, `rusqlite`+FTS5 if a job store is local-file, `subtle` for tokens. GPU/viewer: `wgpu`+WGSL only if a local viewer is in scope. Pure Rust preference; **named** C/FFI exceptions only.

License: **MIT OR Apache-2.0** for redistributable core (OmniOcular D13, Cadre D6). Default configure must not pull GPL (CGAL alpha wrap) or Hunyuan community weights.

---

## 9. Suggested HTTP port **[suggested]**

**Recommend `127.0.0.1:8796`** (`TEXT2MESH_BIND` / later `<NAME>_BIND`).

Avoid (task + garden occupancy):

| Port | Occupant |
|------|----------|
| **8791** | Imaginarium-RS API + studio |
| **8795** | OmniOcular-RS API + WebUI |
| **7411** | Cadre-RS viewer (`CADRE_VIEWER_PORT`) |
| **7410** | Cadre-RS API |
| **7320** | Prefrontal-RS (`prefrontald`) |
| **8788** | Callosum-RS mesh face (ApexOS snapshot also documented on 8788 in places — still forbidden) |
| 8765 | CerebroCortex-RS API |
| 8787 | ApexOS-RS agentd / PWA |
| 8888 / 2739 | ApexRouter proxy / control |
| 8792 / 8793 | Callosum pairing examples in design docs |

**8796** sits in the garden `879x` API band (8791 Imagine, 8795 OmniOcular) and was **not** found in sibling CHARTER/design/README/service files. Default **loopback**; non-loopback bind requires a token (Imaginarium / Cerebro / OmniOcular pattern). CHARTER OQ-6 in the briefing is this choice — lock it as a D* when accepted.

Mnemonic fallback if 8796 collides on a host: **6374** ("MESH" on a phone keypad), still avoiding the banned set.

---

## 10. Cerebro agent id **[suggested]**

Cadre D15 is the pattern: isolate session memory under a shouty product id.

| Recommend | `TEXT2MESH` |
|-----------|-------------|
| Why | Repo is `text2mesh`; garden name still OQ-1. Matches `CADRE`. |
| If OQ-1 picks Figment / Tessera / Loom | Amend CHARTER in the **same** dated entry as crate/binary rename (`FIGMENT` / `TESSERA` / `LOOM`). Do not silently fork ids. |
| Usage | `agent_id="TEXT2MESH"` on `session_recall` / `session_save` / `remember` / `cognitive_bootstrap` for this repo. Tag memories `project:text2mesh`. |
| Hard dep? | No. Cerebro is optional (doctrine #7 is for agents working *on* the repo, not for the runtime product). |

GROK (this researcher) stays `agent_id="GROK"` for harness work; the **product** id above is what CHARTER locks for implementers.

---

## 11. Other house pins writers must not silently drop

- **Contract first** (doctrine #1): `docs/design.md` before code; behaviour + docs in the same commit.
- **Slices, not marathons** (doctrine #2): one branch = one reviewable slice off fresh `origin/main`. **Never stacked PRs.**
- **Pure-fn tests** (doctrine #4): request builders, parsers, planners, contract compilers unit-tested; network behind traits; live tests skip **loudly**.
- **Locked means locked:** `## Locked decisions` / CHARTER D* are not re-derived each session. Amend with a date.
- **Ask Prefrontal first** before writing a tool that might already exist.
- Dual compute from v1: one trait, ≥2 impls (local + networked). Planner `auto`: probe local (weights, VRAM/RAM, license) → else remote if keys → else **stated degrade**.

---

## 12. Sources for this extract

- `/home/andre/Projects/Launchpad-RS/docs/house-doctrine.md`
- `/home/andre/Projects/Launchpad-RS/docs/stack.md`
- `/home/andre/Projects/OmniOcular-RS/docs/CHARTER.md`
- `/home/andre/Projects/Cadre-RS/docs/CHARTER.md`
- `/home/andre/Projects/Scriptum-RS/DocSmith-PRD-Iterated-Opus-123/docsmith-prd.md` §0 + table of contents
- `/home/andre/Projects/Clean-Room/text2mesh/docs/research/BRIEFING.md`
- Port occupancy: sibling CHARTER/design/README/service docs + Prefrontal `docs/ideas/colony-panel.md` (2026-08-03 survey). No reference-project source opened.
