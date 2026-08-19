# text2mesh — orchestrator research briefing (2026-08-19)

This is **not** the PRD. It is a fact pack for PRD-writing agents. Do not copy
 trellis2.cpp source, C ABI, or internal formats. Treat public capability
 descriptions as *functional targets*, then invent original architecture.

Workspace: `/home/andre/Projects/Clean-Room/text2mesh` (empty greenfield).
Operator: André (buckster123). Agent: GROK. House: Launchpad-RS.

---

## 0. Task as given

Clean-room PRD for **text and image → 3D mesh**.

- Target language(s): **Rust + supporting languages** (C/C++/WGSL/Python *only* as named, sanctioned exceptions — never the product).
- Must work against **networked endpoints** *and* **local/onboard compute/inference**.
- Image-to-3D *example* (capability, not a port): https://github.com/RobertBeckebans/AI_trellis2cpp
- The **text2 layer needs a candidate or an invention** (not a placeholder).
- Workflows sub-agent approved at all PRD stages.

---

## 1. Clean-room rules (house precedent: DocSmith, OmniOcular, Cadre)

**Consulted (allowed):** public README, architecture *overview* docs, papers, LICENSE/NOTICE, Hugging Face model cards, hosted-API public docs.

**Forbidden:** opening/copying/paraphrasing statement-level source from:

- `RobertBeckebans/AI_trellis2cpp` (and upstream `rms80/trellis2cpp`)
- Microsoft `TRELLIS` / `TRELLIS.2` Python trees
- Hunyuan3D, TripoSR, Meshy, etc. implementation source

**Depth note:** this briefing comes from public self-description, not black-box probing of a compiled binary. Mark product-judgment items **[inferred]**.

**Recommended custody for implementers:** this PRD + public format specs (glTF 2.0, GGUF spec) + standard crates. Do not read the C++/Python reference trees.

---

## 2. Image-to-3D public capability (functional target)

### 2.1 trellis2.cpp README (public)

- Image in → 3D mesh with PBR attributes out; runtime **C++/ggml**, no PyTorch.
- Export: portable **GLB** (vertex colour + retained PBR; optional UV atlas).
- Backends publicly named: CPU, CUDA, Vulkan, HIP/ROCm, Metal (ggml).
- Quality tiers publicly named: coarse 64³ preview, 512³ fine, 1024³ cascade (stated TRELLIS.2 default), optional 1536³.
- Demo server: job store, browser viewer, device switch, GLB download.
- Optional: watertight print wrap (CGAL Alpha Wrap — **GPL**), quad remesh, normal-map bake.
- Weights: public GGUFs (~14 GB f16) under MIT (TRELLIS.2-4B, TRELLIS-image-large) + DINOv3 license for the encoder GGUF.
- License of the C++ port: MIT. Optional CGAL path is GPL.

### 2.2 trellis2.cpp architecture README (public pipeline *shape*, not code)

Five conceptual moves publicly described:

1. Condition: vision transformer (DINOv3 ViT-L/16) → patch tokens.
2. Sparse structure: DiT → 64³ occupancy (where is there material).
3. Shape SLAT: second DiT on active voxels → sparse decoder → 512³ dual-grid fields.
4. Optional cascade to 1024³ / 1536³.
5. Extract mesh + texture branch (PBR: base colour, roughness, metallic, opacity).

Publicly stated principles we may *aspire to independently*, never copy:

- Stage isolation (each stage independently runnable/testable).
- Compact library + hosts (CLI / server / viewer).
- No hardwired GPU vendor.
- Measure before changing defaults.
- Mesh counters are not an accuracy metric.

**Do not** adopt their container names (`.t2mesh`, `.dinodata`), C ABI (`t2_*`), Go server, or ggml graph layout.

### 2.3 Microsoft TRELLIS (v1) public README — MIT

- Paper: *Structured 3D Latents for Scalable and Versatile 3D Generation* (arXiv:2412.01506, CVPR 2025).
- **Text or image** → radiance fields, 3D Gaussians, **and meshes**.
- Public models: TRELLIS-image-large (1.2B); TRELLIS-text-base/large/xlarge (0.34B–2.0B).
- **Authors' own recommendation (verbatim idea, not code):** text-to-3D is *better* done by generating images with a T2I model, then running the **image-conditioned** 3D model. Native text-conditioned models are “less creative and detailed due to data limitations.”
- Multi-image conditioning exists as a tuning-free algorithm (not a separately trained model).
- Hardware note: NVIDIA ≥16 GB in the Python demo.

### 2.4 Microsoft TRELLIS.2 public card

- Paper: *Native and Compact Structured Latents for 3D Generation* (arXiv:2512.14692, Dec 2025).
- **Image-to-3D**, MIT weights (`microsoft/TRELLIS.2-4B`).
- Higher-fidelity geometry + materials vs v1; native 3D latents (not 2D-feature SLAT).
- Image-only at the public model card; text is not advertised as a first-class TRELLIS.2 condition.

---

## 3. Text-to-3D candidate landscape (public, 2026)

| Candidate | Text? | Image? | Local weights | License posture (public) | Notes |
|---|---|---|---|---|---|
| TRELLIS v1 text models | yes | yes | yes | MIT | Authors recommend T2I→image-3D instead |
| TRELLIS.2 | no (public) | yes | yes (GGUF exist) | MIT | Best *open* image-to-mesh quality class |
| Hunyuan3D 2.1 | often via T2I | yes | yes (heavy VRAM) | **Community license** — no EU/UK/KR; MAU cap; no training on outputs | Do **not** default-vendor |
| Hunyuan hosted 3.1 | yes | yes | no | vendor ToS + geo limits | Networked option only |
| TripoSR | no | yes | yes, fast (~0.5s A100) | research/open (Stability+Tripo) | Preview/Nano path |
| Tripo AI API | yes | yes | no | paid API | Networked option |
| Meshy API | yes | yes | no | paid API | Networked option |
| Cadre-RS (garden) | analytic CAD | n/a | n/a | MIT/Apache + OCCT LGPL engine | Mechanical/dimensioned prompts |
| Imaginarium-RS (garden) | T2I/I2I | n/a | n/a | MIT/Apache | Paid xAI Imagine; spend-gated |

**License landmine:** Hunyuan 2.1 community license is not garden-safe as a default local engine.

---

## 4. Invention seed — do not treat as locked; writer must own it

Working name for the text layer (writer may rename): **Lattice Router + View Contract**.

Three routes, one job object:

1. **Analytic** — prompt is dimensioned / mechanical / CAD-shaped → compose **Cadre-RS** (if present) to STEP/GLB. Not a neural mesh. Honest refuse if Cadre absent.
2. **View Contract** (recommended default for visual/organic text) — compile the prompt into a **multi-view contract** (subject lock, camera ring, lighting, background/alpha, negatives, seed), synthesize N consistent views (Imaginarium or local T2I), then run the **same image-to-3D plane** used by the image path. Matches TRELLIS authors' public advice, but the *compiler + consistency loop + provenance* is original.
3. **Native text-3D** — optional provider when a user has a text-conditioned engine or a hosted API. Never the only path.

Why this is an invention, not “call Flux then Trellis”:

- The contract is a typed artifact (cameras, identity, lighting) that can be inspected, edited, and replayed.
- Consistency is a first-class loop with a retry budget and fail-closed quality gates — not a fire-and-forget T2I.
- The router is prompt-class aware (CAD vs creature vs product photo) and composes siblings instead of reimplementing them.
- The same `MeshJob` runs on **local** or **remote** compute without the caller changing schema.

---

## 5. Dual compute (hard requirement)

One `ComputePlane` trait, at least two implementations from v1:

- **Local/onboard:** models on disk + a local engine (preview feedforward and/or quality DiT). CPU allowed and honest (slow). GPU via capability probe (CUDA/ROCm/Vulkan/Metal) — **capability query, not `#ifdef` product forks**.
- **Networked:** HTTP(S) providers with the same job schema (submit → poll → artifact). Meshy/Tripo/custom OpenAPI-ish adapters. Colony sibling over LAN is a provider, not a special case.

Planner `auto`: probe local (weights present, VRAM/RAM, license flags) → else remote if keys exist → else **stated degrade**, never fake success.

Doctrine: spend gated; jobs never stuck `pending`; missing key ≠ timeout.

---

## 6. Garden — compose, do not reimplement

| Sibling | Owns | text2mesh relationship |
|---|---|---|
| **Cadre-RS** | Agent CAD (Starlark → B-rep → STEP/GLB, mesh viewer) | Analytic route; optional inspect/remesh later. Not organic generation. |
| **Imaginarium-RS** | xAI Imagine image/video, key isolation, library, jobs | View-contract T2I/I2I when live. Standalone T2I provider if absent. |
| **OmniOcular-RS** | Multimodal *tools* (visualize 3D files, not generate them) | May visualize our GLB; we do not steal `visualize`. |
| **CerebroCortex-RS** | Memory | Optional session notes; never a hard dep. |
| **ApexOS-RS** | Agent runtime | Consumer of our MCP, never owner. |
| **Limen-RS / Quest** | Spatial UI | Downstream mesh consumers, out of v1. |

Standalone-first (OmniOcular D5 pattern): every core capability works with **zero siblings**, via thin providers + honest degrades.

Prefrontal: **no existing text2mesh / generative-mesh project.** Cadre has `write_gltf_json` + orbit mesh viewer — reuse *ideas*, not couple at crate level in v1.

---

## 7. Launchpad-RS house constraints (binding unless CHARTER amends)

- Contract first (`docs/design.md` before code).
- Slices, not marathons; PRs off `main`, never stacked base-on-base.
- Honesty invariants: no fake success; stated degrades; jobs fail closed.
- Pure-fn tests; network behind traits; live tests skip loudly.
- Field truth beats green CI.
- Secrets: lengths/heads only; 0600 env files.
- Spend gated. Estimate before paid fire.
- Four-face shape preferred: `core` / `mcp` / `cli` / `api` (+ optional WebUI).
- MCP: hand-rolled JSON-RPC stdio, protocol `2024-11-05`, no SDK, stdout sacred.
- Nano-first: no timeout < 30s; never assume keys; heavy models feature-gated.
- Pure Rust preference; **named** C/FFI exceptions only (e.g. ggml/candle backend, OCCT is Cadre's problem not ours).
- Dual license MIT OR Apache-2.0 for redistributable core.
- CI from commit 0; rustfmt-clean baseline.
- Cerebro agent id to lock in CHARTER (suggest `TEXT2MESH` or product name).

Scaffold expected at S0: CLAUDE.md, README, BACKLOG, LICENSE, CI, CHARTER, design, gotchas, optional banner.

---

## 8. Suggested product shape (writer may improve)

**Working product name:** keep repo `text2mesh`; recommend a garden name in CHARTER OQ-1 (candidates: Figment-RS, Tessera-RS, Loom-RS). Do not bikeshed the whole PRD on naming.

**v1 outcomes a user can feel:**

1. Drop an image → textured GLB (local *or* remote).
2. Type a prompt → View Contract preview (the N views) → same GLB pipeline.
3. `system-check` tells the truth: which engines, which keys, which VRAM, which licenses.
4. MCP + CLI + HTTP share one schema.
5. A job manifest records: prompt/image hash, contract, engine, device, timings, licenses, spend.

**v1 non-goals (seed):**

- Not a CAD kernel (Cadre).
- Not a training/fine-tune stack (Puerperium).
- Not a full DCC (Blender).
- Not wrapping trellis2.cpp as the product (optional user sidecar speaking *our* job protocol is fine).
- Not Hunyuan weights as default (license).
- Not animation/rigging as v1.
- Not multi-tenant SaaS.

---

## 9. Supporting languages (what “Rust+supports” means)

| Language | Allowed role in v1 |
|---|---|
| Rust | Product: core, faces, job plane, mesh I/O, planner |
| WGSL | Optional local viewer (wgpu), not required for headless |
| C ABI | Optional *outbound* stable ABI so game engines can call us; our design, versioned |
| Python | **Test/reference dumps only** if a paper-parity gate is in scope; never a runtime dep |
| C/C++ | Sanctioned FFI to an inference runtime (ggml/llama.cpp-class) if CHARTER names it |
| JS/HTML | Lean WebUI (HTMX or tiny canvas viewer), no SPA-as-product unless argued |

---

## 10. Quality / success seeds

- Image path: GLB imports in Blender/three.js; materials not dropped; provenance in sidecar JSON.
- Text path: for a fixed eval set of N prompts, View Contract produces N views that pass a stated consistency gate more often than naive single-image T2I→3D (writer must pick a measurable gate — CLIP pairwise, pose, or human rubric).
- Dual path: same `MeshJob` JSON round-trips local mock engine and HTTP mock provider in CI.
- Honesty: missing weights / missing key / CPU-only are distinct, structured errors.
- License: default configure never pulls GPL (CGAL) or Hunyuan community weights.

---

## 11. Open questions the PRD must not silently close

1. Product/crate name.
2. Default local quality engine: (a) independent Rust reimplementation from papers + public GGUF layout we define, (b) user-supplied sidecar, (c) both with (b) as v1 and (a) as horizon.
3. Inference runtime: candle / burn / ggml-via-FFI / onnxruntime — pick with rationale; Nano must still build without it.
4. Whether Gaussian/NeRF outputs are v1 or mesh-only.
5. Default View Contract camera count (4 vs 6 vs 8).
6. HTTP bind port (check garden: Imaginarium 8791, OmniOcular 8795, Cadre view 7411 — pick a free one).
7. Watertight/print path: defer (GPL) vs pure-Rust alpha wrap research.

---

## 12. Sources reviewed (public URLs only)

- https://github.com/RobertBeckebans/AI_trellis2cpp (README)
- https://raw.githubusercontent.com/RobertBeckebans/AI_trellis2cpp/main/docs/architecture/README.md
- https://github.com/microsoft/TRELLIS (README; not source)
- https://microsoft.github.io/TRELLIS/
- https://huggingface.co/microsoft/TRELLIS.2-4B
- https://arxiv.org/abs/2412.01506 and https://arxiv.org/abs/2512.14692 (papers)
- https://github.com/Tencent-Hunyuan/Hunyuan3D-2 / 2.1 (README/license commentary, not source)
- https://github.com/VAST-AI-Research/TripoSR (README)
- Launchpad-RS `docs/house-doctrine.md`, `docs/stack.md`
- OmniOcular-RS `docs/CHARTER.md`
- Cadre-RS `docs/CHARTER.md`, `docs/VIEWER.md`
- Imaginarium-RS `docs/ARCHITECTURE.md`
- DocSmith clean-room PRD methodology (`Scriptum-RS/DocSmith-PRD-Iterated-Opus-123/docsmith-prd.md` §0)
