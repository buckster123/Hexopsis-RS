# text2mesh — design contract

| Field | Value |
|---|---|
| **Status** | v0.4 — **implemented** (S0–S12 on `main`, 2026-08-19). Additive changes only; schema bump needs a CHARTER amendment. |
| **Date** | 2026-08-19 |
| **Wins over** | informal chat, research notes |
| **Loses to** | CHARTER D1–D30 |
| **Product twin** | `docs/prd.md` Draft v0.4 |

Code follows this document. A PR that changes behaviour updates this file in the **same commit**. After PR-01, field names freeze; further change is **additive** or a schema version bump (`text2mesh.job.v2`) plus a CHARTER amendment.

Research notes under `docs/research/` are **writer notes**, not implementation specs and not a second custody path.

---

## Provenance / custody

**Allowed:** this file, `docs/prd.md`, `docs/CHARTER.md`, Khronos glTF 2.0, GGUF spec, crates.io, sibling *public* HTTP routes (Imaginarium `/v1/estimate|images/*`, Cadre `/v1/health|build|export`).

**Forbidden:** opening, cloning, or paraphrasing statement-level source from `RobertBeckebans/AI_trellis2cpp`, `rms80/trellis2cpp`, Microsoft `TRELLIS` / `TRELLIS.2` Python trees, Hunyuan3D, TripoSR, Meshy implementation trees. Do not adopt `t2_*`, `.t2mesh`, `.dinodata`, their ggml graphs, sampler defaults, or stage layouts.

**Appendix B** of the PRD is **writer provenance only**. Implementers do **not** follow those GitHub URLs. Do not paste architecture-README content into this file.

---

## 0. Invariants

These are not guidelines. Violating one is a bug.

1. **No fake success.** Manifest `ok=true` **iff** `status=succeeded`. That requires a parser-accepted GLB **and** materials as claimed (UV atlas or equivalent PBR textures/factors the user asked for). Vertex-colour / factors-only → `degraded` (`export.material_mode`). Default-only metallic-roughness factors with no `COLOR_0` variation and no textures → `failed` `export.materials_missing`. **No preview exception.**
2. **Degrades are stated.** Status `degraded` is a distinct terminal. UI/CLI/MCP must surface `degrades[]`. CLI exit **1**. Manifest `ok=false`. A naked green tick is a bug.
2a. **`ok` tokens are split (D29).** Face-wrapper `ok` = this HTTP/RPC call parsed and the job (or report) exists. It never means “meshed.” `system-check` does **not** use `ok` for readiness: `report_complete` + `ready` (`would_pick != null`). Wait timeout: wrapper `ok=true`, `wait_timed_out=true`, job row unchanged.
3. **No orphan `pending`.** JSON status is never the string `pending`. Use `queued | needs_confirm | submitted | running | waiting_upstream`.
4. **Missing key ≠ timeout.** `not_configured` in milliseconds. Same for missing weights, closed spend gate, license block, missing sidecar.
5. **Persist `job_id` before `submit` returns.** Incomplete artefact+manifest writes are ignored on startup (rename-into-place or nothing).
6. **Local death is ours.** Process crash, sidecar SIGKILL, OOM → `failed` `engine.interrupted` / `engine.crash` / `engine.oom`. Do not leave local jobs `running` across reboot.
7. **Paid remote is theirs.** After `upstream_id` exists, our poll/wait timeout → `waiting_upstream`, not silent `failed`. `recover_ttl` (default 24 h) then `failed` `wait.timeout` with spend recorded.
8. **Spend gate default closed.** Any `estimate.usd > 0` needs `allow_spend` (env **or** CLI flag **or** tool arg). MCP prefers the tool arg.
9. **Mock is not quality.** Auto planner selects `local.mock` only if `TEXT2MESH_ALLOW_MOCK=1`. Mock always terminates **`degraded`**.
10. **Analytic never silent-neural.** Route A refuses if Cadre is absent or the prompt is out of grammar.
11. **Do not call Image3dPlane** if a required View Contract camera failed **any of G1–G4** after the retry ladder. Keep the **specific** `error_type` of the gate that failed; wrap `view.consistency` on ladder exhaust (`error.also` = specific). Optional cameras may drop.
12. **stdout of `-mcp` is sacred.** JSON-RPC only. `tracing` → stderr. No `println!` on the MCP path.
13. **No `XAI_API_KEY` in this process.**
14. **No Hunyuan default. No GPL print-wrap default. No DINOv3 without accept.**
15. **Faces share one type layer.** CLI JSON, MCP schemas, OpenAPI generated from the same Rust types. Drift is CI fail.
16. **Capability query, not product `#ifdef`s.** CUDA compiled-out is a probe result, not a second binary product.
17. **Hashes are of bytes we used.** Conditioned image, contract JCS, view PNGs, GLB. Filenames are not identity.
18. **Nano default build has no heavy runtime.** `cargo test --workspace` green without ggml/CUDA/ONNX/14 GB.

---

## 1. Identifiers and schemas

| Kind | Rule |
|---|---|
| `job_id` | ULID, Crockford, minted here. Primary key. |
| `contract_id` | ULID, minted at compile. |
| `upstream_id` | Opaque string from a provider. Nullable. |
| `idempotency_key` | Caller string, ≤128 bytes, optional. Same key inside **`recover_ttl` (24 h)** or while the job dir exists, whichever is shorter → same `job_id`. |
| Schema ids | `text2mesh.job.v1`, `text2mesh.view_contract.v1`, `text2mesh.manifest.v1`, `text2mesh.system_check.v1`, `text2mesh.estimate.v1` |
| Sidecar protocol | `meshplane/1` |
| MCP protocol | `"2024-11-05"` |
| Hash prefix | `sha256:` + 64 hex lowercase |
| Time | RFC3339 UTC (`…Z`) |

Canonical JSON for hashing: **JCS RFC 8785**. If the chosen crate is blocked, freeze an equivalent in this file (UTF-8, sorted object keys, no insignificant whitespace, numbers as shortest IEEE round-trip) **before** the first contract golden lands.

---

## 2. Enums (frozen)

```text
Route            = auto | analytic | view_contract | native
PromptClass      = analytic | creature | character | product | vehicle | architecture | prop | unknown
Quality          = preview | standard | high | ultra
ComputeMode      = auto | local | remote
DeviceKind       = cpu | nvidia.cuda | amd.rocm | gpu.vulkan | apple.metal
PlaneId          = local.mock | local.sidecar | local.preview | local.analytic
                 | remote.meshy | remote.tripo | remote.colony
                 | remote.hunyuan_hosted   # inert unless all D19 gates; never auto if others feasible
JobStatus        = queued | needs_confirm | submitted | running
                 | waiting_upstream | succeeded | degraded | failed | cancelled
MaterialMode     = uv_atlas | vertex_color | factors_only
AlphaMode        = OPAQUE | MASK | BLEND
CameraPreset     = cardinal4 | cardinal4_hero_top | cardinal4_hero_top_quarters
                 | native_passthrough
T2iProviderId    = imaginarium | http | local | mock
Synthesis        = hero_orbit | independent_t2i | native_passthrough | analytic
ArtifactKind     = glb | manifest | contract | view | log
```

`remote.hunyuan_hosted` is inert unless CHARTER D19 gates are all true.

---

## 3. MeshJob

Schema `text2mesh.job.v1`. This is the object CLI `--json`, MCP results, and HTTP bodies share.

```json
{
  "schema": "text2mesh.job.v1",
  "id": "01J9Z0EXAMPLEULID00000000",
  "created_at": "2026-08-19T12:00:00Z",
  "updated_at": "2026-08-19T12:00:01Z",
  "parent_job": null,
  "idempotency_key": null,
  "input": {
    "kind": "text",
    "prompt": "a red fox wearing a yellow raincoat",
    "prompt_hash": "sha256:…",
    "image_path": null,
    "image_hash_raw": null,
    "image_hash_conditioned": null,
    "contract_id": "01J9Z0CONTRACTULID0000000"
  },
  "route": "view_contract",
  "quality": "standard",
  "compute": {
    "mode": "auto",
    "prefer_device": null,
    "provider": null,
    "requested": "auto",
    "actual": null
  },
  "seed": 42,
  "camera_preset": "cardinal4_hero_top",
  "allow_spend": false,
  "allow_neural_cad": false,
  "allow_native_text": false,
  "license_override": null,
  "export": {
    "keep_largest_component": false,
    "force_opaque": false,
    "unit_cube": false,
    "uv_atlas": false,
    "print_wrap": false
  },
  "budget": {
    "max_usd": 2.0,
    "max_credits": null,
    "max_wall_s": 1800,
    "max_vram_mb": null
  },
  "status": "queued",
  "stage": null,
  "pct": 0,
  "plane": null,
  "upstream_id": null,
  "cancel_requested": false,
  "error": null,
  "degrades": [],
  "spend": {
    "estimated_usd": null,
    "actual_usd": null,
    "reserved_retries_usd": null,
    "currency": "USD",
    "usd_uncertain": false
  },
  "artifacts": {
    "glb": null,
    "manifest": null,
    "contract": null,
    "views": []
  }
}
```

### 3.1 Input kinds

| `input.kind` | Required | Notes |
|---|---|---|
| `image` | `image_path` or store-relative bytes already ingested | Single still. PNG/JPEG. |
| `text` | `prompt` (1..=4000 Unicode chars after trim) | Router runs. |
| `views` | `contract_id` + view files on disk | Replay / resume after T2I. |
| `analytic` | `prompt` | Forced Route A. |

Exactly one of `image` / `text` at submit for human faces. `views` is director-internal or a power-user replay.

### 3.2 Error object

```json
{
  "error_type": "view.consistency",
  "message": "required camera back failed G1 after retry ladder",
  "hint": "inspect views/ and contract.json; raise max_usd to allow more I2I, or edit the contract",
  "also": []
}
```

`error_type` values: see PRD §10.2. Stable strings. Add only with a design amendment.

### 3.3 Degrade codes

| Code | Meaning |
|---|---|
| `quality.step_down` | `requested` ≠ `achieved` |
| `export.material_mode` | vertex colour or factors-only instead of UV atlas |
| `export.force_opaque` | alpha discarded at user or engine request |
| `cameras_dropped` | optional cameras removed after ladder |
| `synthesis.independent_t2i` | I2I unavailable |
| `gate.encoder_missing` | only if `TEXT2MESH_ALLOW_UNGATED=1` |
| `remote.material_fidelity` | provider GLB without PBR |
| `compute.cpu_slow` | quality ran on CPU |
| `t2i.provider_fallback` | imaginarium → user HTTP |

### 3.4 `JobSubmit` (one type, all faces)

Caller-settable fields. clap / MCP / OpenAPI are generated from this struct (PR-01b). Defaults match the MeshJob example (all export flags **false**).

| Field | Type | Default | CLI | MCP / HTTP JSON |
|---|---|---|---|---|
| `prompt` | string? | — | `--prompt` | `prompt` |
| `image_path` | string? | — | `--image` | `image_path` |
| `route` | Route | `auto` | `--route` | `route` |
| `quality` | Quality | `standard` | `--quality` | `quality` |
| `compute` | ComputeMode | `auto` | `--compute` | `compute` |
| `provider` | PlaneId? | null | `--provider` | `provider` |
| `prefer_device` | DeviceKind? | null | `--prefer-device` | `prefer_device` |
| `seed` | u64? | null | `--seed` | `seed` |
| `camera_preset` | CameraPreset? | null | `--preset` | `camera_preset` |
| `allow_spend` | bool | false | `--allow-spend` | `allow_spend` |
| `allow_neural_cad` | bool | false | `--allow-neural-cad` | `allow_neural_cad` |
| `allow_native_text` | bool | false | `--allow-native-text` | `allow_native_text` |
| `license_override` | string? | null | `--license-override` | `license_override` |
| `max_usd` | f64 | 2.0 | `--max-usd` | `max_usd` |
| `max_credits` | u64? | null | `--max-credits` | `max_credits` |
| `max_wall_s` | u64 | **1800** (min 30, max 86400) | `--max-wall-s` | `max_wall_s` |
| `idempotency_key` | string? | null | `--idempotency-key` | `idempotency_key` |
| `export.keep_largest_component` | bool | false | `--keep-largest` | `export.keep_largest_component` |
| `export.force_opaque` | bool | false | `--force-opaque` | `export.force_opaque` |
| `export.unit_cube` | bool | false | `--unit-cube` | `export.unit_cube` |
| `export.uv_atlas` | bool | false | `--uv-atlas` | `export.uv_atlas` |
| `export.print_wrap` | bool | false | `--print-wrap` | `export.print_wrap` |
| `job_id` | ULID? | — | confirm only | confirm / resubmit |

Exactly one of `prompt` / `image_path` (or HTTP image body) at create. Confirm: `job_id` + `allow_spend=true` (CLI `text2mesh confirm JOB`, MCP `text2mesh_submit` with `job_id`, HTTP `POST /v1/jobs/{id}/confirm`).

**HTTP image ingest (pick, frozen):** JSON `{ "image_path": "…" }` **or** `multipart/form-data` field `image` (PNG/JPEG bytes) + field `spec` (JSON JobSubmit without image). No raw-body-only path in v1. Caps: design §20.

**Face-only ops:** HTTP has no blocking wait (poll / SSE). MCP `submit` never blocks. CLI `generate` = submit + wait. Frozen wait/wall bounds: min **30**, default **1800**, max **86400** on MCP `timeout_s`, CLI `--timeout-s`, and Route B `max_wall_s`.

---

## 4. View Contract

Schema `text2mesh.view_contract.v1`. Normative fields match PRD §7.4. Additional rules for implementers:

### 4.1 Compiler algorithm (pure, implementable)

No network. No LLM. Deterministic given `(prompt, quality, camera_preset, family_seed, t2i.provider)`.

**`prompt.normalized`:** Unicode NFC; trim; collapse ASCII whitespace (`[ \t\r\n]+` → single space); lowercase **only** for classify/match tables, keep original casing in `prompt.raw`. `prompt.hash` = SHA-256 of UTF-8 `normalized`. `language` is **always `"en"`** in v1 (no detector).

**`identity_phrase`:** start from `normalized`; delete camera-word spans (case-insensitive, whole words / hyphenated): `front view`, `side view`, `back view`, `top view`, `bottom view`, `isometric`, `three-quarter`, `three quarter`, `3/4`, `close-up`, `closeup`, `wide angle`, `wide-angle`, `orthographic`, `bird's eye`, `birds eye`, `worm's eye`. Collapse leftover whitespace. If empty after strip, use `normalized` unchanged. That remainder **is** the identity phrase (no POS tagger). Fixture table `evals/text2/identity.json`: `(prompt, identity_phrase, class)` — at least the 24 eval prompts plus the camera-word cases. PR-10 checks in `prompts.json` **and** `identity.json`.

**`subject_lock.attributes`:** v1 **always `[]`**. No adjective extractor.

**`subject_lock.class`:** `classify` (below). `character` iff any humanoid token matches; else `creature` if species/monster tokens match.

**`canonical_view_id`:** `hero` if the compiled preset contains a camera `id=hero`, else `front`. G0/G1 “vs hero” means vs **`canonical_view_id`**. Hand-off “hero else front” is the same rule. Preview/`cardinal4` therefore uses `front` as the identity view; the compiler must not emit `canonical_view_id=hero` on a 4-view contract.

**Preset from quality** (unless `job.camera_preset` set): `preview` → `cardinal4`; `standard` → `cardinal4_hero_top`; `high|ultra` → `cardinal4_hero_top_quarters`.

**Lighting / background by class (rig/mode):**

| class | `lighting.rig` | `background.mode` | extras |
|---|---|---|---|
| creature, character | `overcast` | `neutral_gray` | Janus negatives |
| product, prop | `studio_three_point` | `neutral_gray` | — |
| architecture | `overcast` | `neutral_gray` | `fov_deg=42` |
| vehicle | `studio_three_point` | `neutral_gray` | `fov_deg=38` |
| unknown | `studio_three_point` | `neutral_gray` | — |

**Class lock strings (frozen exact; S5 goldens are a function of this table + §4.3 `prompt_suffix`):**

| class | `lighting.prompt_lock` | `background.prompt_lock` | `style_lock.medium` | `style_lock.prompt_lock` |
|---|---|---|---|---|
| creature, character | even overcast studio lighting, no hard shadows | plain neutral gray background | photoreal | photoreal product photography, single subject |
| product, prop | studio three-point lighting, soft key, fill, rim | plain neutral gray background | photoreal | photoreal product shot, single object, catalog |
| architecture | even overcast daylight | plain neutral gray background | photoreal | architectural model, single building, no people |
| vehicle | studio three-point lighting, soft key, fill, rim | plain neutral gray background | photoreal | vehicle product shot, single vehicle, no riders |
| unknown | studio three-point lighting, soft key, fill, rim | plain neutral gray background | photoreal | photoreal product shot, single object, catalog |

**`negatives[]` frozen exact JSON arrays:**

creature **and** character (Janus set):

```json
["second face","face on the back of the head","two faces","duplicate head","extra limbs","cropped limbs","text","watermark","multiple subjects","logo"]
```

All other classes (`product`, `prop`, `architecture`, `vehicle`, `unknown`):

```json
["text","watermark","logo","multiple subjects","cropped object","hands holding the object"]
```

**`t2i.quality_tier`:** `preview` if `job.quality=preview`, else `quality`. Default model name is **not** frozen (provider catalog); record whatever `T2iProvider` reports. Example `grok-imagine-image-2.0` is illustrative only.

**`orbit_seed_mode=family_plus_view_index`:** `view_index` = 0-based index in the compiled `cameras[]` array (preset table order, including optional cameras). Seed for camera *k* = `family_seed + k` on independent-T2I degrade; I2I orbit inherits hero latent and still records `family_seed + k`.

**`compile_notes`:** `class={class}; ring={preset}; lighting={rig}; canonical={canonical_view_id}`.

Goldens must match this table (creature = `overcast`, not three-point) **and** the class lock / negatives strings above. Do not invent `prompt_lock` / `medium` / `negatives[]`.

### 4.2 Camera convention `y_up_azimuth_from_front`

Right-handed. +Y up. Subject at origin facing +Z. Camera on sphere radius `distance` (default 1.6).

```
eye = (distance * cos(el) * sin(az),
       distance * sin(el),
       distance * cos(el) * cos(az))
# az, el in radians; az=0 → +Z (front)
look_at = origin
up = +Y
fov_deg default 35, roll 0
```

Not Cadre +Z-up mm. Analytic boundary records `frame=cadre_z_up_mm` separately.

### 4.3 Preset tables

`prompt_suffix` is frozen exact. Preview `cardinal4` has **no** `hero`; `canonical_view_id=front`.

**`cardinal4`**

| id | az | el | required | role | prompt_suffix |
|---|---|---|---|---|---|
| front | 0 | 15 | yes | Tripo `front` | front view, camera on +Z |
| right | 90 | 15 | yes | Tripo `right` | right side view |
| back | 180 | 15 | yes | Janus | back view, camera on -Z |
| left | 270 | 15 | yes | Tripo `left` | left side view |

**`cardinal4_hero_top`** (compiler default; OQ-5 locked 6)

| id | az | el | required | role | prompt_suffix |
|---|---|---|---|---|---|
| hero | 35 | 22 | yes | single-image 3D primary | three-quarter view from the front-right |
| front | 0 | 15 | yes | | front view, camera on +Z |
| right | 90 | 15 | yes | | right side view |
| back | 180 | 15 | yes | | back view, camera on -Z |
| left | 270 | 15 | yes | | left side view |
| top | 0 | 75 | no | droppable | top-down view |

**`cardinal4_hero_top_quarters`**

| id | az | el | required | role | prompt_suffix |
|---|---|---|---|---|---|
| hero | 35 | 22 | yes | single-image 3D primary | three-quarter view from the front-right |
| front | 0 | 15 | yes | | front view, camera on +Z |
| right | 90 | 15 | yes | | right side view |
| back | 180 | 15 | yes | | back view, camera on -Z |
| left | 270 | 15 | yes | | left side view |
| top | 0 | 75 | no | droppable | top-down view |
| qne | 45 | 18 | no | optional quarter | three-quarter view from the front-right, slightly higher |
| qnw | 315 | 18 | no | optional quarter | three-quarter view from the front-left, slightly higher |

**`native_passthrough`**: `cameras: []`. Route C only. No fake scores.

### 4.4 Per-view prompt (pure)

```
hero:
  {identity_phrase}, {style_lock.prompt_lock}, {background.prompt_lock},
  {lighting.prompt_lock}, {camera.prompt_suffix},
  azimuth {azimuth_deg} degrees, elevation {elevation_deg} degrees,
  full subject in frame
  NEGATIVE: {negatives joined by ", "}

orbit:
  hero text + ", same design as the reference"
```

### 4.5 Hash

`contract_hash = sha256(jcs(contract_without_hash_fields))`.

Exclude: none — the compiler does not put a hash *inside* the contract. The job/manifest stores the hash of the whole contract file as written (pretty JSON is **not** the hash input; hash JCS of the parsed value).

---

## 5. Lattice Router

```
route_job(job, probes) -> RouteDecision
  if job.route != auto: honor it (still validate)
  class = classify(prompt or "")
  if job.input.kind == image: Image (no lattice)
  if class == analytic && !job.allow_neural_cad: Analytic
  if job.route == native || job.allow_native_text && t2i_missing && native_live: Native
  else: ViewContract
```

`classify` is keyword + regex, fixture table in `evals/text2/classify.json` (same 24 prompts + edge cases).

**Humanoid tokens (`character`):** `person`, `human`, `humanoid`, `man`, `woman`, `child`, `boy`, `girl`, `character`, `android`, `robot person`, `portrait`.

**Creature tokens:** `creature`, `monster`, `animal`, `beast`, `dragon`, `fox`, `cat`, `dog`, `bird`, `wolf`, `bear`, `horse`, `fish`, `snake`, `wearing` (garment on a living subject). Closed list is **`evals/text2/species.txt`** (extend only with a design amendment). Inline tokens in this section **must match** that file — not a second list. First match wins: humanoid → `character`, else creature-token → `creature`. PR-13 checks in `classify.json` + `species.txt`.

**Product tokens:** `product shot`, `product photo`, `consumer`, `gadget`, `bottle`, `mug`, `chair`, `lamp`, `shoe` (no dimension).

Analytic signals (v1):

- `\b\d+(\.\d+)?\s*(mm|cm|m|in|inch(?:es)?)\b`
- `\bM[2-9]\d?\b` (metric fastener)
- words: `fillet`, `chamfer`, `extrude`, `bore`, `through-hole`, `through hole`, `step`, `iso 2768`, `standoff`, `flange` **when a dimension also matches**
- `bracket` + dimension

If both analytic and creature/character fire: **ViewContract** unless the user set `route=analytic`.

### 5.1 Route A grammar (v1)

Closed. Anything else → `analytic.too_complex`.

```
box <L>x<W>x<H> mm
cylinder d=<D> h=<H> mm
tube od=<D> id=<d> h=<H> mm
through-hole M3|M4|M5|M6 at (x,y) [pattern nx=<n> dx=<mm>]
fillet r=<mm>   # optional, one radius
```

Clearance holes (mm), copied as **our** fixture numbers (Cadre doctrine as public compose, not a crate dep):

| Thread | Clearance Ø mm |
|---|---|
| M3 | 3.4 |
| M4 | 4.5 |
| M5 | 5.5 |
| M6 | 6.6 |

Emit Starlark from **our** templates under `templates/analytic/` into `jobs/<id>/analytic/source.star`. Cadre is allowed to read that path. Wire: design §19.1. If `TEXT2MESH_CADRE_URL` and `TEXT2MESH_CADRE_BIN` both missing → `analytic.unavailable`. Probe = `GET {CADRE_URL}/v1/health` (5 s).

### 5.2 Route B hand-off

| Backend | Primary | Extras |
|---|---|---|
| single-image sidecar | `hero` else `front` | remaining required views as refs |
| Tripo multiview adapter | named `front,left,back,right` | hero/top ignored or extra slot |
| Meshy multi-image | required views in listed order | |
| local.mock | hero bytes | unused |

### 5.3 Route C

Degenerate contract. No G0–G4. Manifest `synthesis=native_passthrough`. `auto` does **not** pick C for visual prompts when B is feasible. `allow_native_text` default **false**.

---

## 6. Consistency gates

Pure. `gate_version = "g0_v0"`. Encoder id recorded (`openclip_vit_b32_laion2b` | `dinov2_s14` | `none`).

| ID | Pass | Fail |
|---|---|---|
| G0 | `clip_cos(canonical_view, T) ≥ 0.26` where `T = identity_phrase` if non-empty else `prompt.normalized` (not union, concat, or max) | `view.hero_text_mismatch` |
| G1 | mean canonical×required ≥ 0.72; each required ≥ 0.64; **adjacent cardinals** ≥ 0.70 | `view.identity_drift` |
| G2 | creature/character only: `cos(front, FACE) - cos(back, FACE) ≥ 0.04` AND back closer to BACK than FACE | `view.janus_face` |
| G3 | subject mask 0.28–0.82 of pixels; bbox not glued to two opposite edges | `view.framing` |
| G4 | subject-bbox luminance within ±18% of canonical; gray-world RGB ratios within 0.15 | `view.lighting_drift` |

FACE = `"a face, two eyes, front of a head"`.
BACK = `"the back of a head, no face"`.
No third FACE string. G0/G1/G4 score against **`canonical_view_id`**, not a missing `hero`. Preview `cardinal4` has no `hero`.

**Adjacency (G1 “adjacent cardinals”):** only the ring `{front, right, back, left}` in that cyclic order. `hero`, `top`, `qne`, `qnw` are **not** adjacent pairs. Missing cardinal → skip that pair.

**G3 algorithm (pure, no ML):**
1. If the PNG has an alpha channel: subject mask = `alpha ≥ 16`.
2. Else: pixel is background iff Chebyshev distance of sRGB to `background.hex` (default `#B4B4B4`) is ≤ 18 on each channel; subject = complement.
3. Pass if subject pixel fraction ∈ [0.28, 0.82].
4. Axis-aligned bbox of subject pixels: fail if bbox touches **two opposite** image edges within 4 px (left+right, or top+bottom).

**G4 algorithm:** mean luminance of subject bbox (`Y = 0.2126 R + 0.7152 G + 0.0722 B`, 0–1) within ±18% of canonical view. Gray-world: `(meanR, meanG, meanB) / (R+G+B)` per subject bbox; each channel ratio vs canonical within 0.15.

**Worst-view for retry:** lowest G1 vs canonical among required views that fail **any** of G1/G2/G3/G4. G3-only and G4-only failures enter the same ladder (not G1/G2 only).

Without encoder: G0–G2 cannot run. Default → `failed` `feature_off`. If `TEXT2MESH_ALLOW_UNGATED=1` → continue with `degrades+=gate.encoder_missing`; G3–G4 still run.

CI without `gate-clip`: use precomputed scores in `evals/text2/scores/` or a mock encoder that hashes bytes → fixed vector (not CLIP, labelled `encoder=mock_hash`, **not** used for M3 claims).

### 6.1 RetryPolicy

```json
{
  "max_hero_resamples": 2,
  "max_orbit_edits": 3,
  "max_reseed_rounds": 1,
  "fail_down_drop_optional": true,
  "never_retry_on": [
    "not_configured",
    "license.blocked",
    "cancelled",
    "spend.estimate_exceeded",
    "spend.provider_402"
  ]
}
```

Ladder: worst required view (any of G1–G4) → I2I `[canonical, nearest passing neighbor]` → tighter suffix → reseed failed subset → drop optional → fail-closed. After retries, a **required** view failing **any of G1–G4** → **do not** call Image3dPlane. Keep the **specific** `error_type` of the gate that failed; wrap `view.consistency` on exhaust (`error.also` = specific). Optional cameras may drop.

---

## 7. ComputePlane

```rust
#[async_trait]
pub trait ComputePlane: Send + Sync {
    fn id(&self) -> PlaneId;
    fn kind(&self) -> PlaneKind; // Local | Remote
    fn caps(&self) -> PlaneCaps;

    async fn probe(&self) -> ProbeReport;
    fn estimate(&self, spec: &JobSpec) -> Result<CostEstimate, PlaneError>;
    async fn submit(&self, spec: JobSpec) -> Result<JobHandle, PlaneError>;
    async fn poll(&self, id: &JobId) -> Result<JobSnapshot, PlaneError>;
    async fn wait(&self, id: &JobId, timeout: Duration) -> Result<JobSnapshot, PlaneError>;
    async fn cancel(&self, id: &JobId) -> Result<CancelOutcome, PlaneError>;
    async fn artifact(&self, id: &JobId, kind: ArtifactKind) -> Result<ArtifactRef, PlaneError>;
}
```

`probe` and `estimate` are **free**. `probe` must not hang on missing keys (5 s per device, 20 s total).

`wait` timeout is **our** budget. If `upstream_id` is set, map to `waiting_upstream` + return snapshot with `error_type=wait.timeout` on the **call**, job row stays non-terminal.

`artifact` returns a filesystem path or HTTP content URL under the job dir — **never** multi-MB base64.

### 7.1 PlaneCaps

```json
{
  "image_to_mesh": true,
  "text_native": false,
  "view_contract": false,
  "pbr": true,
  "preview_tier": true,
  "standard_tier": true,
  "high_tier": false,
  "ultra_tier": false,
  "cpu_ok": true,
  "devices": ["cpu"],
  "sync": true,
  "cancel": "supported",
  "licenses": ["MIT"],
  "max_input_bytes": 33554432,
  "estimated_vram_mb": null,
  "estimated_disk_mb": null
}
```

`cancel`: `supported | best_effort | unsupported`.

### 7.2 Planner (pure)

`plan(spec, ProbeSnapshot, SpendPolicy) -> PlaneChoice | Degrade`

Stable first-reason order:

```
feature_off
not_configured
weights_missing
license.blocked
device_missing
vram_short
disk_short
spend.gated
unsupported
```

Rules:

1. Analytic → Cadre live? else `analytic.unavailable`.
2. View Contract, views not on disk → T2I sub-plan (spend on **parent**). Mesh plane chosen independently.
3. Local candidates (in order): sidecar handshake+weights+licenses; preview feature if `quality=preview`; mock **only** if allow-mock.
4. Local feasible: weights ∧ licenses accepted ∧ (CPU ok or GPU VRAM ≥ floor) ∧ disk ∧ feature ∧ sidecar alive.
5. `prefer_device` set and missing → `device_missing` in `local` mode; `auto` may pick another **local** device, never a silent CPU quality run when the user pinned a GPU.
6. `mode=local` never calls remote. `mode=remote` never uses mock.
7. `auto` never selects mock unless allow-mock.
8. `ultra` never selected by auto quality rewrite.
9. Quality rewrite only in `mode=auto`, written into `degrades`.
10. Count **device VRAM** (`vram_mb`), never host RAM. Record `shared=true` when the probe says so. If `shared=true` **or** `vram_mb < 6144`, auto quality is **preview or remote or degrade** — never silent `standard`/`high`.
11. `remote.hunyuan_hosted` is a candidate **only** if every D19 gate is true. Even then, auto does not pick it when any of colony / tripo / meshy / local is feasible.
12. Feasible remotes, auto order: `remote.colony` (usd=0) → `remote.tripo` → `remote.meshy`. Never `remote.hunyuan_hosted` in that list unless step 11 applies **and** the others are infeasible.

VRAM / disk floors — **one number per pick** (`need_mb`). Amend only with field truth.

| Pick | Disk `need_mb` | VRAM `need_mb` |
|---|---|---|
| local preview | 2200 (preview weights) | 6144 GPU **or** CPU (`slow=true`) |
| local standard | **16384** (sum of named quality files × 1.1, min 16 GiB) | **24576** |
| local high/ultra | 16384 + cascade files × 1.1 | **24576** |
| remote | n/a | n/a |

Krackan 2026-08-19: AMD Radeon 840M, `vram_mb≈512`, `shared=true`, 22 GiB host RAM, no NVIDIA. Auto **must** `would_pick=remote` (if key+gate) or degrade — never local standard. Manifest examples must not show that iGPU as the device that ran a quality job.

Disk working set: `weights + 2048 MiB` free. Weight pull refuses if `free < want * 1.1`.

Catalog “supports route”:

| Plane | image | view_contract (hero) | view_contract (cardinals) | native text |
|---|---|---|---|---|
| local.mock | yes | yes (hero/front bytes) | no | no |
| local.sidecar | yes | yes | if handshake `view_contract` | no |
| local.preview | yes | yes | no | no |
| remote.tripo | yes | yes (hero→image) | yes (`front,left,back,right`) | yes if `allow_native_text` |
| remote.meshy | yes | yes | yes (multi-image) | yes if `allow_native_text` |
| remote.colony | same schema as us | same | same | same |
| remote.hunyuan_hosted | flagged | flagged | flagged | flagged |

### 7.3 Mandatory planner tests

Named tests **plus** the I/O table. FR-CMP-6 points **here**, not at research notes.

Fixture rows `(spec, ProbeSnapshot, SpendPolicy) → PlaneChoice | Degrade`:

| # | spec | probe | spend | expect |
|---|---|---|---|---|
| 1 | image, quality=standard, auto | CPU, no weights, no keys | closed | Degrade `weights_missing` (first reason) |
| 2 | image, standard, auto | quality weights, vram=24576, licenses ok | n/a | Local sidecar |
| 3 | image, standard, auto | no weights, `TRIPO_API_KEY` present | open, usd ok | Remote `remote.tripo` |
| 4 | image, standard, **local** | no weights, tripo key | open | Degrade `weights_missing` (never remote) |
| 5 | image, standard, auto | weights, vram=512, shared=true, tripo key | open | Remote tripo (not local standard) |
| 6 | image, standard, auto | weights, vram=512, shared=true, no keys | closed | Degrade `vram_short` or `spend.gated` |
| 7 | prefer_device=cuda, **local** | CPU only | n/a | Degrade `device_missing` |
| 8 | usd>0, auto, remotes feasible | no local | **closed** | `needs_confirm` / Degrade `spend.gated` |
| 9 | allow-mock=1, no weights, no keys | CPU | n/a | Local mock |
| 10 | quality=ultra, auto | weights+24 GB | n/a | rewrite to high **or** keep high; never auto-select ultra |
| 11 | hunyuan key only, no D19 | — | open | never `remote.hunyuan_hosted` |
| 12 | all D19 + hunyuan + tripo | no local | open | `remote.tripo` (not hunyuan) |

Also: `wait_timeout_remote_goes_waiting_upstream`, `wait_timeout_local_goes_failed`, `watchdog_queued_flips_failed`, `watchdog_needs_confirm_ttl`, `job_json_roundtrip_local_mock` (status=`degraded`), `job_json_roundtrip_http_mock`, `system_check_cpu_only_fixture` (`ready=false`), `feature_off_caps_hide_quality`, `analytic_absent_refuses`, `view_gate_fail_does_not_call_image3d`, `schema_drift_cli_mcp_openapi` (generated MCP `text2mesh_wait` default `timeout_s` **equals** CLI `--timeout-s` default **1800**), `export_not_ready_409`, `grey_default_material_not_succeeded`, `wrapper_ok_poll_running`.

---

## 8. State machine

```
queued
  ├─ preflight fail / watchdog.queue          → failed
  ├─ usd>0 && !allow_spend                    → needs_confirm
  ├─ remote POST accepted                     → submitted (upstream_id set)
  └─ local engine start                       → running

needs_confirm
  ├─ allow_spend + remote                     → submitted
  ├─ allow_spend + local                      → running
  └─ abort                                    → cancelled

submitted
  ├─ upstream executing                       → running
  └─ our poll window expired                  → waiting_upstream

running
  ├─ GLB clean                                → succeeded
  ├─ GLB + degrades                           → degraded
  ├─ error                                    → failed
  ├─ cancel ok                                → cancelled
  ├─ remote heartbeat stale + upstream_id     → waiting_upstream
  └─ local child **dead** (pid gone)          → failed engine.crash
     (silence with live pid is **not** a crash; director still heartbeats)

waiting_upstream
  ├─ poll resume / success                    → running | succeeded | degraded
  ├─ recover_ttl (24h)                        → failed wait.timeout
  └─ vendor cancel ok                         → cancelled
```

### 8.1 Watchdog (SQLite is truth)

Tick 15 s and on director start:

| Condition | Action |
|---|---|
| `queued` older than `queue_stale_secs` (60) with no worker | `failed` `watchdog.queue` |
| local `running`, **child pid dead** | `failed` `engine.crash` |
| local `running`, pid live, no progress line | **alive**; director emits parent heartbeat |
| remote `running`/`submitted`, heartbeat stale, `upstream_id` set | `waiting_upstream` |
| `waiting_upstream` older than `recover_ttl` (24h) | `failed` `wait.timeout` |
| `needs_confirm` older than `confirm_ttl` (24h) | `failed` `spend.gated` (no POST happened) |
| boot, local `running` | `failed` `engine.interrupted` |
| boot, remote `submitted`/`running`/`waiting_upstream` | resume poll |
| boot, `needs_confirm` within TTL | leave as-is |

`TEXT2MESH_HB_S` default **300** (5 min). Missing progress lines are **not** a crash if `pid` is live. Handshake and client wait **minimum** remain 30 s.

Director **must** emit parent heartbeats while waiting on T2I children or a sidecar child so the parent cannot die of silence.

### 8.2 Nested T2I children (View Contract)

Parent `running` until children terminal. Paid T2I child `waiting_upstream` → parent stays `running`. Required view fail after ladder (any of G1–G4) → parent `failed` with the **specific** gate `error_type`; wrap `view.consistency` on exhaust; **Image3dPlane is not submitted**.

Children are **not** `MeshJob`s in v1. They are `child_jobs[]` of `{ id, kind: "t2i", provider, upstream_id, status, usd }`. They do **not** appear in `text2mesh_list_jobs` unless `include_children=true`. Spend still rolls up to the parent.

### 8.3 Cancel

| Plane | Behaviour |
|---|---|
| mock | immediate `cancelled` |
| sidecar | SIGTERM, then SIGKILL after `cancel_grace` ≥ 30 s; if still alive → `failed` `engine.crash` |
| remote | vendor cancel if `caps.cancel=supported`; else `cancel_requested=true`, state unchanged, UI honest |
| needs_confirm | `cancelled` (no spend) |

---

## 9. Honest degrades (catalogue)

Every row is a **stated** outcome, never a silent success.

| Situation | Status | `error_type` / degrade |
|---|---|---|
| No key, paid path | `failed` | `not_configured` |
| Key length 0 | `failed` | `not_configured` |
| Weights missing | `failed` | `weights_missing` |
| Feature not compiled | `failed` | `feature_off` |
| User pinned CUDA, none | `failed` | `device_missing` |
| VRAM < floor, mode=local | `failed` | `vram_short` |
| `auto` steps high→standard | `degraded` | `quality.step_down` |
| CPU quality | `degraded` or slow `running` then same | `compute.cpu_slow` |
| Spend gate closed, usd>0 | `needs_confirm` or `failed` | `spend.gated` |
| Estimate > max_usd | `failed` | `spend.estimate_exceeded` |
| Vendor 402 | `failed` | `spend.provider_402` |
| DINOv3 on disk, not accepted | `failed` | `license.dinov3_unaccepted` |
| Hunyuan without attestation | `failed` | `license.blocked` |
| print_wrap, no non-GPL | `failed` | `license.print_wrap_unavailable` |
| Cadre absent, Route A | `failed` | `analytic.unavailable` |
| Out of analytic grammar | `failed` | `analytic.too_complex` |
| No T2I, Route B | `failed` | `t2i.unavailable` |
| Required view fails any of G1–G4 after retries | `failed` | specific gate `error_type`; wrap `view.consistency` on exhaust |
| Optional top dropped | `degraded` or continue | `cameras_dropped` |
| I2I missing, N independent T2I | continue + degrade | `synthesis.independent_t2i` |
| Engine no PBR, vertex colour | `degraded` | `export.material_mode` |
| Engine no colour at all | `failed` | `export.materials_missing` |
| Remote no PBR | `degraded` | `remote.material_fidelity` |
| Local wait timeout | `failed` | `wait.timeout` |
| Remote wait timeout + id | `waiting_upstream` | snapshot `wait_timed_out=true`; wrapper `ok=true` |
| Mock / vertex-colour GLB | `degraded` | `export.material_mode` |
| GET artifact while non-terminal | HTTP 409 | `export.not_ready` |
| Sidecar crash | `failed` | `engine.crash` |
| Reboot mid local job | `failed` | `engine.interrupted` |
| Mock without allow-mock | not selected | planner degrade, not a job |
| Vendor 429 | stay running / retry-after | `rate_limit` on the **call** |

HTTP mapping:

| Class | Status |
|---|---|
| `spec.rejected` | 400 |
| `not_configured`, `weights_missing`, `feature_off` | 409 or 424 (use **409** + `error_type`; do not invent 424 if clients are dumb — **Recommended: 409**) |
| `spend.gated`, `license.*` | 403 |
| `spend.provider_402`, `spend.estimate_exceeded` | 402 |
| `rate_limit` | 429 + `Retry-After` |
| job not found | 404 |
| `internal` | 500 |
| create job | **202** + body |
| create job | **202** `{ ok: true, job_id, status, poll_url }` — **no** `artifact_url` |
| poll existing job (any status) | **200** `{ ok: true, job: { status, … } }` — wrapper `ok` = found, **not** meshed |
| GET artifact, job not `succeeded`/`degraded` | **409** `export.not_ready` |
| GET artifact, terminal with GLB | **200** bytes or file |

Wrapper `ok=true` on poll/202 never implies a GLB. Mesh success is only `job.status == succeeded`.

---

## 10. MCP tools

Transport: newline-delimited JSON-RPC over stdio. Protocol `2024-11-05`. Implementation string `hand-rolled`. Frame cap 32 MiB. Notifications: no response. Echo `id` exactly.

Tool failure = result with `isError: true` and text + structured JSON in content. Do not use JSON-RPC error for business failures.

### 10.1 `text2mesh_system_check`

Args: `{ "refresh": false }`
Free. Returns `text2mesh.system_check.v1` (see §13).

### 10.2 `text2mesh_estimate`

Args: same input subset as submit (`prompt` / `image_path`, `quality`, `route`, `compute`, `camera_preset`, `max_usd`).
Free. Never paid. Returns `text2mesh.estimate.v1`.

### 10.3 `text2mesh_compile_contract`

Args: `{ "prompt": "...", "quality": "standard", "camera_preset": null, "seed": null }`
Pure. No T2I. Returns `{ contract, contract_hash, class, compile_notes }`.

### 10.4 `text2mesh_submit`

Args: **`JobSubmit`** (§3.4). Mints `job_id`, persists, returns `{ ok: true, job }`. If usd>0 and `allow_spend!=true` → persist `needs_confirm` (still a `job_id`). Confirm: same tool with `job_id` + `allow_spend: true`.

### 10.5 `text2mesh_status`

Args: `{ "job_id": "…" }`
Non-blocking snapshot.

### 10.6 `text2mesh_wait`

Args: `{ "job_id": "…", "timeout_s": 1800 }`
`timeout_s` min **30**, default **1800**, max 86400. Same bounds as CLI `--timeout-s` and Route B `max_wall_s`. Schema-drift (PR-01b) asserts the generated MCP tool default equals the CLI default.
**Frozen:** wrapper `ok=true` whenever the RPC succeeds (job exists). JSON-RPC / `isError` only for unknown `job_id` or protocol breakage.
If still non-terminal when the wait budget ends: `wait_timed_out=true`, `error_type=wait.timeout` on the **snapshot**, job row **unchanged** (may be `running` or `waiting_upstream`). Agents loop on `job.status`, not on wrapper `ok`.

`tools/list` stays live (status/wait split).

### 10.7 `text2mesh_cancel`

Args: `{ "job_id": "…" }`

### 10.8 `text2mesh_artifact`

Args: `{ "job_id": "…", "kind": "glb", "view_id": null }`
Returns `{ "path": "…", "sha256": "sha256:…", "bytes": 12345, "media_type": "model/gltf-binary" }`.
**Path, not blob.**

### 10.9 `text2mesh_list_jobs`

Args: `{ "status": null, "limit": 20, "include_children": false }`
`limit` 1..=100, default 20, newest first. Children hidden unless `include_children`.

No `weights pull` on MCP in v1 (CLI only — multi-GB, long stderr progress).

---

## 11. HTTP routes

Default bind `127.0.0.1:8796` (`TEXT2MESH_BIND`). Non-loopback requires `Authorization: Bearer $TEXT2MESH_TOKEN`.

| Method | Path | Auth | Notes |
|---|---|---|---|
| GET | `/v1/health` | no | `{ "ok": true, "version": "…" }` |
| GET | `/v1/system-check` | loopback free | `?refresh=1` |
| POST | `/v1/estimate` | same | free |
| POST | `/v1/contracts` | same | compile only |
| POST | `/v1/jobs` | same | 202 `{ job_id, status, poll_url }` only. JSON JobSubmit or multipart (`image` + `spec`) |
| GET | `/v1/jobs` | same | list |
| GET | `/v1/jobs/{id}` | same | 200 snapshot |
| POST | `/v1/jobs/{id}/cancel` | same | |
| POST | `/v1/jobs/{id}/confirm` | same | sets allow_spend, starts fire |
| GET | `/v1/jobs/{id}/artifact` | same | `?kind=glb\|manifest\|contract\|view&view_id=` |
| GET | `/v1/jobs/{id}/events` | same | optional SSE `text/event-stream` |
| GET | `/v1/openapi.json` | no on loopback | generated |
| GET | `/` | loopback | HTMX studio (always on in `-api`; off-loopback 404) |

SSE events:

```
event: progress
data: {"stage":"orbit","pct":40,"message":"i2i left"}

event: status
data: {"status":"running"}
```

Colony / CI HTTP mock implements the **same** `/v1/jobs` surface.

---

## 12. CLI

Binary `text2mesh`. clap derive. Global `--json`, `--store`, `--allow-spend`.

```
text2mesh system-check [--json] [--refresh]
text2mesh estimate (--prompt TEXT | --image PATH) [job flags] [--json]
text2mesh compile --prompt TEXT [--quality] [--preset] [--out FILE] [--json]
text2mesh generate (--prompt TEXT | --image PATH) [job flags] [--allow-spend] [--json]
text2mesh confirm JOB [--allow-spend]
text2mesh status JOB
text2mesh wait JOB [--timeout-s 1800]   # min 30, default 1800, max 86400
text2mesh cancel JOB
text2mesh artifact JOB [--kind glb]
text2mesh jobs [--status] [--limit]
text2mesh weights pull ID --accept-license TAG
text2mesh mcp            # exec stdio server (or point .mcp.json at text2mesh-mcp)
text2mesh serve          # exec API on TEXT2MESH_BIND
```

`generate` = submit + wait (`--timeout-s` min 30, default **1800**, max 86400). S2/PR-04 **must** pass `--compute local --provider local.mock` (auto still forbidden without allow-mock).

Job flags: every `JobSubmit` field (§3.4), including `--allow-native-text`, `--license-override`, `--max-wall-s`, `--max-credits`, export flags.

Exit codes (stable):

| Code | When |
|---|---|
| 0 | `status=succeeded` only |
| 1 | `status=degraded` (print `DEGRADED` to stderr; JSON has `status` + `degrades`) |
| 2 | usage |
| 3 | `not_configured` / `weights_missing` / `feature_off` |
| 4 | spend / license |
| 5 | engine / upstream |
| 6 | `view.consistency` / analytic refuse |
| 7 | cancelled |
| 8 | wait budget ended; inspect JSON `wait_timed_out` / `status` |
| 9 | internal |

`system-check` exits 0 if `report_complete=true`. Readiness = `ready` (`planner.would_pick != null`).

---

## 13. system-check

Schema `text2mesh.system_check.v1`.

```json
{
  "schema": "text2mesh.system_check.v1",
  "report_complete": true,
  "ready": false,
  "product": "text2mesh",
  "version": "0.1.0",
  "features": {
    "compiled": ["remote-http"],
    "not_compiled": ["sidecar", "preview-onnx", "preview-candle", "gate-clip", "webui"],
    "horizon_unscheduled": ["quality-candle", "quality-ggml"]
  },
  "devices": [
    { "kind": "cpu", "ok": true, "slow": true, "vram_mb": null, "shared": false },
    { "kind": "nvidia.cuda", "ok": false, "reason": "nvidia-smi not found" },
    { "kind": "amd.rocm", "ok": false, "reason": "rocminfo not found" },
    { "kind": "gpu.vulkan", "ok": true, "name": "AMD Radeon 840M", "vram_mb": 512, "shared": true, "slow": true },
    { "kind": "apple.metal", "ok": false, "reason": "not macos" }
  ],
  "weights": [],
  "licenses": {
    "dinov3_accepted": false,
    "hunyuan_community": "blocked_by_default",
    "cgal_gpl": "blocked_by_default"
  },
  "keys": [
    { "id": "MESHY_API_KEY", "present": false, "len": 0, "head": null },
    { "id": "TRIPO_API_KEY", "present": true, "len": 48, "head": "tsk_" },
    { "id": "TEXT2MESH_TOKEN", "present": false, "len": 0, "head": null },
    { "id": "XAI_API_KEY", "present": false, "len": 0, "head": null, "note": "must not be read by this process" }
  ],
  "sidecars": [],
  "siblings": [
    { "id": "imaginarium", "url": "http://127.0.0.1:8791", "ok": false, "reason": "not probed" },
    { "id": "cadre", "url": "http://127.0.0.1:7410", "ok": false, "reason": "not probed" }
  ],
  "planner": { "mode": "auto", "would_pick": null, "degrade": { "error_type": "vram_short", "message": "shared iGPU 512 MiB; no remote key with spend gate open" } },
  "spend": { "gate": "closed", "spent_today_usd": 0.0, "max_usd_per_job": 2.0, "max_usd_per_day": 10.0 }
}
```

Key row: `present`, `len`, `head` (2–4 chars). Never print an `XAI_API_KEY` head. Report `present=false` **always** from *our* env (we do not load it). If a careless operator exported it into this process, still do not **use** it; may warn `leaked_into_process=true` without printing the secret.

CI fixture: CPU-only, empty weights, `report_complete=true`, `ready=false`.

Weight row:

```json
{
  "id": "encoder.dinov3_vitl16",
  "present": false,
  "want_bytes": 607000000,
  "have_bytes": null,
  "path": "~/.local/share/text2mesh/weights/encoder.dinov3_vitl16",
  "sha256_head": null,
  "license": "DINOv3",
  "accepted": false
}
```

Weight ids we name (ours, not their filenames as API):

| id | Role | Disk ballpark | License |
|---|---|---|---|
| `preview.feedforward` | Preview-class feedforward (if a MIT pack is wired) | measure on pull | MIT |
| `quality.stack` | Quality image-3D pack (user sidecar) | **≥16 GiB** class; sum actual files | MIT + maybe DINOv3 |
| `encoder.dinov3_vitl16` | Optional conditioning encoder | measure on pull | DINOv3 |
| `encoder.openclip_vit_b32` | Gate encoder | measure on pull | MIT/OpenCLIP |
| `native.text_dit` | Optional Route C local text weights | measure on pull | MIT |

A public GGUF pack, if used, is a **weight option** with license + hash in `system-check`, not our stage graph. Do not name vendor checkpoint filenames as API. `native.text_dit` is a **weight id**, not a `PlaneId` — Route C local uses `local.sidecar` (or refuse). No `local.trellis_text` plane.

`system-check` sums **actual** bytes.

---

## 14. Estimate

Schema `text2mesh.estimate.v1`.

```json
{
  "schema": "text2mesh.estimate.v1",
  "ok": true,
  "plane": "remote.tripo",
  "usd": 0.54,
  "usd_uncertain": false,
  "credits": 30,
  "credit_unit": "tripo",
  "seconds_p50": 420,
  "tier": "standard",
  "views": 6,
  "breakdown": [
    { "step": "t2i.hero", "usd": 0.04, "n": 1 },
    { "step": "t2i.orbit", "usd": 0.20, "n": 5 },
    { "step": "t2i.reserved_retries", "usd": 0.06, "n": 3 },
    { "step": "mesh.tripo", "usd": 0.30, "n": 1 }
  ],
  "caps": { "max_usd_per_job": 2.0, "max_usd_per_day": 10.0, "spent_today": 0.0 },
  "gate": "closed"
}
```

I2I units: call `T2iProvider::estimate`. **Do not hardcode 2×** (OQ-9). Reserved retries = `max_orbit_edits * i2i_unit * 0.5`.

`seconds_p50` **must** include N T2I + reserved retries + mesh, not mesh alone. Local mesh `usd=0`; conservative catalog (amend with field truth): preview GPU 30 s; local standard **only if vram_mb≥24576** 400 s; CPU preview 600 s; CPU quality **7200 s** (`slow=true`). Shared iGPU is not a standard device.

---

## 15. Manifest

Schema `text2mesh.manifest.v1`. Written atomically next to the GLB. Fields: PRD §13.1. Additional required:

- `gate_version`, `gate_scores`, `cameras_dropped`
- `synthesis`
- `child_jobs: [{ id, kind, provider, upstream_id, status, usd }]`
- `ok`: true **only** if `status=succeeded`
- `weight_files: [{ id, path, sha256, license }]`
- `sidecar_protocol: "meshplane/1" | null`
- `disclaimer`: `"not-a-model"` for mock, else null

No secrets. Key material never appears.

---

## 16. Store layout

```
$TEXT2MESH_STORE/                  # default: $XDG_DATA_HOME/text2mesh  or ~/.local/share/text2mesh
  jobs.sqlite
  jobs.sqlite-wal
  weights/                         # or $XDG_DATA_HOME/text2mesh/weights
  jobs/
    <job_id>/
      job.json
      manifest.json
      contract.json                # if any
      input/original.bin
      input/conditioned.png
      views/<id>.png
      views/scores.json
      scratch/                     # sidecar only; confined
      analytic/source.star         # Route A
      artifact.glb
      artifact.glb.sha256
      artifact.step                # Route A if Cadre emitted it
      extras/                      # optional Gaussian/NeRF/etc iff engine emitted them (OQ-4 b); not SUCCESS
      log.stderr.txt               # sidecar/vendor tails, redacted
```

`TEXT2MESH_STORE=""` → `tempfile::TempDir`, wiped on exit. Job ids still ULIDs; not durable.

Writes: `*.tmp` + `rename`. Startup: ignore dirs missing `job.json` or with `*.tmp` leftovers.

---

## 17. Sidecar `meshplane/1`

Preferred: **stdio NDJSON**. Logs on stderr. Optional: loopback HTTP + bearer file 0600.

Parent creates `scratch/` and `views/` and passes **absolute** paths under the job dir.

Handshake (child → parent, first line, 30 s):

```json
{
  "protocol": "meshplane/1",
  "engine": "user-engine-name",
  "version": "1.2.3",
  "caps": { "image_to_mesh": true, "pbr": true, "tiers": ["preview", "standard"] },
  "licenses": ["MIT"],
  "devices": ["cpu", "gpu.vulkan"]
}
```

Parent → child:

```json
{ "op": "submit", "job": { /* JobSpec */ }, "paths": {
  "conditioned": "/…/input/conditioned.png",
  "scratch": "/…/scratch",
  "out_glb": "/…/artifact.glb"
}}
```

Child → parent:

```json
{ "op": "progress", "stage": "form", "pct": 55, "message": "form fields" }
{ "op": "artifact", "kind": "glb", "path": "/…/artifact.glb" }
{ "op": "fail", "error_type": "engine.oom", "message": "…" }
{ "op": "pong" }
```

Rules:

- Path must be canonical and under `scratch/` or the job dir. Else `engine.crash`.
- Protocol mismatch → `unsupported`.
- No handshake in 30 s → `not_configured`.
- Exit ≠ 0 → `engine.crash`.
- We never import `.t2mesh` / `.dinodata` / `t2_*`. User adapter translates.
- Handshake `licenses` containing `cgal` or `gpl` → `system-check` warn; we still do not bundle GPL.

---

## 18. Mock engine

Always compiled.

- Input: conditioned bytes or prompt string.
- Output: valid GLB, 1 mesh, vertex colours, no external `.bin`.
- Contents = deterministic function of `sha256(input_bytes || seed_le_bytes)`.
- Completes <50 ms. Walks `queued → running → degraded`.
- Manifest `engine=mock`, `quality=preview`, `disclaimer=not-a-model`, `export.material_mode=vertex_color`, `ok=false`, `status=degraded`.
- Golden hash pinned in `crates/text2mesh/tests/golden/mock.glb.sha256`. Golden test: default-material grey mesh must **not** be `succeeded`.

HTTP mock provider: same `/v1/jobs` as §11. Sequences:

1. `queued → running → degraded` + tiny vertex-colour GLB
2. `queued → submitted → waiting_upstream → degraded`
3. 402 after a present key (never use 401 for missing key — we fail before POST)
4. GET artifact while `queued` → 409 `export.not_ready`

---

## 19. T2iProvider

```rust
#[async_trait]
pub trait T2iProvider: Send + Sync {
    fn id(&self) -> T2iProviderId;
    fn caps(&self) -> T2iCaps; // t2i, i2i, max_ref_images, estimate_usd
    async fn probe(&self) -> ProbeReport;
    async fn estimate(&self, req: &T2iEstimateReq) -> Result<CostEstimate, PlaneError>;
    async fn generate(&self, req: &T2iGenReq) -> Result<ImageRef, PlaneError>;
    async fn edit(&self, req: &T2iEditReq) -> Result<ImageRef, PlaneError>;
}
```

Never send them a key we do not have. Edit sources ≤ 3. Hero + nearest neighbor.

### 19.1 Imaginarium wire (public routes only)

Base: `TEXT2MESH_IMAGINARIUM_URL` default `http://127.0.0.1:8791`. Optional `TEXT2MESH_IMAGINARIUM_TOKEN`. **Never** `XAI_API_KEY`.

Probe (5 s): sibling public `GET /health`, then `GET /v1/health` if the first 404s.

v1 director is **blocking**. The `T2iProvider` impl is sync (`reqwest` blocking); the sketch above stays the shape. Tests inject a loopback fake. Live calls require `allow_spend` and skip unless `TEXT2MESH_LIVE=1`.

| Our op | Their route | We send | We read |
|---|---|---|---|
| estimate | `POST /v1/estimate` | sibling `{ kind:"image", model, n }` where `n = n_t2i + n_i2i` (no distinct I2I unit → `usd_uncertain` when `n_i2i>0`, OQ-9) | `estimated_usd` |
| hero T2I | `POST /v1/images/generations` | assembled hero prompt + 1:1 / 1k / model `2.0` | `assets[0].content_url` (or `upstream_url`) → `views/hero.png` |
| orbit I2I | `POST /v1/images/edits` | `images`: `library:{job_id}` and/or `data:image/png;base64,…` (≤3; **no bare paths**) + orbit prompt | write `views/<id>.png` |

Timeouts: per-image uses **job** budget (≥30 s), not probe 5 s. Errors: 402 → `spend.provider_402`; missing sibling → `t2i.unavailable`. Our `max_usd` = min(ours, their caps). Paid T2I on a non-mock plane with `usd>0` and a closed gate → `needs_confirm`. `local.mock` keeps the $0 mock T2I.

### 19.2 Cadre wire (public routes only)

Probe: `GET {TEXT2MESH_CADRE_URL}/v1/health` or `cadre --version` if `TEXT2MESH_CADRE_BIN` set. 5 s.

| Our op | Their route / CLI | We send | We read |
|---|---|---|---|
| write | `POST /v1/build` after placing source, or CLI `cadre` HTTP write (never stdio `write_source` unless user flipped Cadre’s flag) | `jobs/<id>/analytic/source.star` | job/build id |
| export GLB | `POST /v1/export` `format=glb` | build id | bytes → `artifact.glb` |
| export STEP | `POST /v1/export` `format=step` | build id | `artifact.step` if present |

Poll vs sync: if they return a job id, poll their status with **job** timeouts (≥30 s). 5 s probe is only health. Missing both URL and bin → `analytic.unavailable`.

### 19.3 Meshy / Tripo wire (public routes only)

v1 director is **blocking**. Tests inject loopback fixtures. Live POSTs require a present key **and** `allow_spend`. Missing key → `not_configured` **before** any POST (never a 401 for absence). Live tests skip unless `TEXT2MESH_LIVE=1`.

**Meshy** (`TEXT2MESH_MESHY_URL` default `https://api.meshy.ai`, key `MESHY_API_KEY`):

| Our op | Their route | We send | We read |
|---|---|---|---|
| image | `POST /openapi/v1/image-to-3d` | `image_url` data-URI + `target_formats=["glb"]` | `result` task id |
| native text (`allow_native_text`) | `POST /openapi/v2/text-to-3d` | `{mode:preview, prompt}` | `result` |
| poll | `GET …/{id}` | — | `status` `PENDING\|IN_PROGRESS\|SUCCEEDED\|FAILED\|CANCELED`; `model_urls.glb` |

**Tripo** (`TEXT2MESH_TRIPO_URL` default `https://openapi.tripo3d.ai/v3`, key `TRIPO_API_KEY`):

| Our op | Their route | We send | We read |
|---|---|---|---|
| image | `POST /generation/image-to-model` | `file.type=data_url` | `data.task_id` |
| native text | `POST /generation/text-to-model` | `{prompt, model}` | `data.task_id` |
| multiview (cardinals) | `POST /generation/multiview-to-model` | named `front,left,back,right` | `data.task_id` |
| poll | `GET /tasks/{id}` | — | `status=success\|failed\|…`; `output.model_url` |

HTTP: **402** → `spend.provider_402` (job `failed`); **429** → `rate_limit` + `Retry-After` hint (no silent retry loop). Poll window expiry with `upstream_id` → `waiting_upstream`. Fixture GLB is the mock vertex-colour mesh → `degraded` `export.material_mode`. Hunyuan stays inert.

---

## 20. Image preprocess

Pure.

1. Reject if **compressed** upload > **32 MiB** → `spec.rejected`.
2. Decode PNG/JPEG only. Other → `spec.rejected`.
3. Reject if long edge > **4096** or uncompressed decode buffer > **64 MiB** (4096² × 4). No auto-scale in v1. Field `image.scaled` is **absent** until a CHARTER amendment adds scaling.
4. Alpha-aware bbox crop + pad to square, 8 px margin (does not change the long-edge cap; crop happens after the reject checks).
5. Write `input/original.bin` (raw upload) and `input/conditioned.png`. Hash both SHA-256.

---

## 21. Export

1. Engine writes GLB to `artifact.glb.tmp`.
2. Parse with `gltf` crate. Fail → `export` error / `engine.crash`.
3. Inspect materials. **Default-only** metallic-roughness factors (glTF spec defaults) with no `COLOR_0` variation and no textures → `failed` `export.materials_missing`.
4. `COLOR_0` and/or factors-only (no UV atlas) → `material_mode=vertex_color` or `factors_only`, status **`degraded` always** (including `quality=preview` and mock). No preview exception.
5. Alpha: wire `BLEND`/`MASK` or honour `force_opaque`.
6. `unit_cube` / `uv_atlas` only if the **job flag is true** (default false). Record transform if applied.
7. `rename` to `artifact.glb`. Write `artifact.glb.sha256` + `manifest.json` (`ok` iff `succeeded`).
8. Do not claim `watertight` unless a wrap stage ran.

---

## 22. Env vars

| Name | Default | Notes |
|---|---|---|
| `TEXT2MESH_BIND` | `127.0.0.1:8796` | D27 / OQ-6 locked |
| `TEXT2MESH_TOKEN` | unset | required if bind ≠ loopback |
| `TEXT2MESH_STORE` | XDG data `text2mesh` | `""` = ephemeral |
| `TEXT2MESH_ALLOW_SPEND` | unset/0 | gate |
| `TEXT2MESH_ALLOW_MOCK` | unset/0 | planner |
| `TEXT2MESH_ALLOW_UNGATED` | unset/0 | Route B without CLIP |
| `TEXT2MESH_ALLOW_HUNYUAN` | unset/0 | D19 |
| `TEXT2MESH_HUNYUAN_ATTESTATION` | unset | 0600 file |
| `TEXT2MESH_ACCEPT_DINOV3` | unset/0 | D20 |
| `TEXT2MESH_MAX_USD_PER_JOB` | `2.0` | |
| `TEXT2MESH_MAX_USD_PER_DAY` | `10.0` | |
| `TEXT2MESH_SIDECAR` | unset | `meshplane/1` binary |
| `TEXT2MESH_IMAGINARIUM_URL` | `http://127.0.0.1:8791` | compose |
| `TEXT2MESH_MESHY_URL` | `https://api.meshy.ai` | public Meshy base |
| `TEXT2MESH_TRIPO_URL` | `https://openapi.tripo3d.ai/v3` | public Tripo v3 base |
| `TEXT2MESH_CADRE_URL` | unset | e.g. `http://127.0.0.1:7410` |
| `TEXT2MESH_CADRE_BIN` | unset | `cadre` on PATH if set empty-and-found |
| `TEXT2MESH_IDLE_UNLOAD_S` | `120` | |
| `TEXT2MESH_QUEUE_STALE_S` | `60` | |
| `TEXT2MESH_HB_S` | `300` | progress stale **minutes**; pid-live ≠ crash |
| `TEXT2MESH_RECOVER_TTL_S` | `86400` | also idempotency window |
| `TEXT2MESH_CONFIRM_TTL_S` | `86400` | `needs_confirm` → `spend.gated` |
| `TEXT2MESH_CANCEL_GRACE_S` | `30` | |
| `MESHY_API_KEY` | unset | length/head only |
| `TRIPO_API_KEY` | unset | |
| `TEXT2MESH_LIVE` | unset | live tests |
| `TEXT2MESH_LOG` | `text2mesh=info` | tracing filter |
| `XAI_API_KEY` | — | **must not be used** |

`remote.custom` is **not** a v1 `PlaneId` (dropped). Colony + Meshy + Tripo only.

Config `~/.config/text2mesh/config.toml` is a 1:1 of the non-secret env keys above (`bind`, `store`, `allow_spend`, caps, URLs, TTLs). **Env wins.** Secrets stay in `~/.config/text2mesh/env` or `/etc/text2mesh/env` mode **0600**.

---

## 23. Cargo features

| Feature | Default | Links |
|---|---|---|
| *(none)* | yes | tokio, reqwest rustls, rusqlite, serde, axum in `-api` |
| `remote-http` | **default** | Meshy/Tripo/colony adapters (inert without keys) |
| `sidecar` | off | process + meshplane |
| `preview-onnx` | off | `ort` |
| `preview-candle` | off | candle-core CPU (tiny preview only) |
| `cuda` / `metal` / `vulkan` | off | still probed |
| `gate-clip` | off | OpenCLIP |
| `webui` | **on** (S11) | HTMX studio in `-api` (`GET /`) |

**Horizon, do not schedule in v1 `Cargo.toml`:** `quality-candle`, `quality-ggml`. Adding them needs CHARTER D28 amendment.

Default workspace tests **do not** enable sidecar/preview/gate-clip.

---

## 24. Timeouts

**Probe budgets** (the only sub-30 s timers; **must not** be reused for sidecar generate or vendor poll):

| Call | Default | On expiry |
|---|---|---|
| device probe | 5 s each, 20 s total | `unavailable` |
| estimate local catalog | 0 ms | — |
| estimate remote refresh | 10 s stale-ok | use cache |
| sibling health | 5 s | sibling `ok=false` |

**Job budgets** (all ≥ 30 s):

| Call | Default | Floor |
|---|---|---|
| job_status HTTP | 30 s | 30 s |
| job_wait | **1800 s** (max 86400) | 30 s |
| sidecar handshake | 30 s | 30 s |
| sidecar generate | heartbeat; pid-live = alive | no short timeout |
| T2I per image | provider; parent `max_wall_s` | 30 s |
| Route B `max_wall_s` | **1800 s** (max 86400) | 30 s |
| image-only preview / mock wall | 180 s Nano optional | 30 s |
| `recover_ttl` / `confirm_ttl` | 24 h | — |

**Nano detection:** `system-check` `tier=nano` when no sidecar feature, no quality weights, and (`vram_mb` null or `< 6144` or `shared=true`). The **180 s** cap applies only to **mock / single-image preview**, never Default-6 Route B.

---

## 25. Security

- Loopback default. Non-loopback: bearer token, `subtle` compare, 32+ random bytes.
- Sidecar: no `..`, canonical paths, no provider key env passthrough.
- Redact secrets in `log.stderr.txt` (patterns `sk-`, `tsk_`, `Bearer `, `xai-`).
- MCP stdio TCB = the harness.
- No telemetry.

---

## 26. Crate / module sketch (S0)

```
crates/
  text2mesh/src/
    lib.rs
    job.rs
    error.rs
    contract.rs
    compiler.rs
    router.rs
    gate.rs
    planner.rs
    director.rs
    store.rs
    watchdog.rs
    spend.rs
    planes/{mod,mock,sidecar}.rs
    t2i/{mod,mock}.rs
    export.rs
    preprocess.rs
    system_check.rs
  text2mesh-cli/src/main.rs
  text2mesh-mcp/src/main.rs
  text2mesh-api/src/main.rs
```

v1 workspace is **these four crates only**. Adapter/sidecar/store code lives as **modules** in `text2mesh` (D30). No `-provider` / `-engine` / `-io` / `-slint` members.

S0 may ship stubs that return `feature_off` / `not yet` honestly.

---

## 27. Eval

Checked-in fixtures (same files PR-10 / PR-13 / PR-11 / PR-21):

| File | Owner PR | Role |
|---|---|---|
| `evals/text2/prompts.json` | PR-10 | 24 prompts, 8 each `{creature, product, prop}` |
| `evals/text2/identity.json` | PR-10 | `(prompt, identity_phrase, class)` remainder table |
| `evals/text2/classify.json` | PR-13 | classifier fixture (same 24 + edge cases) |
| `evals/text2/species.txt` | PR-13 | closed creature-token list; inline tokens **must match** this file |
| `evals/text2/scores/` | PR-11 may add | precomputed G0–G4 fixtures when `gate-clip` is off |

Primary metric (PRD M3): G0∧G1∧G3 (+G2 if classed) pass rate ≥ +20 pp vs naive independent T2I at the same cameras/spend band. If naive ≥80%, Janus fail-rate ≤ ½ naive. S5 goldens are a function of §4.1 class locks + §4.3 `prompt_suffix` — one expected JSON per prompt.

No live network in CI. Live 3D CLIP-T is skip-loud.

---

## 28. What implementers must not do

1. Open forbidden source trees (`AI_trellis2cpp`, `rms80/trellis2cpp`, Microsoft TRELLIS / TRELLIS.2 Python, Hunyuan3D, TripoSR, Meshy implementation). **Do not follow PRD Appendix B URLs.**
2. Name public APIs after voxel exponents or their stage ids. Stage names here are **progress vocabulary**, not an in-process graph to reimplement in S0–S11.
3. Ship `t2_*` C ABI or `.t2mesh` containers. Outbound ABI, if ever, is `mesh_abi_v1` after the D1 sweep PR.
4. Auto-pull multi-GB weights on first generate.
5. Auto-select mock or Hunyuan.
6. Put GLB bytes on MCP stdout.
7. Use Python at runtime.
8. Link CGAL or imaginarium-slint.
9. Stack PRs. After every merge: `git fetch && git checkout -b … origin/main`.
10. Change this contract without updating the file in the same commit.
11. Treat `docs/research/*` as implementation specs.
12. Schedule `quality-candle` / `quality-ggml` as v1 slices.
13. Treat Gaussian/NeRF as a second success definition or first-class DCC. Extra files under `jobs/<id>/extras/` are allowed if an engine emits them (OQ-4 b). SUCCESS remains `artifact.glb` + core PBR.
