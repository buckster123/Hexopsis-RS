# Image-to-3D — functional target (clean-room)

**Status:** research note for PRD writers, not a PRD and not an implementation plan.  
**Date:** 2026-08-19  
**Scope:** the *image → textured mesh* plane only. Text-to-3D is a separate note; it must *reuse this plane*, not fork it.  
**Posture:** treat public capability descriptions as a **functional bar**. Invent original architecture, names, formats, and hosts. Do not port a reference tree.

Items that are product judgment, unmeasured, or extrapolated from public self-description (not black-box probing) are marked **[inferred]**.

Custody for implementers: this note + Khronos glTF 2.0 / PBR specs + GGUF spec + garden doctrine. **Do not read** the C++/Python reference trees listed in §9.

---

## 1. What the user can feel

Drop one image. Get back a **portable GLB** that opens in Blender / three.js / a browser viewer with **geometry plus PBR materials**, not a colour-only blob.

That is the v1 image-path outcome. Everything below exists to make that job **honest** on local CPU, local GPU (any vendor that actually probes), or a networked provider — same job object.

**Non-goals for this plane (v1 seed):**

- Not a CAD kernel, not animation/rigging, not a training stack, not multi-tenant SaaS.
- Not wrapping a C++ sidecar *as the product*. A user-supplied sidecar that speaks **our** job protocol is an allowed *engine*, not the identity.
- Not Gaussian / NeRF as the primary artefact **[inferred — CHARTER open question]**. Mesh + PBR GLB is the contract. Extra representations may ride as optional sidecar blobs later.
- Not Hunyuan weights as a default local engine (license; §7).
- Not a watertight 3D-print kernel as default (GPL trap; §7).

---

## 2. Inputs

### 2.1 Required

| Field | Functional requirement |
|---|---|
| **Image** | One still. Public systems advertise PNG/JPEG (and typically RGBA). Alpha is useful: subject-on-transparent is the cleanest condition. |
| **Job id** | Server/CLI/MCP minted. Caller never invents persistence keys. |

**[inferred]** Accept at least PNG and JPEG; reject non-images with a structured error (not a timeout). Optional WebP if the decoder crate is already a garden dep. Do not silently convert video frames.

### 2.2 Optional job knobs (product, not copied internals)

| Knob | Meaning |
|---|---|
| **Quality** | One of `preview` / `standard` / `high` / `ultra` (§4). Never a raw voxel exponent in the public schema. |
| **Seed** | Reproducible sampling when the engine supports it. Omit → engine draws; **record the seed that actually ran**. |
| **Compute** | `auto` \| a probed device id \| `remote:<provider>`. `auto` is a planner, not a vendor `#ifdef`. |
| **Export** | GLB always. Optional extras: keep-largest-component, opaque-force (no alpha), unit-scale, UV-atlas vs vertex colour. Destructive cleanup is **opt-in**, never default. |
| **Print wrap** | Off by default. If requested and the engine lacks a non-GPL wrap, **fail closed** with `license.print_wrap_unavailable` — do not silently skip. |

### 2.3 Conditioning class (public facts we must not over-claim)

- **TRELLIS.2** public card: **image-to-3D only**. Text is not a first-class condition of that model.
- **TRELLIS v1** public README: text *or* image; authors still recommend T2I → image-conditioned 3D. Multi-image exists as a **tuning-free** algorithm, not a separately trained model.
- **TripoSR** public README: single image, feedforward, fast preview class.

**[inferred]** v1 image plane is **single-image**. Multi-view is the *text* path's View Contract (N synthesised views fed into this same plane). Native multi-image conditioning is a later engine capability, not a second job type.

### 2.4 Preprocess (capability, not a recipe)

Public demos crop to the subject, respect alpha, and resize to the encoder's expected square. **[inferred]** We own a small, pure preprocess: decode → optional alpha-aware crop/pad → hash the **bytes we actually conditioned on** (not the original filename). Record both hashes in the manifest.

---

## 3. Outputs — GLB + PBR

The artefact of record is **glTF 2.0 binary (.glb)** as specified by Khronos, not a private mesh container.

### 3.1 Geometry

- Triangle mesh (indexed primitives).
- **[inferred]** Place the asset in a **centred unit cube** (glTF +Y up). Record the transform we applied; do not silently bake a mystery scale.
- Topology honesty: public TRELLIS.2 card admits **small holes / minor discontinuities** in raw meshes; watertight is a post-process, not a generation guarantee. We **must not** label a raw mesh "printable" or "manifold" unless a wrap stage actually ran and reported success.

### 3.2 PBR (Khronos core metallic-roughness)

Target the **core** glTF 2.0 metallic-roughness model, which importers actually honour:

| Channel | glTF home | Why |
|---|---|---|
| Base colour | `pbrMetallicRoughness.baseColorFactor` / `baseColorTexture`; optional `COLOR_0` | Albedo / metal reflectance colour. Vertex colour is a legal core multiplier. |
| Metallic | `metallicFactor` / `metallicRoughnessTexture` (B) | Metal vs dielectric. |
| Roughness | `roughnessFactor` / `metallicRoughnessTexture` (G) | Microfacet blur. |
| Opacity | `baseColorTexture` A + `alphaMode` (`OPAQUE` \| `MASK` \| `BLEND`) | Public TRELLIS.2 note: GLB often ships `OPAQUE` even when an alpha map exists; transparency must be **explicitly enabled**, not assumed. |
| Normal (optional bake) | `normalTexture`, MikkTSpace tangents | glTF-prescribed tangent basis so Blender/three.js agree. |
| Occlusion / emissive | optional core textures | Not required for v1 if the engine does not emit them. |

**Do not** make a private vertex attribute the *contract* for metallic/roughness. If an engine only emits per-vertex PBR, **our** exporter still writes **standard** glTF materials (factors, or a packed metallic-roughness texture we generate). Extra attributes, if any, are additive and documented in the sidecar.

**[inferred] v1 export policy**

1. Always write a GLB that validates as glTF 2.0.
2. Prefer **UV atlas + textures** when the engine can bake; otherwise **vertex colour + material factors** is an honest degrade, recorded as `export.material_mode = vertex_color`.
3. Never drop materials silently. If PBR is missing, the job is `degraded` or `failed`, not `succeeded` with a grey mesh.
4. Sidecar JSON (same stem as the GLB) holds provenance (§8). The GLB must still be useful **without** the sidecar.

### 3.3 What success looks like

- GLB imports in Blender and three.js.
- Base colour visible; metallic/roughness affect a lit preview.
- Alpha, if present, is either wired (`BLEND`/`MASK`) or the manifest says it was forced opaque.
- A human can rotate it in the optional viewer and download the same bytes the API returns.

Mesh triangle counts are **not** an accuracy metric (public architecture principle we may independently adopt). Pretty ≠ faithful.

---

## 4. Quality tiers — product names

Public systems advertise voxel-grid resolution classes. **Those numbers are not our API.** The public schema exposes **product quality names**. A private mapping table may exist inside an engine; it must not leak into MCP/CLI/HTTP enums.

| Product name | User-facing promise | Planner notes **[inferred]** |
|---|---|---|
| **Preview** | Seconds-class silhouette. Good enough to check "is this the right object?" Not a shipping asset. | Coarse occupancy / marching-cubes class. Allowed when shape weights are missing; still labelled Preview, never High. |
| **Standard** | Full geometry + PBR at the "fine" public class. Default **local** pick when VRAM/RAM is modest. | Corresponds to the publicly named 512³ fine tier in TRELLIS.2 / ggml-port READMEs. |
| **High** | Default **quality** pick when the engine and device can take it. Refinement pass on top of Standard. | Corresponds to the publicly named 1024³ cascade, stated TRELLIS.2 default. |
| **Ultra** | Explicit opt-in maximum. Slow, memory-heavy. **Never** selected by `auto`. | Corresponds to the publicly named 1536³ class. Public port notes: requested vs achieved may differ (budget step-down); both must be reported. |

**Planner `auto` [inferred]:** Preview if only coarse weights exist; else Standard on small VRAM; High when weights + VRAM headroom exist; **never Ultra**. If the user asked High and the engine steps down, status is `degraded` with `requested_quality` / `achieved_quality`.

Public speed *illustrations* (not our SLA, not copied as targets): TRELLIS.2 card quotes ~3 s / ~17 s / ~60 s on an NVIDIA H100 at the three fine/cascade classes. A ggml-port README quotes ~110 s for the fine class on a 16 GB RTX 50-series and minutes + ~10 GB VRAM for the cascade class. **[inferred]** We publish *our* measured times per device in `system-check`, never their numbers.

---

## 5. Backends — capabilities, not product forks

One compute plane. At least two implementations from v1: **local/onboard** and **networked**. The caller does not change schema.

### 5.1 Local capabilities (probe, don't `#ifdef` the product)

Public ggml-class runtimes name these backends. We treat them as **capability bits**:

| Capability id | Meaning |
|---|---|
| `cpu` | Always available. Honest and slow. Allowed. |
| `nvidia.cuda` | NVIDIA GPU via CUDA-class runtime. |
| `amd.rocm` | AMD GPU via HIP/ROCm. |
| `gpu.vulkan` | Cross-vendor GPU via Vulkan (NVIDIA/AMD/Intel/…). |
| `apple.metal` | Apple GPU. |

`system-check` reports: which bits probed true, driver/runtime versions, VRAM/RAM, which weight files are on disk, which licenses those files carry.

Public hardware notes (context, not requirements):

- TRELLIS v1 Python demo: NVIDIA, ≥16 GB.
- TRELLIS.2 Python card: Linux-tested; NVIDIA ≥24 GB; verified A100/H100.
- ggml-class port: CPU + the GPU vendors above; Windows/Linux scripts exist; CUDA graphs/ROCm quirks are *their* bugs, not ours to copy.

**[inferred]** Nano/CI builds with **zero** GPU runtime and **zero** weights. Local engine is feature-gated. Missing GPU ≠ crash; it is `compute.cpu_only` plus an estimate.

**[inferred]** Device switch is a *runtime* capability: drop resident weights, next job reloads on the chosen backend. Record `compute.requested` and `compute.actual`.

### 5.2 Networked capabilities

Same `MeshJob`. Submit → poll → artefact.

- Hosted image-to-3D APIs (Meshy, Tripo, Hunyuan *hosted* 3.1, Azure Foundry TRELLIS, …) as **adapters**.
- A colony sibling on LAN is just another provider.
- Spend gated: estimate before paid fire; missing key is a structured error, not a hang.

**[inferred]** Remote may not expose Preview/Ultra the same way. Adapter maps *our* quality names onto provider knobs and writes the mapping into the manifest. If the provider cannot honour PBR, the job is `degraded` with `remote.material_fidelity`.

### 5.3 Engine slot (CHARTER will pick)

Functional target is **weights on disk + an inference runtime**, not "link trellis2.cpp". Options the PRD must choose, not this note: independent Rust from papers + a GGUF layout *we* define; user sidecar; or sidecar-now / reimplementation-horizon. Nano must still build without the runtime.

Public weight packs relevant to the *bar* (licenses in §7):

- `microsoft/TRELLIS.2-4B` — MIT, image-to-3D, 4B, 512³–1536³ class.
- `microsoft/TRELLIS-image-large` — MIT, 1.2B, image-to-3D (v1 SLAT).
- LocalAI-io GGUF conversions of the above — MIT, ~14 GB f16 set advertised for the ggml-class port.
- DINOv3 ViT-L/16 GGUF — **not MIT** (§7). Conditioning encoder.

TripoSR (MIT, ~0.5 s on A100, ~6 GB VRAM, vertex colour or baked texture) is a legitimate **Preview/Nano** engine class, not a High-quality substitute.

---

## 6. Job and server behaviour

Functional behaviour observed in public demos, restated as **our** contract. Hosts (CLI, MCP, HTTP, optional WebUI) share one schema.

### 6.1 Lifecycle

```
submit → queued → running → { succeeded | degraded | failed }
                 ↘ cancelled
```

Invariants (house doctrine, binding):

- Jobs **never stick on `pending`**. A process crash leaves them `failed` with `engine.interrupted`, or they resume from a durable checkpoint *we* design.
- Incomplete writes are **ignored** on startup (atomic commit of artefact + manifest, or nothing).
- Missing weights, missing key, CPU-only, license-blocked feature, OOM, user cancel are **distinct error codes**.
- `degraded` is a success-shaped result with an explicit reason (quality step-down, vertex-colour materials, remote without PBR). The UI must not draw a green tick without the reason.
- No fake success. A grey untextured mesh is not a textured GLB.

### 6.2 Persistence

Completed jobs land under a store directory: GLB, sidecar manifest, hashed input image, optional preview frames. Restart restores **the same job ids**. `-store ''` (or equivalent) is ephemeral.

**[inferred]** Manifest is JSON we specify. Minimum fields: image hash (raw + conditioned), quality requested/achieved, engine id + version, device actual, seed, timings per *our* stage names, licenses of every weight file loaded, spend (0 for local), export mode, error/degrade codes.

### 6.3 Server / viewer (optional face)

Public demo shape we may *aspire to*, with original code:

- Browser: drop image → quality + device → live-or-polled progress → orbit viewer → download GLB.
- Idle unload: start without allocating model VRAM; load on first job; release when the queue is idle (Nano-friendly).
- Regenerate-from-saved-image with current settings (new job id, link `parent_job`).

The Go demo, its port, its routes, and its embedded viewer are **not** ours. HTTP bind port is a CHARTER open question (garden: Imaginarium 8791, OmniOcular 8795, Cadre view 7411).

### 6.4 CLI / MCP

Same job object. Headless must work with zero siblings. MCP: hand-rolled JSON-RPC stdio, protocol `2024-11-05`, stdout sacred. `system-check` is a first-class tool: engines, keys (lengths/heads only), VRAM, licenses.

---

## 7. License and GPL traps

Garden redistributable core: **MIT OR Apache-2.0**. Default configure **must not** pull GPL or Hunyuan community weights.

### 7.1 Safe-ish defaults (still record them)

| Piece | Public license | Use as default? |
|---|---|---|
| Microsoft TRELLIS / TRELLIS.2 **weights + majority code** | MIT | Yes, as a *weight* option. Not as a source tree to copy. |
| ggml-class C++ port | MIT | Sidecar-only if CHARTER allows; do not vendor as the product. |
| LocalAI-io TRELLIS GGUFs | MIT (inherited) | Weight pack option. |
| TripoSR | MIT (README) | Preview-class engine option. |
| glTF 2.0 spec | Khronos | Yes — the export contract. |

### 7.2 Traps

**CGAL 3D Alpha Wrapping — GPL-3.0-or-later** (or GeometryFactory commercial). Public print-wrap paths that link CGAL infect the **binary**. Default configure must not detect-and-enable this by accident. A v1 print path is either deferred, feature-gated behind an explicit `--features print-cgal` that we probably **never ship in garden builds**, or a future pure-Rust wrap. TRELLIS.2's own card offers hole-filling scripts as post-process; that is still not a license to copy them.

**DINOv3 License (Meta, 2025-08-14)** — the public conditioning encoder. Not MIT.

- Redistribution must include the Agreement and **prominently display "Built with DINOv3"**.
- Trade-control / ITAR / military-end-use restrictions.
- No reverse-engineering of underlying components (their clause).
- Official HF repo is **gated** (contact info). GGUF redistributions exist under the same license.
- Litigation against Meta terminates the license.

`system-check` and the README must surface this. A DINOv3-free Preview engine (or a differently licensed encoder) is an honesty path if the user cannot accept Meta's terms **[inferred]**.

**Hunyuan3D 2.0 / 2.1 Community License** — **not garden-safe as a default local engine.** Confirmed from the public LICENSE files:

- Territory **excludes EU, United Kingdom, and South Korea**. Use, distribution, and even *outputs* outside the Territory are unlicensed.
- If Licensee's products exceed **1 million MAU**, a separate Tencent license is required.
- **No using Works or Outputs to improve any other AI model** (distillation / synthetic-data training ban).
- Hosted-service attribution rules; Hong Kong law.

Hosted Hunyuan 3.x may still be a **networked adapter** behind user ToS acceptance — never silently pulled, never the local default.

**Other copyleft / extra licenses to keep off the default path:** Eigen MPL-2.0 if a remesh stage needs it (file-level copyleft; do not vendor); nvdiffrast / nvdiffrec (TRELLIS.2 README: separate licenses, CUDA-era Python stack — not our runtime).

### 7.3 Honesty rules

- `system-check` lists every weight file and its SPDX-or-named license.
- Planner refuses a path that would mix GPL into a garden binary unless the user passed an explicit, documented flag **and** we decide CHARTER even allows that flag.
- Output sidecar repeats the licenses that produced it (DINOv3 attribution string included when that encoder ran).

---

## 8. What we must independently design

Public *principles* we may aspire to. The **mechanisms** are original.

### 8.1 Stage isolation

A generation is a graph of **named stages we own**, each loadable and runnable alone:

1. **Condition** — image → vision tokens.  
2. **Occupancy** — coarse "where is there material".  
3. **Shape** — fields / latents on occupied cells → extractable geometry.  
4. **Refine** (optional) — High/Ultra cascade.  
5. **Shade** — PBR maps conditioned on image + shape.  
6. **Export** — mesh + materials → GLB + sidecar.

Each stage: typed input artefact, typed output artefact, a test that does not boot the rest of the pipeline. **[inferred]** Our artefact names and on-disk formats are new (e.g. versioned tensors in a layout we specify, or ephemeral memmaps). We do **not** adopt reference container names or C ABI.

Parity, if ever in scope: independent reference dumps (CPU) vs our stage; never a shared backend that can cancel a bug. Python is test-only, never a runtime dep.

### 8.2 Provenance

Provenance is **what actually ran**, not what the UI selected.

Record: conditioned-image hash, encoder id+license, weight file hashes, sampler seed, requested vs achieved quality, backend that executed, attention/precision mode if the engine has one, timings, git/crate version, export mode. A backend selector string that disagrees with the library that ran is a **bug**, not a log line.

### 8.3 Honesty

- Distinct errors for missing weights / missing key / CPU-only / license-blocked / OOM / cancel / remote 4xx/5xx.
- `auto` degrades with a sentence a human can read.
- Mesh stats are diagnostics, not pass/fail.
- Spend estimate before any paid provider call.
- Feature-gated heavy models; Nano timeouts never < 30 s.

### 8.4 Compact library + interchangeable hosts

Rust `core` owns jobs, planning, mesh I/O, engine traits. Faces: `cli`, `mcp`, `api`, optional WebUI. Optional **outbound** C ABI *we version* for game engines — designed here, not cloned. Inference runtime (candle / burn / ggml-via-FFI / onnxruntime) is a CHARTER pick; the trait hides it.

### 8.5 No hardwired GPU vendor

Capability query at start of job. Workarounds for vendor bugs, if we hit them, are measured, documented as *our* bugs/, and gated — not copied from another project's chunk sizes.

### 8.6 Dual compute

`ComputePlane` trait: `LocalEngine` and `RemoteProvider`. Planner `auto`: weights present ∧ license ok ∧ VRAM/RAM ⇒ local; else keys ⇒ remote; else stated degrade.

---

## 9. Sources (public only)

Consulted for this note. No source trees, no binaries probed.

- trellis2.cpp README + LICENSE (MIT): <https://github.com/RobertBeckebans/AI_trellis2cpp>
- trellis2.cpp architecture *overview* (pipeline shape, not code): <https://github.com/RobertBeckebans/AI_trellis2cpp/blob/main/docs/architecture/README.md>
- Microsoft TRELLIS README + LICENSE (MIT): <https://github.com/microsoft/TRELLIS> · project page <https://microsoft.github.io/TRELLIS/>
- Paper: *Structured 3D Latents for Scalable and Versatile 3D Generation*, arXiv:2412.01506
- Microsoft TRELLIS.2 README + model card (MIT): <https://github.com/microsoft/TRELLIS.2> · <https://huggingface.co/microsoft/TRELLIS.2-4B>
- Paper: *Native and Compact Structured Latents for 3D Generation*, arXiv:2512.14692
- LocalAI-io GGUF cards: TRELLIS.2-4B-GGUF (MIT), dinov3-vitl16 GGUF (DINOv3 License)
- DINOv3 model card + license: <https://huggingface.co/facebook/dinov3-vitl16-pretrain-lvd1689m> · <https://ai.meta.com/resources/models-and-libraries/dinov3-license/>
- Hunyuan3D-2 / 2.1 README + Community LICENSE (territory + MAU + no-train-on-outputs)
- TripoSR README (MIT, fast feedforward): <https://github.com/VAST-AI-Research/TripoSR>
- Khronos glTF PBR: <https://www.khronos.org/gltf/pbr/> · spec <https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html>
- CGAL license page (GPL + commercial): <https://www.cgal.org/license.html>
- Workspace briefing: `docs/research/BRIEFING.md`

---

## 10. Do not copy

Short list. If a PR contains any of these, it is not clean-room.

1. **Source** from `AI_trellis2cpp`, `rms80/trellis2cpp`, `microsoft/TRELLIS`, `microsoft/TRELLIS.2`, Hunyuan3D trees, TripoSR trees — including "I only paraphrased the C++."
2. **C ABI** (`t2_*`, their option structs, ABI versioning scheme) and **Go demo server** (routes, port, job store layout).
3. **Private containers** (`.t2mesh`, `.dinodata`, `.occ`, `.latent`, their GGUF KV namespace).
4. **Internal layouts** — channel counts, token budgets, sampler step/CFG defaults, ggml graph structure, dual-grid field packing, cascade scaffold sizes as *our* public API.
5. **Custom glTF extras** as the material contract (non-standard vertex PBR attributes). Export Khronos core, or fail.
6. **Directory / CLI / symbol names** from those repos (`trellis2_*`, `ss_flow`, their example binary names).
7. **GPL print wrap** (CGAL Alpha Wrap) or **Hunyuan community weights** as default configure.
8. **Python/PyTorch** as a runtime dependency. Test dumps only, if CHARTER says so.
9. Their **architecture diagrams, mermaid, or AGENTS.md rules** pasted into our tree.
10. Their **measured workarounds** (chunk sizes, attention splits, graph-disable flags) copied without our own measurement.

Aspire to the *user-visible job*: image in, honest PBR GLB out, on CPU or any probed GPU or a network provider. Invent the rest.
