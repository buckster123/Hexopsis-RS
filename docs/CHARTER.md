# text2mesh — charter

> **The decisions log below is BINDING.** Amend it with a dated entry; never silently.
> Where this document and the code disagree, one of them is a bug — say which.
> Where a later doc and D1–Dn disagree, **D1–Dn win**.
> Working name `text2mesh`. Garden name **Tessera-RS** (OQ-1 locked 2026-08-19). Cerebro product id **TESSERA**.
> Adopted 2026-08-19 from `docs/prd.md` Draft v0.1; **amended 2026-08-19 for Draft v0.2**; **amended 2026-08-19 for Draft v0.3 freeze leftovers (D1–D30 unchanged)**; **amended 2026-08-19 for Draft v0.4 OQ-1..7 lock**.

**Custody (not an implementer bibliography).** Implement from `docs/prd.md` + `docs/design.md` + this charter + Khronos glTF 2.0 + the GGUF spec + crates.io. Do **not** clone or open `src/` of AI_trellis2cpp, TRELLIS / TRELLIS.2 Python, Hunyuan3D, TripoSR, or Meshy. PRD Appendix B is **writer provenance only** — not a clone list. Research notes under `docs/research/` are not implementation specs. Full allowed/forbidden list: PRD §0.1 and design § Provenance.

## What this is

**text2mesh** is a standalone-first Rust product that turns a **still image** or a **text prompt** into a **portable glTF 2.0 GLB with core PBR materials**. One `MeshJob` schema is shared by MCP, CLI, and HTTP. Dual compute: local/onboard **and** networked, behind one `ComputePlane` trait. The text path is an original **Lattice Router + typed View Contract + Hero-Orbit** loop that feeds the **same** image-to-mesh plane — not a vendor wrapper and not a fire-and-forget T2I call.

It is a garden sibling: Cadre owns dimensioned CAD; Imaginarium owns T2I keys; OmniOcular visualizes 3D files; this product **generates** the mesh.

## What it is not

- **Not a CAD kernel** — Cadre-RS owns Starlark → B-rep → STEP. We compose or refuse.
- **Not an Imagine client** — we never hold `XAI_API_KEY`; we never link `imaginarium-slint`.
- **Not OmniOcular** — we do not steal `visualize`.
- **Not a port** of trellis2.cpp, Microsoft TRELLIS Python, Hunyuan, TripoSR, or Meshy source. Clean-room only.
- **Not a wrapper product** whose identity is someone else’s binary. A user sidecar speaking **our** `meshplane/1` is an engine, not the product.
- **Not a training stack** (Puerperium) and **not a DCC** (Blender).
- **Not Hunyuan-community default weights** and **not a GPL print-wrap binary**.
- **Not multi-tenant SaaS.** HTTP is loopback-default.
- **Not animation, rigging, Gaussian/NeRF-as-success-metric, or Limen/Quest UI** in v1. Gaussian/NeRF may ride as **optional extra artefacts** beside the GLB if an engine emits them (OQ-4 b); they are not a second success definition and not first-class DCC.
- **Not Python at runtime.** Test dumps only if a later amendment says so.

## Decisions

Numbered, binding, dated. One decision per entry, with the reason. House-briefing **OQ-1..7 locked 2026-08-19** (Resolved list). **OQ-8/9/10 remain open.**

- **D1 — Garden name Tessera-RS; working crates stay `text2mesh` until a sweep PR (2026-08-19; amended 2026-08-19 OQ-1).** Garden name locked **Tessera-RS**. Repo, crate prefix, and binaries stay `text2mesh` until a crates.io + trademark sweep PR renames to `tessera` / `tessera-mcp` (do not rename files until that PR). Cerebro product id **TESSERA** from this amendment (D16). `figment` and `loom` remain taken on crates.io.

- **D2 — Clean-room, not a port (2026-08-19; amended v0.2).** Implement from `docs/prd.md`, `docs/design.md`, public format specs (glTF 2.0, GGUF), and crates.io. Do not open or paraphrase statement-level source from AI_trellis2cpp / TRELLIS / Hunyuan / TripoSR / Meshy trees. Independent types, wire formats, file formats, crate names, and stage names. **Implementers do not follow PRD Appendix B URLs** (writer provenance only; do not clone those trees). Rules out dual-maintenance of a C++/Python surface.

- **D3 — Four-face shape (2026-08-19; amended v0.2).** v1 workspace members are **only** `text2mesh` (core) · `text2mesh-mcp` · `text2mesh-cli` · `text2mesh-api` (REST + optional HTMX WebUI). Core owns storage, traits, planner, mock, and adapters; faces stay thin. No native/Slint crate in v1. Provider/engine/io splits are **post-v1**, CHARTER amendment required. Matches Launchpad `docs/stack.md`.

- **D4 — MCP protocol pin `2024-11-05` (2026-08-19).** Hand-rolled newline-delimited JSON-RPC over stdio. **No official SDK** until a dated amendment re-opens this. stdout sacred; tracing → stderr. Tool failures = MCP `isError` results. JSON-RPC errors = protocol breakage only. Cadre D17 / OmniOcular D7 precedent.

- **D5 — Standalone-first; compose siblings, do not reimplement (2026-08-19).** ApexOS-RS is a consumer, never the owner. Zero siblings must still work via mock/thin providers and honest degrades. Cadre = Route A analytic (refuse if absent). Imaginarium = T2I/I2I when live (never our key). OmniOcular may visualize our GLB. Cerebro optional. Callosum LAN sibling is a `RemotePlane`, not a special protocol.

- **D6 — Dual compute from v1 (2026-08-19).** One `ComputePlane` trait; at least two implementations: `LocalPlane` and `RemotePlane`. `auto` is a **pure planner** over probes, not a third plane. Faces never talk to Meshy/CUDA/sidecar directly. Local ULID `job_id` is primary; `upstream_id` secondary.

- **D7 — Text path is Lattice Router + View Contract + Hero-Orbit (2026-08-19).** This is the load-bearing invention. Visual/organic prompts compile to `text2mesh.view_contract.v1`, synthesize a hero then I2I-orbit, fail-close on gates G0–G4, then call the **same** image-to-mesh plane. Naive fire-and-forget T2I is not the product. Native text-3D APIs are Route C, opt-in, never the only path.

- **D8 — Analytic never silently becomes a neural mesh (2026-08-19).** Dimensioned / mechanical prompts route to Cadre or **refuse** (`analytic.unavailable` / `analytic.too_complex`). Neural CAD requires explicit `route=view_contract` or `allow_neural_cad=true` (non-default).

- **D9 — Artefact of record is glTF 2.0 GLB + core PBR (2026-08-19; amended v0.2; amended 2026-08-19 OQ-4).** Khronos metallic-roughness. Private vertex attributes are not the material contract. Vertex-colour or factors-only is always `degraded` (`export.material_mode`) — **including preview and mock**. Default-only factors with no `COLOR_0` variation and no textures → `failed` `export.materials_missing`. Mock emits a valid GLB but terminates **`degraded`** (`disclaimer=not-a-model`). Do not label raw meshes printable/manifold unless a wrap ran. **SUCCESS is the GLB+PBR.** Gaussian/NeRF are **not** a second success definition and **not** first-class v1 contract. Optional extra artefacts **beside** the GLB are allowed when a sidecar/remote actually emits them (OQ-4 b).

- **D10 — Public quality names `preview | standard | high | ultra` (2026-08-19).** Voxel exponents stay inside an engine. `ultra` is never selected by `auto`. Requested vs achieved both recorded; step-down is `degraded`.

- **D11 — Dual license MIT OR Apache-2.0 for redistributable core (2026-08-19).** Default configure must not pull GPL (CGAL) or Hunyuan community weights.

- **D12 — Spend is gated (2026-08-19).** Paid POST never auto-fires from a default flow. Estimate is free and required first. Gate opens with `TEXT2MESH_ALLOW_SPEND=1` **or** `--allow-spend` **or** per-call `allow_spend` (MCP prefers per-call). Local $0 mesh does not need the gate; paid T2I sub-jobs do. Caps in config. Tests mock upstream.

- **D13 — Honesty and job states (2026-08-19; amended v0.2).** No fake success. Degrades stated. Missing key / missing weights / license-block fail **closed immediately**. Ban orphan status `pending`. Precise states: `queued | needs_confirm | submitted | running | waiting_upstream | succeeded | degraded | failed | cancelled`. **`ok` polarity:** mesh `ok=true` (manifest) only for `status=succeeded`. Face-wrapper `ok` means “this call parsed / job found,” never “meshed.” CLI exit **0** only for `succeeded`; exit **1** for `degraded`. `system-check` uses `report_complete` + `ready` (not `ok` for readiness). Local crash → `failed` `engine.interrupted`. Paid remote poll expiry → `waiting_upstream`.

- **D14 — Nano-first (2026-08-19; amended v0.2).** Default `cargo build` / `cargo test --workspace` succeed without ggml, CUDA, ONNX, or 14 GB weights. Heavy engines are **cargo features**. **Job** client timeouts (wait / handshake / generate) never < 30 s. **Probe/estimate** may use 5 s / 20 s / 10 s budgets and must **not** be reused for sidecar generate or vendor poll. Heartbeat stale is **minutes**, not a 30 s generate kill; a live child pid without a new progress line is **alive**. Never assume keys or 16 GB VRAM. Count **device VRAM**, not host RAM; record `shared`. Shared iGPU / `<6 GB` / `shared=true` → auto quality is preview-or-remote-or-degrade, never silent `standard`. CPU is allowed and slow. Capability query, not `#ifdef` product forks.

- **D15 — Contract first (2026-08-19).** `docs/design.md` is the wire/file/API contract. Behaviour changes update design in the **same commit**. PRD is product intent; design is implementable freeze; D* win.

- **D16 — Cerebro agent id `TESSERA` (2026-08-19; amended 2026-08-19 OQ-1).** Session memory for this repo: `agent_id="TESSERA"`, tags `project:text2mesh`. Not a runtime hard dependency. Crate/binary rename still waits for the D1 sweep PR.

- **D17 — Crate map is a requirement; internal modules are free (2026-08-19; amended v0.2).** v1 members: **only** `text2mesh`, `text2mesh-mcp`, `text2mesh-cli`, `text2mesh-api`. Core may contain rusqlite, process spawn, and HTTP adapter *modules* — that is four-face, not a sixth crate. `text2mesh-provider` / `-engine` / `-io` / `-slint` are **not v1**; splitting them needs a dated amendment. Rename of prefix follows D1 (crates.io + trademark sweep PR).

- **D18 — Single schema source (2026-08-19).** CLI JSON, MCP tool schemas, and OpenAPI generated from one Rust type layer. Drift is a CI failure.

- **D19 — Hunyuan blocked by default (2026-08-19; amended v0.2).** Local 2.1 community license excludes EU/UK/KR (Krackan is UK), MAU cap, no-train-on-outputs, HK law. No default download, no HF auto-pull. `remote.hunyuan_hosted` is **inert** unless **all** of: key, `TEXT2MESH_ALLOW_HUNYUAN=1`, 0600 territory attestation (not in git), job `license_override`. Even then, `auto` **never** picks it when Meshy, Tripo, local, or colony is feasible. Missing attestation is a structured refuse. Plane id stays unused through S10.

- **D20 — DINOv3 is opt-in (2026-08-19).** Encoder is not MIT. Planner requires `TEXT2MESH_ACCEPT_DINOV3` or `--accept-license dinov3`. File on disk with flag off → `present:true`, `accepted:false`. Outputs must carry “Built with DINOv3” when that encoder ran.

- **D21 — No GPL print wrap in garden builds (2026-08-19; OQ-7 locked defer 2026-08-19).** CGAL Alpha Wrap infects the binary. `print_wrap=true` without a non-GPL path → `license.print_wrap_unavailable`. OQ-7 is deferred: how/when a print path exists is horizon; we still never default-link GPL.

- **D22 — No telemetry (2026-08-19).** Nothing phones home.

- **D23 — Secrets (2026-08-19).** Lengths and heads only in logs/system-check. 0600 env files. This process never sees `XAI_API_KEY`. Non-loopback HTTP requires `TEXT2MESH_TOKEN`.

- **D24 — Mock is not a quality engine (2026-08-19; amended v0.2).** In-process mock always compiled (CI/Nano). Auto planner selects it only if `TEXT2MESH_ALLOW_MOCK=1`. Manifest `engine=mock`, `disclaimer=not-a-model`, **`status=degraded`**, `export.material_mode=vertex_color`. Parser-valid GLB; never `succeeded`.

- **D25 — CI from commit 0 (2026-08-19).** rustfmt + clippy `-D warnings` + test + build. Pure-fn tests; network behind traits; live tests skip loudly (`TEXT2MESH_LIVE=1`).

- **D26 — Slices, not stacked PRs (2026-08-19).** One branch = one reviewable slice off fresh `origin/main`. Never open a PR whose base is another feature branch.

- **D27 — HTTP bind `127.0.0.1:8796` (2026-08-19; OQ-6 locked 2026-08-19).** Env `TEXT2MESH_BIND`. Avoid 8791 / 8795 / 7411 / 7410 / 7320 / 8788 / 8765 / 8787. Fallback mnemonic `6374` if 8796 collides on a host.

- **D28 — v1 quality is sidecar or remote, not an in-process DiT (2026-08-19 v0.2; OQ-2 locked (c) 2026-08-19).** Stage ids (`condition` / `occupy` / `form` / `refine` / `shade` / `export`) are **manifest / meshplane progress names**, not a mandate to implement those stages in-process in S0–S11. Cargo features `quality-candle` / `quality-ggml` are **horizon, do not schedule**. Independent Rust from papers is horizon (OQ-2 a), not v1. S11 “live GLB” = fixture `meshplane/1` child, **or** paid remote with `TEXT2MESH_LIVE=1`, **or** a stated `not_configured` / `vram_short` on Krackan. Krackan field quality is remote or degrade unless a sidecar actually fits (512 MiB AMD iGPU as of 2026-08-19).

- **D29 — `ok` tokens are split (2026-08-19 v0.2).** (1) Manifest `ok=true` ⇔ `status=succeeded`. (2) Face-wrapper `ok` ⇔ call succeeded (job found / RPC parsed). (3) `system-check.ready` ⇔ `planner.would_pick != null`. (4) Wait timeout: wrapper `ok=true`, `wait_timed_out=true`, job row unchanged. (5) `POST /v1/jobs` 202 returns `job_id` + `status` + `poll_url` only — no `artifact_url` until a terminal GLB exists.

- **D30 — Four-face core owns I/O modules (2026-08-19 v0.2).** “No I/O glue” is retracted. Core owns the job store, sidecar spawn, and provider HTTP *behind traits*. Faces do not grow business logic.

## Phases

Aligned with PRD §14. Each done-when is checkable. v1 feel = S0–S11.

| Phase | Scope | Done when |
|---|---|---|
| **S0** Scaffold | Workspace, dual LICENSE, CI, CLAUDE.md, README, BACKLOG, charter, design, gotchas | `cargo test --workspace` green on default features; stub bins exist; no Launchpad placeholders |
| **S1** Types + store | MeshJob, errors, SQLite, state machine, watchdog | Persist → queued; watchdog fail path; atomic artefact commit |
| **S2** Mock + faces skeleton | Deterministic GLB; CLI/MCP/HTTP against mock | Same job JSON; mock hash pinned; status=`degraded`; allow-mock required for auto; CLI `--compute local --provider local.mock` |
| **S3** Honesty surfaces | system-check, estimate, spend gate | Missing key `not_configured` in <100 ms; gate blocks POST |
| **S4** Dual-path planner | Pure planner + HTTP mock `/v1/jobs` | MeshJob round-trips local mock **and** HTTP mock in CI |
| **S5** View Contract compiler | Types, hash, presets 4/6/8, prompt assembly | Golden contracts for 24 eval prompts (compile only); `prompts.json` **and** `identity.json` checked in |
| **S6** Hero-Orbit + gates | G0–G4, mock T2I, retry ladder, fail-closed | Eval harness offline; 3D not called on gate fail |
| **S7** Lattice + Cadre | Classifier (`classify.json` + `species.txt`); Route A refuse-if-absent | Analytic without Cadre → `analytic.unavailable` |
| **S8** Imaginarium T2I | Estimate-then-fire; no xAI key here | Wiremock green; live test skip-loud |
| **S9** Sidecar `meshplane/1` | Handshake, confinement, cancel | Fixture child writes GLB; crash → `engine.crash` |
| **S10** Remote adapters | Meshy + Tripo fixtures | 402/429 mapped; live ignored |
| **S11** Export + WebUI | glTF validate; PBR degrade; HTMX | Blender import of **mock** GLB; live GLB = fixture sidecar **or** paid remote `TEXT2MESH_LIVE=1` **or** stated Krackan degrade; amber degrade banner |
| **S12** Harden | Weights pull, license flags, idle unload | Krackan `system-check` matches reality (`vram_mb≈512`, `shared=true`, `would_pick=remote` or degrade) |

Horizon (not v1): in-process quality DiT from papers (OQ-2/OQ-3 locked; sidecar v1, hybrid runtime). Do **not** schedule `quality-candle` / `quality-ggml` in S0–S12.

## Deliberately out of v1

**Permanently out (identity)**

- CAD kernel / Starlark authoring / OCCT (Cadre)
- Training / distillation (Puerperium); using Hunyuan outputs as synthetic data
- Full DCC, mesh sculpting as modelling medium
- Wrapping a C++ port as the product
- Hunyuan community weights as default
- Multi-tenant public SaaS
- Stealing OmniOcular `visualize`; holding `XAI_API_KEY`
- Python/PyTorch runtime dependency
- Linking GPL Slint or CGAL into garden binaries
- Telemetry

**Out of v1, honestly deferred**

- In-process 4B-class quality engine (horizon)
- Gaussian / NeRF as first-class artefacts or a second success metric (OQ-4 b: extras-allowed beside the GLB if an engine emits them; GLB+PBR still defines success)
- Animation, rigging, skinning, USD
- Watertight print wrap (OQ-7 deferred)
- Native multi-image 3D as a second job type (views already exist)
- Native/Slint viewer; `text2mesh-slint` / `-provider` / `-engine` / `-io` crates
- Streamable HTTP MCP
- In-process occupy/form/refine as a v1 slice (D28)
- Underside cameras / unlocked per-view lighting
- Burn tensor core
- Limen-RS / Quest consume

## Resolved questions

House briefing OQ-1..7. Locked 2026-08-19 by André. Not a silent close.

1. **OQ-1 Product/crate name — (a) Tessera-RS.** Garden name locked **Tessera-RS**. Working crate prefix / binaries stay `text2mesh` until a crates.io + trademark sweep PR renames to `tessera` / `tessera-mcp`. Cerebro product id **TESSERA** from this amendment (D16). Tags stay `project:text2mesh`. (b) Figment-RS — crates.io collision. (c) Loom-RS — collision. (d) keep `text2mesh` as garden name — rejected.

2. **OQ-2 Default local quality engine — (c) sidecar v1 + (a) as horizon.** S9 implements sidecar; D28 forbids treating in-process occupy/form/refine as a v1 dependency. Independent Rust from papers is horizon, not v1. `quality-candle` / `quality-ggml` stay unscheduled. (b) in-process ggml in v1 — rejected.

3. **OQ-3 Inference runtime — (a) hybrid.** Default none; sidecar quality; candle horizon; `quality-ggml` named C exception default off; ONNX for small encoders; Burn not v1. Preview candle vs ONNX wait until a MIT weight is wired. Nano must build with none of them **regardless**. (b) ggml-only / (c) Burn-only — rejected as v1 exclusive.

4. **OQ-4 Gaussian/NeRF — (b) optional extras.** GLB+PBR remains the SUCCESS definition. Gaussian/NeRF may be stored as extra artefacts beside the GLB if a sidecar/remote actually emits them. Not a second success metric. Not first-class DCC. (a) mesh-only-exclusive — rejected. (c) first-class v1 — rejected.

5. **OQ-5 Default View Contract camera count — (b) 6 `cardinal4_hero_top`.** S5 still ships all three presets (4/6/8); compiler **default is 6**. (a) 4 / (c) 8 as default — rejected (8 remains a quality-tier preset).

6. **OQ-6 HTTP bind port — (a) 8796.** Locked as D27. Env `TEXT2MESH_BIND`. (b) 6374 fallback if 8796 collides. Avoid garden-occupied ports.

7. **OQ-7 Watertight/print — (a) defer.** D21 stays: never default-link GPL. (b) never-on `print-cgal` feature — still a footgun, not scheduled. (c) pure-Rust wrap research — horizon.

## Open questions

None that block S0–S12. House OQ-1..7 locked 2026-08-19. Executive lock of OQ-8..10 same day (GROK, André granted executive power):

- **OQ-8 CLIP thresholds — locked.** Ship `gate_version=g0_v0` defaults in design §6. First field eval may retune ±0.04 without a schema bump if `gate_version` is recorded. No silent threshold drift.
- **OQ-9 I2I billing — locked.** Always call `T2iProvider::estimate`. Never hardcode 2× in core. `usd_uncertain=true` when the catalog has no I2I unit.
- **OQ-10 Distinct `degraded` — locked (D13/D29).** Terminal `degraded` is not `succeeded`+`degrades[]`. Manifest `ok=true` only for `succeeded`. CLI exit 1 for `degraded`.

## Amendments

Dated entries. A decision changes here first, then in the code.

- **2026-08-19** — Charter adopted from `docs/prd.md` Draft v0.1 and `docs/design.md` v0.1 at clean-room PRD stamp. D1–D27. OQ-1..7 open with Recommended labels. S0 not yet coded.
- **2026-08-19** — Draft v0.2 review pass. D2 Appendix-B custody; D3/D17 four-crate lock; D9/D24 mock+vertex-colour `degraded`; D13/D29 `ok` split; D14 probe-vs-job clocks + VRAM honesty; D19 auto never Hunyuan; D28 sidecar/remote quality, stages as progress names; D30 core owns I/O modules. S2/S11/S12 done-whens amended. D1–D30 now bind.
- **2026-08-19** — Draft v0.3 freeze leftovers (`docs/reviews/verify.md` Issues 1–7). Wait/wall min 30 default 1800 max 86400 on MCP, CLI, Route B; G0–G4 vs `canonical_view_id`; G2 FACE/BACK pair only; G0 text = identity_phrase else normalized; fail-closed G1–G4; frozen `prompt_suffix` + class locks + negatives; watchdog pid-live (no silence-kill); drop `TEXT2MESH_CUSTOM_*` / no `remote.custom`; confirm + artifact `?kind=` includes `contract`; eval fixtures in PR-10/11/13/21. **D1–D30 unchanged.**
- **2026-08-19** — Executive lock OQ-8/9/10 (GROK): CLIP ±0.04 via `gate_version`; I2I always estimate; distinct `degraded` terminal (already D13/D29). S0 scaffold started.
- **2026-08-19** — Draft v0.4. André locked OQ-1..7. D1 garden name **Tessera-RS** (crates/binaries stay `text2mesh` until a crates.io + trademark sweep PR); D16 `agent_id="TESSERA"`, tags `project:text2mesh`; D9 extras-allowed, GLB still defines success; D27 locks OQ-6 `127.0.0.1:8796`; D28 locks OQ-2 sidecar v1 + papers-as-horizon; D21 stays, OQ-7 defer; OQ-3 hybrid runtime; OQ-5 six cameras `cardinal4_hero_top`. OQ-1..7 moved to Resolved; OQ-8/9/10 remain open. Wait clocks, gates, and prompt suffixes unchanged.
