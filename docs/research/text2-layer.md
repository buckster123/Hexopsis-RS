# text2 layer — invention spec (2026-08-19)

**Status:** research lock for PRD writers. Not the PRD.  
**Custody:** this note + public papers/READMEs/LICENSE/API docs + garden sibling *architecture* docs. Do not open reference-project source (trellis2.cpp, Microsoft TRELLIS Python trees, Hunyuan/TripoSR/Meshy implementation trees).  
**Verdict in one line:** ship **Lattice Router + typed View Contract + Hero-Orbit loop** as the text path; feed the same image-to-3D plane the image path uses; never default Hunyuan.

---

## 0. What this document owns

The briefing requires a *candidate or an invention*, not a placeholder. This file **locks the invention** and evaluates the public 2026 landscape against it.

| Locked | Left to CHARTER / PRD |
|---|---|
| Text-path architecture: Lattice Router + View Contract v1 | Product name (OQ-1) |
| Default camera ring = **6** (4 cardinals + hero + top) | Local 3D engine: sidecar vs reimplementation (briefing §11.2) |
| Consistency gates G0–G4 with numeric v0 thresholds | Inference runtime (candle / ggml / …) |
| Retry / spend budget | Mesh-only vs Gaussian/NeRF |
| Hunyuan is **license-blocked as default** (local *and* hosted) | HTTP bind port |
| Sibling compose: Imaginarium T2I/I2I, Cadre analytic | Watertight/print GPL path |

Items marked **[inferred]** are product judgment, not quoted from a vendor.

---

## 1. Candidate evaluation (public facts)

### 1.1 Native TRELLIS-text (v1)

| | |
|---|---|
| **What it is** | Microsoft TRELLIS v1 text-conditioned DiTs: `TRELLIS-text-base` 342M, `TRELLIS-text-large` 1.1B, `TRELLIS-text-xlarge` 2.0B. MIT. Same SLAT family as image models. Paper: arXiv:2412.01506. |
| **Text?** | Yes, first-class. |
| **Image?** | Separate `TRELLIS-image-large` (1.2B). |
| **Local?** | Yes (HF weights). Python demo wants NVIDIA ≥16 GB. |
| **Authors' own advice** | Verbatim from the public README (2025-03-25 update): *“It is always recommended to do text to 3D generation by first generating images using text-to-image models and then using TRELLIS-image models for 3D generation. Text-conditioned models are less creative and detailed due to data limitations.”* A second note: *“It is always recommended to use the image conditioned version of the models for better performance.”* |
| **Adoption signal** | Hugging Face “downloads last month” at time of writing: image-large ~1.8M vs text-large ~2.3k vs text-xlarge ~8.2k. The community already follows the authors. |
| **Multi-image** | Tuning-free algorithm on the *image* model (not a separately trained text model). Publicly described as not always best for all inputs. |
| **Verdict** | **Optional native provider**, never the default visual route. MIT-clean, so it is a legitimate *offline* fallback when T2I is absent. Do not build the product around it. |

### 1.2 TRELLIS.2 image-to-3D (the quality class the text path should target)

| | |
|---|---|
| **What it is** | `microsoft/TRELLIS.2-4B`, 4B flow-matching transformer, **image-to-3D only**. MIT weights + MIT code license. Paper: arXiv:2512.14692. Input: *single image*. Output: mesh + PBR (base color, roughness, metallic, opacity). 512³–1536³. H100: ~3s / ~17s / ~60s. Public card: NVIDIA ≥24 GB, Linux. |
| **Text?** | **Not advertised.** No public text-conditioned TRELLIS.2 checkpoint. |
| **Why it still belongs in the text layer** | The authors of v1 already told us text-to-3D is better as T2I → image-3D. TRELLIS.2 is the open image-3D quality class. A text product that cannot feed this plane is leaving the best MIT mesh on the table. |
| **Implication for our contract** | Even a single-image 3D engine needs a *chosen hero view*. Extra views are not wasted: they gate identity *before* 3D spend, become texture/albedo references, and drop into any later multi-image engine (TRELLIS v1 multi-image, Tripo multiview, Meshy multi-image) without recompiling the prompt. |
| **Verdict** | **Default 3D plane** for the visual route (local sidecar or independent engine speaking our job protocol — engine choice is CHARTER, not this file). Text layer's job is to produce a **gated view set**, not to reinvent the 3D DiT. |

### 1.3 T2I-then-image-3D (naive)

Fire one Flux/Imagine image, pipe it to image-to-3D. This *is* the authors' recommended *direction*, and it is what Tripo's hosted text-to-model already does internally (`image_seed` controls “the internal text-to-image stage”).

It is **not** an invention:

- No inspectable camera/identity artifact.
- No consistency loop; Janus and identity drift land in the mesh.
- No prompt-class routing (a `40 mm M3 bracket` becomes a hallucinated neural bracket).
- No retry budget, no fail-closed gate, no provenance of *which* view was the condition.
- No dual-compute schema: local vs Meshy vs Tripo become three products.

**Verdict:** the *physics* we want (2D prior is stronger than native text-3D). The *product* must be the compiler + loop around it.

### 1.4 Hunyuan (local 2.1 + hosted 3.1)

See **§3 license vet**. Technical notes only here:

- **2.1 local:** image-to-3D (text often via an internal T2I), PBR, heavy VRAM (public commentary 10–29 GB). Hugging Face license tag `tencent-hunyuan-community`. GitHub LICENSE dated 2025-06-13.
- **3.1 hosted:** closed, text + image + up to 8 multiviews (third-party reports). Tencent Cloud “HY 3D Global API” + consumer site. fal.ai list price on the order of **$0.375 / generation** (Normal), more with PBR / extra views. Not open weights.
- **Verdict:** capable, **not garden-safe as default**. Networked adapter may exist behind an explicit license flag. Default configure must never pull 2.1 weights.

### 1.5 Tripo / Meshy hosted APIs

Both are **submit → poll → GLB** providers that already speak text, image, and multi-image. They fit the briefing's networked `ComputePlane` without being the architecture.

**Tripo AI API** (public developer docs, 2026):

- Base: `https://openapi.tripo3d.ai/v3`, `POST /generation/text-to-model`.
- Prompt ≤1024 chars, optional `negative_prompt`, separate seeds for mesh / internal T2I / texture.
- Models named `v2.5-20250123`, `v3.0-20250812`, `v3.1-20260211`.
- Credits: **1 credit = $0.01**. Text-to-3D **10 (no tex) / 20 (standard tex)**; image-to-3D 20/30; **multiview-to-3D 20/30**.
- Multiview endpoint canonicalizes **`[front, left, back, right]`**; front is mandatory; ≥2 images. This is a hard compatibility constraint on *our* camera ring (see §5).
- Internal T2I is opaque. We cannot inspect their view set. Fine as a *native* route; bad as the only text path.

**Meshy API** (docs.meshy.ai):

- Text-to-3D is **two-step**: `mode: preview` (mesh) then `mode: refine` (texture) on `POST /openapi/v2/text-to-3d`. Prompt ≤600 chars.
- Status enum: `PENDING | IN_PROGRESS | SUCCEEDED | FAILED | CANCELED`. SSE stream exists. `402` = no credits, `429` = rate limit. Credits refunded on `FAILED`.
- Pricing (credits/call): text preview **20** (Meshy-6/7; +5 Ultra); refine **10** (2k/4k) or **15** (8k); image-to-3D **20** untex / **30** tex / **35** 8k; multi-image same band. Credit-to-USD depends on the user's plan (not a stable unit like Tripo's $0.01).
- Formats: glb/obj/fbx/stl/usdz/3mf. PBR maps on refine when `enable_pbr`.
- Native text route is a first-class adapter. Not a substitute for View Contract when we want local 3D or inspectable views.

**TripoSR** (open, MIT, Stability+Tripo, arXiv:2403.02151): image-only, feedforward, **~0.5 s A100**, ~6 GB for one image. **Preview/Nano image-3D**, not a text model. License of the GitHub tree is MIT (copyright 2024 Tripo AI & Stability AI).

**Verdict:** Meshy + Tripo are **Route C providers** (native text-3D) and also valid **image-3D / multi-view-3D** backends for Route B's view set. They do not own the text layer.

### 1.6 Cadre-RS analytic (garden sibling)

Cadre is a Rust CAD runtime: **hermetic Starlark → B-rep → STEP (primary) / STL / 3MF / GLB (secondary)**. Dual MIT OR Apache-2.0 core; OCCT kernel is **opt-in LGPL** behind `GeomKernel`, not a default link. Not organic generation; meshes are export targets.

Public compose surface (docs, not source):

- MCP tools include `build`, `write_source` / `read_source`, `inspect_refs`, `measure`, `snapshot`, `export`. HTTP mirrors `/v1/build`, `/v1/export`, … Viewer on **:7411**.
- **`write_source` is OFF on stdio by default** (`CADRE_MCP_WRITE_SOURCE`); ON on HTTP. Compose via **CLI or HTTP**, not stdio write, unless the operator flips the flag.
- Units doctrine: **millimetres, XY base, +Z up**. glTF is Y-up. The text layer must not pretend these frames are the same (see §5.3).

**Verdict:** **Route A**. Dimensioned / mechanical prompts must not silently become neural meshes. If Cadre is absent, **honest refuse**, do not “degrade” to TRELLIS.

### 1.7 Imaginarium-RS (garden sibling, T2I/I2I)

Local-first xAI Imagine client. Core MIT OR Apache-2.0 (Slint GUI crate is GPL and **must not** be linked into text2mesh). Default bind **127.0.0.1:8791**. Key isolation: `XAI_API_KEY` lives on the Imaginarium node, never in text2mesh.

Public Imagine surface we will compose (not reimplement):

| Op | Path | Notes |
|---|---|---|
| Estimate | `imaginarium_estimate` | **Required before paid fire** (house doctrine §8) |
| T2I | `POST /v1/images/generations` | `grok-imagine-image` ~$0.02, `image-quality` ~$0.05–0.07, `image-2.0` ~$0.04 / out |
| I2I | `POST /v1/images/edits` | **1–3 source images**; this is the Hero-Orbit primitive |
| Caps | config `[limits]` | `max_usd_per_job`, `max_usd_per_day`, paid RPM |

xAI public docs: image edits are billed for **input image(s) and generated output**. Treat I2I as **≥2 image-units** until `imaginarium_estimate` says otherwise. Never hard-code USD in core; always estimate.

**Verdict:** default **T2I provider** when live. Standalone T2I (local sd.cpp / user OpenAPI) when absent. text2mesh never holds the xAI key.

### 1.8 Scoreboard

| Candidate | Text | Image | Local MIT/Apache | Garden-safe default | Role in *our* text layer |
|---|---|---|---|---|---|
| TRELLIS-text v1 | yes | via sibling ckpt | yes (MIT) | yes as *optional* | Route C offline |
| TRELLIS.2-4B | no | yes (single) | yes (MIT) | **yes (3D plane)** | Consume hero (+ extras as refs) |
| Naive T2I→3D | via T2I | yes | depends | architecture-no | Baseline to beat, not to ship |
| Hunyuan 2.1 local | via T2I | yes | **no** (community) | **no** | License-blocked |
| Hunyuan 3.1 hosted | yes | yes | no (ToS) | no (flag only) | Route C, explicit opt-in |
| Tripo API | yes | yes + 4-view | no (paid) | networked ok | Route C *or* Route B 3D backend |
| Meshy API | yes (preview+refine) | yes + multi | no (paid) | networked ok | same |
| TripoSR | no | yes, fast | yes (MIT) | Nano 3D | Preview image-3D |
| Cadre-RS | analytic | n/a | MIT/Apache (+ OCCT LGPL opt-in) | **yes (Route A)** | Mechanical prompts |
| Imaginarium-RS | T2I/I2I | n/a | MIT/Apache (no Slint) | **yes (T2I)** | Hero-Orbit synthesis |

---

## 2. The invention — Lattice Router + View Contract + Hero-Orbit

Working name (writer may rename in CHARTER, not here): **Lattice**.

Three routes, **one `MeshJob` object**. The router picks a route; the job schema does not change.

```
                    prompt + MeshJob
                           │
                    Lattice Router
                           │
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
     Route A          Route B            Route C
     Analytic        View Contract      Native text-3D
     (Cadre)         + Hero-Orbit       (Meshy/Tripo/
                     → image-3D plane    TRELLIS-text/
                                         Hunyuan-flag)
```

### 2.1 Why this is not “call Flux then Trellis”

Four properties a fire-and-forget T2I pipeline does not have:

1. **Typed artifact.** The View Contract is JSON with a schema id, a content hash, and a camera list a human (or agent) can edit and replay. Changing azimuth is a contract patch, not a prompt prayer.
2. **Hero-Orbit loop.** We do **not** sample N independent T2I images. We lock identity on one hero view, then I2I-orbit the ring with the hero as a reference (Imaginarium edits accept 1–3 images). That is the original synthesis, not N lucky samples.
3. **Fail-closed gates + retry budget.** Consistency is a job phase with numeric thresholds, a bounded retry ladder, and a structured failure. We do not spend 3D compute on a Janus set.
4. **Prompt-class router that composes siblings.** CAD-shaped prompts go to Cadre or **refuse**. Visual prompts go to View Contract. Native APIs are opt-in. License flags are a router input, not a README footnote.

Same `MeshJob` runs local or remote. The caller does not change schema when the 3D plane is a LAN sidecar vs Meshy.

### 2.2 Hero-Orbit (the synthesis, not the slogan)

```
compile ViewContract
    → estimate spend (T2I + reserved retries + 3D) → user/policy gate
    → T2I hero (canonical 3/4)
    → G0: hero vs prompt (CLIP-T / text-image). Fail → retry hero (budget)
    → for each remaining camera: I2I(hero [+ neighbor], camera lock prompt)
    → G1–G4 on the view set
    → retry ladder (§6) on worst views only
    → hand surviving views to image-to-3D plane
         primary condition = hero (or front, if engine demands a named slot)
         extras = multi-image / texture refs / eval pack
    → write provenance (contract hash, view hashes, scores, spend, licenses)
```

Independent N-view T2I is permitted only as a **degrade** when I2I is unavailable, and it still runs G1–G4.

---

## 3. Hunyuan license vet (binding)

Sources: `TENCENT HUNYUAN 3D 2.1 COMMUNITY LICENSE AGREEMENT`, Release Date **June 13, 2025**, GitHub `Tencent-Hunyuan/Hunyuan3D-2.1` and Hugging Face `tencent/Hunyuan3D-2.1` (same text). Not legal advice; this is an engineering default.

### 3.1 Local 2.1 — not garden-safe

Quoted constraints:

| Clause | Effect on us |
|---|---|
| Preamble + §1.l Territory = worldwide **excluding EU, UK, and South Korea** | Operator on Krackan (UK) **cannot** use, reproduce, modify, distribute, or **display outputs**. §5.c makes extra-territorial use “unlicensed and unauthorized.” |
| Grant is **“for the Territory only”** (§2) | Shipping 2.1 weights in a default download is a license violation for UK/EU/KR users and for any binary we distribute into those territories. |
| Hosted Service is defined (§1.d) and the preamble binds “including via any Hosted Service” | Wrapping 2.1 behind our HTTP API does not escape the community license. |
| §4 MAU cap | If all products of the licensee have **>1 million MAU** on the 2.1 release date, rights are **unlicensed** until Tencent grants a separate license (email `hunyuan3d@tencent.com`). Garden is nowhere near this; still a landmine if the code is reused. |
| §5.b | **No training / distillation / synthetic-data improvement of any other AI model** from Works or Outputs. Blocks Puerperium-style use and any “use Hunyuan to bootstrap our DiT.” |
| §5.c | Outputs themselves must not be used/displayed outside Territory. A UK user generating a GLB and viewing it is already out of scope. |
| §3.e | If we ever expose Hunyuan to third parties we must name the real provider and state Tencent is not affiliated. |
| Exhibit A AUP | No military; no extra-territory; long content list; Tencent may update AUP. |
| §9 | Hong Kong law + exclusive HK courts. |
| §6.d | Tencent claims no rights in outputs *you generate* — but only if you were licensed to generate them. |

**Engineering rule:** default configure, `system-check`, and CI **never** fetch Hunyuan 2.1 (or 2.0 — same family of community licenses with the same EU/UK/KR exclusion, public since 2025). A `hunyuan21` feature flag is insufficient if the flag can be on in the UK; the flag must also require an explicit **territory attestation** that we do not auto-detect as “yes.”

### 3.2 Hosted 3.1 — networked option only, unresolved for EU/UK/KR

- 3.1 is **closed**. Public launch Nov 2025 (Tencent / Tencent Cloud HY 3D Global API). Third-party aggregators (fal, Runware, 3D AI Studio) resell it.
- Tencent Cloud has a **separate** “HY 3D Global API Online Terms of Service.” That document is not the community license and does not, on its face, copy the EU/UK/KR exclusion. The 2.1 community license *defines Hosted Service* and claims to govern hosted use of 2.1 Works. **3.1 is a different model**, so the documents **conflict / do not clearly apply**. Until Tencent publishes a clarifying note, EU/UK/KR commercial use of *hosted* Hunyuan 3.x is **unresolved risk**.
- Garden doctrine: spend-gated networked adapters are allowed. **Default vendor: no.**

### 3.3 Product rules (copy into CHARTER)

1. `LicenseMatrix` in `system-check` lists Hunyuan as `blocked_by_default` with reasons `territory_eu_uk_kr`, `mau_cap`, `no_train_on_outputs`, `hk_law`.
2. No Hunyuan weights in the default model pack; no Hugging Face auto-pull.
3. Route C adapter `hunyuan_hosted` requires **all** of: API key present, `TEXT2MESH_ALLOW_HUNYUAN=1`, operator-signed territory attestation file (0600, not in git), and a non-default job field `license_override: "hunyuan_hosted"`.
4. Missing attestation is a **structured refuse**, not a timeout, not a silent skip to another engine that might still be Hunyuan.
5. Outputs from a Hunyuan run (if ever) carry `licenses[]` including the community-license / ToS URI so we never launder provenance.

---

## 4. View Contract — normative fields

Schema id: **`text2mesh.view_contract.v1`**. JSON, pretty-printable, hashed with SHA-256 over canonical JCS (or equivalent documented canonicalization). The hash goes on the `MeshJob` manifest.

```json
{
  "schema": "text2mesh.view_contract.v1",
  "contract_id": "ulid",
  "created_at": "RFC3339",
  "prompt": {
    "raw": "string",
    "normalized": "string",
    "hash": "sha256:…",
    "language": "en"
  },
  "subject_lock": {
    "identity_phrase": "string",
    "class": "creature|character|product|vehicle|architecture|prop|unknown",
    "attributes": ["string"],
    "canonical_view_id": "hero"
  },
  "camera_ring": {
    "preset": "cardinal4_hero_top",
    "count": 6,
    "convention": "y_up_azimuth_from_front",
    "distance": 1.6,
    "fov_deg": 35,
    "cameras": [
      {
        "id": "front",
        "role": "cardinal",
        "azimuth_deg": 0,
        "elevation_deg": 15,
        "roll_deg": 0,
        "required": true,
        "prompt_suffix": "front view, camera on the subject's forward axis"
      }
    ]
  },
  "lighting": {
    "rig": "studio_three_point",
    "locked": true,
    "key_azimuth_deg": -30,
    "fill_ratio": 0.4,
    "white_balance": "D65",
    "prompt_lock": "identical studio lighting in every view, no hard sunlight change"
  },
  "background": {
    "mode": "neutral_gray",
    "hex": "#B4B4B4",
    "alpha_preferred": true,
    "prompt_lock": "plain seamless studio backdrop, no scenery, subject fully in frame"
  },
  "style_lock": {
    "medium": "photoreal_product",
    "albedo_bias": false,
    "prompt_lock": "photoreal, sharp, no illustration, no watermark"
  },
  "negatives": [
    "extra limbs", "multiple faces", "janus", "text overlay",
    "watermark", "cropped", "wide-angle distortion", "different outfit"
  ],
  "seed_policy": {
    "family_seed": 0,
    "hero_seed": 0,
    "orbit_seed_mode": "family_plus_view_index"
  },
  "frame": {
    "width": 1024,
    "height": 1024,
    "aspect": "1:1"
  },
  "t2i": {
    "provider": "imaginarium|local|http|mock",
    "model": "grok-imagine-image-2.0",
    "quality_tier": "preview|quality"
  },
  "compile_notes": "why this ring / class / lighting"
}
```

### 4.1 Field rules

| Field | Rule |
|---|---|
| `prompt.raw` | Immutable once compiled. Edits clone a new contract. |
| `subject_lock.identity_phrase` | Compiler extracts a noun-phrase lock (“a red fox wearing a yellow raincoat”), not the whole prompt. Injected into every view prompt. |
| `subject_lock.class` | Drives **which gates run** (Janus probe is creature/character only). |
| `camera_ring.cameras[].required` | A failed *required* camera fails the job after retries. Optional cameras may be dropped (fail-down to Nano 4). |
| `lighting.locked` | Always true in v1. Per-view lighting is a v2 research item. |
| `background.mode` | `neutral_gray` default (T2I models honour “white/gray studio” more reliably than true alpha). `alpha` is best-effort; G3 degrades rather than fakes alpha. |
| `style_lock.albedo_bias` | When true, prompt asks for unlit / clay / albedo (better for PBR bake, worse for pretty previews). Default false. |
| `seed_policy.family_seed` | Job-level. Hero uses `hero_seed` (default = family). Orbit view *k* uses `family + k` in independent-T2I degrade; I2I orbit inherits hero latent via the edit, seeds still recorded. |
| `t2i.provider` | Name of a `T2iProvider` impl, not a URL with a key. |
| `compile_notes` | Human/agent readable. Required. |

### 4.2 Compiler (prompt → contract)

Deterministic, pure, unit-tested:

1. **Classify** (same function the Lattice Router uses) → `subject_lock.class`.
2. **Lock identity** via a small extract: keep material, colour, garment, species; strip camera words (“front view”, “isometric”) so they cannot fight the ring.
3. **Select ring preset** from quality tier: Nano 4 / Default 6 / Quality 8.
4. **Fill lighting/background** from class: `product` → three-point + gray sweep; `creature` → overcast + gray; `architecture` → overcast, slightly wider FOV.
5. **Negatives** = base list ∪ class extras (`creature` += extra legs, extra tails, face on the back).
6. Emit contract + `compile_notes`.

No network in the compiler. No LLM required in v1; a later optional LLM rewrite may fill `identity_phrase` but the compiler must work without it.

### 4.3 Per-view prompt assembly (pure)

```
{identity_phrase}, {style_lock.prompt_lock}, {background.prompt_lock},
{lighting.prompt_lock}, {camera.prompt_suffix},
azimuth {azimuth_deg} degrees, elevation {elevation_deg} degrees,
full subject in frame, same design as the reference
NEGATIVE: {negatives}
```

Hero T2I omits “same design as the reference”. Orbit I2I includes it and passes the hero (and optionally the nearest successful neighbor) as edit sources.

---

## 5. Camera ring

### 5.1 Convention

Name: **`y_up_azimuth_from_front`**.

- Right-handed. **+Y up.** Subject at origin, facing **+Z** (glTF-style front).
- Camera on a sphere of radius `distance` (default **1.6**, subject scale ≈ 1).
- `azimuth_deg`: rotation about +Y, **0 = front** (camera on +Z looking at origin). Increases toward +X (right-hand).
- `elevation_deg`: from the XZ plane, positive = above. 90 = top.
- `fov_deg` default **35** (mild tele; reduces wide-angle limb distortion that poisons 3D).
- `roll_deg` = 0 in v1.

This is **not** Cadre's +Z-up millimetre frame. Conversion is explicit at the analytic boundary (§8.2).

### 5.2 Presets (lock)

Literature we are *compatible with*, not copying: MVDream 4-view (0/90/180/270); Wonder3D 6-view equatorial; Zero123++ 6-view with interleaved elevations; **Tripo multiview API requires named `front, left, back, right`**. Our default therefore **contains those four named cardinals**.

**Nano 4 — `cardinal4`** (cheap, Tripo-shaped):

| id | az | el | required | role |
|---|---|---|---|---|
| front | 0 | 15 | yes | identity fallback, Tripo `front` |
| right | 90 | 15 | yes | Tripo `right` |
| back | 180 | 15 | yes | Janus witness, Tripo `back` |
| left | 270 | 15 | yes | Tripo `left` |

**Default 6 — `cardinal4_hero_top`** (recommended; closes briefing OQ-5):

| id | az | el | required | role |
|---|---|---|---|---|
| hero | 35 | 22 | yes | identity lock, TRELLIS.2 primary |
| front | 0 | 15 | yes | Tripo `front` |
| right | 90 | 15 | yes | |
| back | 180 | 15 | yes | Janus witness |
| left | 270 | 15 | yes | |
| top | 0 | 75 | **no** | polar; droppable on fail-down |

**Quality 8 — `cardinal4_hero_top_quarters`**: default 6 plus `qne` (az 45, el 18) and `qnw` (az 315, el 18), both optional.

Underside (`elevation_deg ≈ -20`) is **out of v1 default**. Table-top objects rarely need it; adding it doubles Janus/identity risk for cheap T2I.

### 5.3 Why 6, not 4 or 8

- **4** matches Tripo and is cheapest, but a single-image 3D engine (TRELLIS.2) wants a 3/4 hero, and 4 equatorial views miss the top (cups, vehicles, hats).
- **8** is Hunyuan-class and Zero123++-adjacent; I2I spend and drift both grow. Optional quality tier.
- **6** = 4 API-named cardinals + hero + droppable top. One contract feeds TRELLIS.2 (hero), Tripo multiview (cardinals), Meshy multi-image, and our eval pack.

---

## 6. Consistency gate (measurable)

All gates are **pure functions** over view bytes + contract. Encoder choice is a named, versioned artifact so scores replay.

**Default identity encoder (v0):** OpenCLIP `ViT-B-32` (laion2b), MIT/OpenCLIP weights. Alternative, same trait: DINOv2-S/14. We do **not** pull Hunyuan or TRELLIS encoders into the gate (license + size). Marked **calibrate-on-eval**: numbers below are v0 shipping defaults, not claims of optimality.

### 6.1 Gates

| ID | Applies | Pass when | Fail code |
|---|---|---|---|
| **G0 Hero-text** | always | `clip_cos(hero, identity_phrase ∪ prompt.normalized) ≥ 0.26` | `hero_text_mismatch` |
| **G1 Pairwise identity** | always | mean `clip_cos(hero, view_i)` for required views ≥ **0.72**; each required view ≥ **0.64**; adjacent cardinals ≥ **0.70** | `identity_drift` |
| **G2 Janus** | `creature` \| `character` only | `clip_cos(front, "a face, two eyes, front of a head") - clip_cos(back, same) ≥ 0.04` **and** back is closer to `"the back of a head, no face"` than to `"a face looking at camera"` | `janus_face` |
| **G3 Framing** | always | subject-ish mask (non-background gray/white cluster **or** alpha) occupies **0.28–0.82** of pixels; bounding box not glued to two opposite edges | `framing` |
| **G4 Lighting lock** | always | mean luminance of subject bbox within **±18%** of hero; gray-world RGB ratios within **0.15** of hero | `lighting_drift` |

Front-vs-back CLIP on *identity* is expected to be *lower* than adjacent views. We do **not** demand 0.72 on front×back; we demand G1 vs **hero**, plus G2 for faces.

**Fail-closed:** any required camera failing G1 or G3 after the retry ladder → job `failed`, `error_type=view_consistency`, scores + contract + view paths preserved. **Do not** call the 3D plane. Optional `top` may be dropped and the job continue (fail-down), recorded in the manifest as `cameras_dropped: ["top"]`.

### 6.2 Eval protocol (the briefing's success seed)

Fixed set **N = 24** prompts, 8 each of `{creature, product, prop}`, checked in as `evals/text2/prompts.json` (no live network in CI).

**Baseline (naive):** 1× T2I at hero pose → image-3D (or, for the *view* metric only, skip 3D).  
**Ours:** View Contract Default-6 + Hero-Orbit + gates.

**Primary metric (v1, no 3D required):** gate **pass rate** (G0∧G1∧G3, plus G2 if classed creature/character).  
**Target:** **≥ +20 percentage points** absolute vs naive 6 independent T2I samples of the same cameras (same spend band), on this 24-set. If naive already ≥80%, switch target to **Janus fail-rate ≤ half of naive**.

**Secondary (live, skip-loudly):** after 3D, CLIP-T of 8 orbit renders vs prompt (T³Bench-style) — informational, not a CI gate, until we have a local 3D plane in the test harness.

Human rubric (spot, not CI): extra limbs, extra faces, colour identity, “is this the same object.”

---

## 7. Retry budget (normative)

House: spend gated; jobs never stuck `pending`; missing key ≠ timeout.

```
RetryPolicy v1
  max_hero_resamples:     2          # G0 failures
  max_orbit_edits:        3          # total I2I retries across all views, not per view
  max_reseed_rounds:      1          # family_seed += 1, rebuild failed views only
  fail_down_drop_optional: true      # drop `top` (and quality extras) after ladder
  never_retry_on:         [missing_key, license_block, user_abort,
                           estimate_exceeded, provider_402]
  on_exhausted:           fail_closed  # error_type=view_consistency
```

**Ladder, in order:**

1. Identify worst required view (lowest G1 vs hero, or G2 fail on `back`).
2. **I2I edit** that view with sources = `[hero, nearest passing neighbor]` (≤3, Imagine cap).
3. If still fail and `max_orbit_edits` remains, one more edit with a tighter camera suffix.
4. If exhausted edits: **reseed** `family_seed+1` for the failed subset only (`max_reseed_rounds`).
5. Drop optional cameras; re-run G1–G4 on required set.
6. Fail closed.

**Spend:**

- Call `T2iProvider::estimate(contract, retry_policy)` **before** any paid POST.
- Estimate must include: 1 hero T2I + (count−1) orbit I2I + reserved retries (bill `max_orbit_edits` * I2I unit * **0.5**, labelled `reserved_retries`) + 3D plane estimate.
- Hard cap: estimated USD. Crossing it is `estimate_exceeded`, not a silent extra call. Operator may resubmit with a higher `max_usd`.
- Local T2I: USD = 0; **time budget** `max_wall_s` (Nano: 180s, default: 600s) still applies so jobs cannot sit `pending`.
- 402 / no-credits from Imaginarium or Meshy/Tripo: structured `provider_402`, job `failed`.

---

## 8. Lattice Router

### 8.1 Prompt classes

Pure classifier, keyword + pattern, no network:

| Class | Signals (examples) | Default route |
|---|---|---|
| `analytic` | `\b\d+(\.\d+)?\s*(mm|cm|m|in|inch)\b`, `M[2-9]\d?`, fillet/chamfer/extrude/bore/through-hole, “STEP”, “ISO 2768”, bracket/flange/standoff with dimensions | **A** |
| `creature` / `character` | animal, person, monster, “wearing”, named species | **B** |
| `product` | photo-like object, “product shot”, consumer goods without mm | **B** |
| `vehicle` / `architecture` / `prop` | as named | **B** |
| `unknown` | | **B** |

Override: job field `route: auto|analytic|view_contract|native`. `auto` is default.

**Honesty:** `analytic` **never** silently falls through to neural mesh. If Cadre is absent or the prompt is outside the analytic grammar → `error_type=analytic_unavailable` or `analytic_too_complex`, with a hint. Neural CAD requires explicit `route: view_contract` or `allow_neural_cad: true` (non-default).

### 8.2 Route A — Analytic (Cadre)

v1 grammar (closed, testable; not a CAD LLM):

- Primitives: `box | cylinder | tube` with millimetre dimensions.
- Features: through-holes (`M3`/`M4`/`M5`/`M6` clearance from Cadre doctrine table), simple linear patterns, optional fillet radius.
- Compiler emits **Starlark source** (our templates, not Cadre internals) + calls Cadre `write_source` (HTTP or CLI) → `build` → `export --format glb` (and STEP as the honest primary).
- `write_source` over stdio is **off** by default; compose uses `cadre` CLI or Cadre HTTP. If neither binary nor HTTP is present: refuse.
- Frame: Cadre +Z up, mm. Manifest records `frame: cadre_z_up_mm` and a glTF Y-up transform applied at GLB export if we re-wrap; prefer Cadre's own GLB.
- Out of grammar (impeller, organic housing, “looks like a dragon but 40mm”): `analytic_too_complex`. Mixed prompts (“steampunk clock, 40 mm diameter”) stay **B** unless CAD tokens dominate *and* the user did not ask for a creature.

Cadre OCCT is Cadre's LGPL problem, not ours; we do not link OCCT.

### 8.3 Route B — View Contract (default visual)

§4–§7, then the **same image-to-3D plane** as the image path (`Image3dPlane` trait).

Hand-off mapping:

| 3D backend | What it receives |
|---|---|
| TRELLIS.2-class / single-image sidecar | `hero` bytes (fallback `front`) |
| TRELLIS v1 multi-image | required views, order recorded |
| Tripo `multiview-to-model` | `{front,left,back,right}` from named cameras; hero/top ignored or sent as extras if the adapter has a slot |
| Meshy multi-image | required views as listed by their API |
| Texture-only follow-up | surviving views as colour refs |

### 8.4 Route C — Native text-3D

Opt-in provider when the user has keys or local TRELLIS-text weights. The router still records a **degenerate contract** (`preset: native_passthrough`, `cameras: []`) so provenance exists. No fake View Contract scores.

Providers: `meshy_text`, `tripo_text`, `trellis_text_xlarge`, `hunyuan_hosted` (flagged). Never the only path; Nano builds without any of them.

### 8.5 Dual compute (one trait)

```
ComputePlane {
  probe() -> Caps { weights, vram, ram, keys_present, licenses, spend_gate }
  estimate(job) -> Money + Duration
  submit(job) -> JobId
  poll(JobId) -> Status  # never stuck pending
}
```

Implementations from v1: `LocalOnboard`, `HttpProvider` (Meshy/Tripo/OpenAPI-ish), `ColonySibling` (LAN, not a special case). Planner `auto`: probe local → else remote if keys → else **stated degrade**. CPU-only is a distinct error/slow path, not a fake GPU success.

---

## 9. Sibling compose (standalone-first)

Zero siblings must still work: mock T2I + mock 3D in CI; local T2I provider if the user configured one; honest degrade otherwise.

| Sibling | How we compose | If absent |
|---|---|---|
| **Imaginarium-RS** | `T2iProvider` over HTTP `:8791` or MCP (`imaginarium_estimate`, image gen, image edit). We never read `XAI_API_KEY`. Respect their spend caps; our `max_usd` is the min of both. | Local T2I / user HTTP / mock. Job states `t2i_provider=none`. |
| **Cadre-RS** | `AnalyticProvider` via CLI or HTTP (`build`, `export`). No crate-level couple in v1. | Route A refuses. |
| **OmniOcular-RS** | May *visualize* our GLB. We do not steal `visualize`. | n/a |
| **CerebroCortex-RS** | Optional session notes. | n/a |
| **ApexOS-RS** | MCP consumer. | n/a |

**Do not** link `imaginarium-slint` (GPL). **Do not** vendor Cadre's OCCT.

Key isolation: INSTALLED ≠ ACTIVE. `system-check` prints which siblings responded, which keys exist (lengths/heads only), which licenses are blocked.

---

## 10. MeshJob (shared with the image path)

Minimum fields the text layer must fill (PRD may extend):

```
MeshJob {
  id, created_at
  input: { kind: text|image|views|analytic, prompt?, image_hash?, contract_id? }
  route: analytic|view_contract|native
  plane: local|http|colony|auto
  provider: name
  status: queued|running|needs_confirm|succeeded|failed|canceled
  error: { type, message, hint } | null
  spend: { estimated_usd, actual_usd, reserved_retries_usd, currency }
  timings_ms: { compile, t2i, gate, image3d, export }
  licenses: [ { name, spdx_or_uri, role } ]
  artifacts: { glb?, sidecar_json, views[] }
}
```

Sidecar JSON always stores: prompt hash, contract hash, view hashes, gate scores, engine id, device, licenses, spend. GLB without sidecar is incomplete (honesty).

Paid fire from a default MCP/CLI path is **forbidden** unless `confirm_spend: true` or an already-approved budget token is on the job. Estimate is free.

---

## 11. Recommended default + two alternatives

### (a) Recommended — Lattice Router + View Contract + Hero-Orbit

- **Text default:** Route B. Compile contract → estimate → hero T2I → I2I orbit → G0–G4 → image-to-3D plane.
- **3D plane:** TRELLIS.2-class MIT image-to-3D (sidecar in v1 is fine; independent engine is the horizon). Nano preview may use TripoSR-class feedforward.
- **T2I:** Imaginarium when live; else local/user HTTP; else honest refuse of Route B.
- **Analytic:** Route A via Cadre when classified; else refuse.
- **Native APIs:** present as Route C, not default.
- **Hunyuan:** blocked (§3).
- **Camera count:** Default-6.
- **Why:** matches public SOTA advice (T2I then image-3D), keeps MIT-clean defaults, is inspectable, spend-gated, sibling-composed, and has a numeric eval target. This is an original compiler+loop, not a vendor wrapper.

### (b) Alternative — Native-API-first (Meshy or Tripo as the text path)

- Router still classifies analytic vs visual, but visual jobs `POST` prompt to Meshy/Tripo text-to-model and poll GLB.
- Faster to a first mesh; **no local 3D**; **no inspectable views**; spend is vendor credits; quality and ToS are vendor-shaped.
- Use when the operator wants a thin networked client and accepts lock-in. Still implement View Contract as a *preview* mode (even if 3D is native) if we want the eval story.

### (c) Alternative — Local TRELLIS-text-xlarge (native text-3D, MIT)

- No T2I key required. MIT weights. Authors themselves rank this **below** T2I→image-3D on creativity/detail. HF adoption agrees.
- Valid **offline / air-gap** Route C when Imaginarium and paid APIs are absent and the user has ≥16 GB NVIDIA and the xlarge ckpt.
- Must not be advertised as equal quality to (a). `system-check` should say so.

**Not an alternative we will pick:** Hunyuan 2.1 local as default; Hunyuan 3.1 hosted as default; naive single-image T2I without gates.

---

## 12. Open questions this file does / does not close

**Closed here**

- Text-layer architecture (Lattice + View Contract + Hero-Orbit).
- Default camera count = **6** (`cardinal4_hero_top`).
- Hunyuan default = **no**.
- Consistency gates G0–G4 with v0 numbers + 24-prompt eval.
- Retry ladder and spend reservation.
- Sibling compose seams.

**Still CHARTER/PRD**

1. Product/crate name.
2. Default local *3D* engine (sidecar vs reimplementation).
3. Inference runtime.
4. Gaussian/NeRF as v1 outputs.
5. HTTP port (8791 Imaginarium, 8795 OmniOcular, 7411 Cadre view — pick free).
6. Watertight/print (GPL CGAL vs later pure-Rust).
7. Whether I2I is billed 2× on xAI — **do not guess in core; estimate**.
8. CLIP thresholds: v0 defaults; first field eval may retune by ±0.04 without a schema bump if documented in the sidecar `gate_version`.

---

## 13. Sources (public URLs only)

- Microsoft TRELLIS README / MIT LICENSE — https://github.com/microsoft/TRELLIS
- TRELLIS.2 card + README + MIT — https://huggingface.co/microsoft/TRELLIS.2-4B · https://github.com/microsoft/TRELLIS.2 · arXiv:2512.14692
- TRELLIS v1 paper — arXiv:2412.01506
- Hunyuan 2.1 community LICENSE — https://github.com/Tencent-Hunyuan/Hunyuan3D-2.1/blob/main/LICENSE (also HF `tencent/Hunyuan3D-2.1`)
- Tencent Cloud HY 3D Global API ToS — https://www.tencentcloud.com/document/product/301/78149
- Tripo developers (text, pricing, multiview) — https://developers.tripo3d.ai/en · https://developers.tripo3d.ai/en/pricing
- Meshy text-to-3D + pricing — https://docs.meshy.ai/en/api/text-to-3d · https://docs.meshy.ai/en/api/pricing
- TripoSR README + MIT LICENSE — https://github.com/VAST-AI-Research/TripoSR
- xAI Imagine overview — https://docs.x.ai/docs/guides/image-generation
- T³Bench (eval style) — arXiv:2310.02977
- Zero123++ / Wonder3D camera notes (compatibility, not a port) — public READMEs / papers cited in §5
- Garden docs (architecture only): Imaginarium-RS `docs/ARCHITECTURE.md`, `docs/LICENSING.md`; Cadre-RS `docs/CHARTER.md`, `docs/cadre-prd.md`, `docs/LICENSING.md`, `docs/HERMES_MCP.md`, `docs/VIEWER.md`; Launchpad-RS `docs/house-doctrine.md`
- Briefing: `docs/research/BRIEFING.md`

No trellis2.cpp, TRELLIS Python, Hunyuan, or Meshy/Tripo *implementation* source was opened for this note.
