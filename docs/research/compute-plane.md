# Compute plane — dual-path requirement

> Research note for PRD writers. Not the PRD. Invents an original `ComputePlane`
> contract so the same `MeshJob` runs on local/onboard inference **or** networked
> providers. Clean-room: public cards, papers, garden doctrine. No reference-project
> source, C ABI, or container names.
>
> Date: 2026-08-19 · Workspace: `docs/research/` · Binding house: Launchpad-RS
> doctrine #3 (honesty), #8 (spend gated), #9 (never orphan spend), stack
> Nano-first.

---

## 0. Hard requirement (do not weaken)

v1 ships **one** `ComputePlane` trait and **at least two** live implementations:

1. **Local / onboard** — weights on disk + a local engine (preview feedforward
   and/or quality DiT). CPU is allowed and **slow, honestly**. GPU is a
   *capability probe* (CUDA / ROCm / Vulkan / Metal), never a `#ifdef` product fork.
2. **Networked** — HTTP(S) providers with the same job schema (submit → poll →
   artifact). Meshy / Tripo / custom OpenAPI-ish adapters. A Colony sibling on
   LAN is a **provider**, not a special case.

A **planner** (`local` | `remote` | `auto`) chooses a plane. Auto is **not** a
third plane.

Doctrine translated:

| Rule | Compute-plane meaning |
|------|------------------------|
| No fake success | Empty GLB, missing texture, or a mocked mesh must never report `ok: true` as a quality result. Mock success is tagged `engine=mock` and is a test/demo path only. |
| Stated degrade | Missing weights / missing key / CPU-only / feature compiled out / spend gate closed are **distinct** structured errors. |
| Jobs never stuck `pending` | `pending` is not a dump state. Every path ends in a terminal state **or** a recoverable wait with an id + resume recipe. |
| Missing key ≠ timeout | `NotConfigured` immediately. Never wait 30s to discover a missing key. |
| Spend gated | Estimate **before** any paid POST. Live-fire is explicit. Tests mock upstream. |
| Never orphan spend | A paid remote job that outlives *our* poll window stays recoverable (`waiting_upstream`), not silently `failed`. |
| Nano-first | Default build has **no** heavy inference runtime. No timeout < 30s. Never assume keys or 16 GB VRAM. |
| Probe, don't assume | `system-check` reports compiled features, devices, weights, licenses, keys (length/head), sidecar handshake. INSTALLED ≠ ACTIVE. |

---

## 1. Recommended locks for CHARTER (writer may rename, not silently drop)

**(a) Recommended.**

| # | Topic | Lock |
|---|--------|------|
| C1 | Trait | One `ComputePlane`. Faces (MCP/CLI/HTTP) never talk to Meshy/CUDA/sidecar directly. |
| C2 | Planes in v1 | `LocalPlane` + `RemotePlane`. `Auto` is a pure planner over probes. |
| C3 | Job identity | Local ULID `job_id` is primary. Provider `upstream_id` is secondary, nullable. |
| C4 | Local quality v1 | **Sidecar** speaking *our* job protocol. We do not wrap a C++ port as the product. |
| C5 | In-process v1 | **Mock engine always.** Optional tiny preview behind a cargo feature. No 4B DiT in the default binary. |
| C6 | Horizon in-process quality | Independent Rust engine from papers + a layout **we** define. Tensor lib of record: **candle**. Named C exception: **ggml-FFI** for GGUF consumption. ONNX for small encoders. Burn deferred. |
| C7 | Nano | `cargo test --workspace` and `cargo build` succeed with default features on a 512 MB class box. Heavy engines are `--features`. |
| C8 | Spend | `TEXT2MESH_ALLOW_SPEND` (or product-prefixed twin) + `--allow-spend` + tool arg. Caps in config. Estimate tool is free. |
| C9 | Timeouts | Status/wait split. Client poll timeout is not job failure when `upstream_id` exists. Floor 30s on any wait default. |
| C10 | Hunyuan / CGAL | Never default-download Hunyuan community weights. Never default-link GPL Alpha Wrap. License flags fail closed. |

**(b)** In-process ggml quality in v1 (faster local path, heavier Nano, named C exception from day 0).

**(c)** Remote-only v1 (local is mock + “install sidecar later”). Weaker dual-path story; only acceptable as a first *slice*, not as the v1 product lock.

---

## 2. Architecture sketch

```
                    ┌─────────────────────────────────────────┐
                    │              Faces                       │
                    │  MCP  ·  CLI  ·  HTTP  ·  (WebUI)        │
                    └─────────────────┬───────────────────────┘
                                      │ MeshJob (one JSON schema)
                                      ▼
                    ┌─────────────────────────────────────────┐
                    │           Job director                   │
                    │  persist · watchdog · spend gate · SSE   │
                    └─────────────────┬───────────────────────┘
                                      │
                    ┌─────────────────┴───────────────────────┐
                    │              Planner                     │
                    │   mode = local | remote | auto           │
                    │   input: JobSpec + ProbeSnapshot         │
                    │   output: PlaneChoice | Degrade          │
                    └─────────────────┬───────────────────────┘
                                      │
              ┌───────────────────────┼───────────────────────┐
              ▼                       ▼                       ▼
     LocalPlane              RemotePlane               (future)
     - mock                  - Meshy adapter
     - sidecar (stdio/HTTP)  - Tripo adapter
     - preview feature       - OpenAPI-ish custom
     - (horizon: candle/ggml)- Colony LAN sibling
              │                       │
              └───────────┬───────────┘
                          ▼
                   Artifact store
                   GLB + sidecar JSON manifest
```

Standalone-first: zero siblings still work. Imaginarium (View-Contract T2I) and
Cadre (analytic CAD) are **optional providers** on the same plane trait or a
sibling compose URL, advertised in `system-check`.

---

## 3. `ComputePlane` trait (contract)

Names are illustrative; freeze field names in `docs/design.md` when the first
slice lands. Additive fields only after freeze.

```rust
#[async_trait]
pub trait ComputePlane: Send + Sync {
    fn id(&self) -> PlaneId;            // "local.mock" | "local.sidecar" | "remote.meshy" | …
    fn kind(&self) -> PlaneKind;        // Local | Remote
    fn caps(&self) -> PlaneCaps;

    /// Free. Never paid. Safe on Nano. Must not hang on missing keys.
    async fn probe(&self) -> ProbeReport;

    /// Free. Pure w.r.t. network if the catalog is local.
    /// Remote catalogs may refresh; cache + stale-ok.
    fn estimate(&self, spec: &JobSpec) -> Result<CostEstimate, PlaneError>;

    /// Spend gate already passed (or local $0). Persist job_id BEFORE this returns.
    async fn submit(&self, spec: JobSpec) -> Result<JobHandle, PlaneError>;

    /// Non-blocking. One poll of local engine or upstream.
    async fn poll(&self, id: &JobId) -> Result<JobSnapshot, PlaneError>;

    /// Blocking with timeout. Timeout ≠ upstream failure (see state machine).
    async fn wait(&self, id: &JobId, timeout: Duration) -> Result<JobSnapshot, PlaneError>;

    async fn cancel(&self, id: &JobId) -> Result<CancelOutcome, PlaneError>;

    /// Bytes live in the artifact store; this returns a handle, never multi-MB b64.
    async fn artifact(&self, id: &JobId, kind: ArtifactKind) -> Result<ArtifactRef, PlaneError>;
}
```

### 3.1 Caps (what a plane may claim)

```rust
pub struct PlaneCaps {
    pub image_to_mesh: bool,
    pub text_native: bool,          // hosted text-3D or native text weights
    pub view_contract: bool,        // this plane consumes N images (multiview)
    pub pbr: bool,
    pub preview_tier: bool,         // coarse / feedforward
    pub quality_tier: bool,         // 512³-class or hosted quality
    pub cascade_tier: bool,         // 1024³ / 1536³-class
    pub cpu_ok: bool,
    pub devices: Vec<DeviceKind>,   // probed, not compiled
    pub sync: bool,                 // true only for mock / tiny preview
    pub cancel: CancelSupport,      // Supported | BestEffort | Unsupported
    pub licenses: Vec<LicenseTag>,
    pub max_input_bytes: u64,
    pub estimated_vram_mb: Option<u64>,
    pub estimated_disk_mb: Option<u64>,
}
```

A plane that cannot do PBR must say so. A remote Meshy-class adapter may claim
`pbr=true` from the public API catalog, not from hope.

### 3.2 `JobSpec` (same for every plane)

```rust
pub struct JobSpec {
    pub route: Route,               // Image | ViewContract | Analytic | NativeText
    pub quality: QualityTier,       // Preview | Fine | Cascade | HostedDefault
    pub input: JobInput,            // image hash/path, contract id, prompt
    pub seed: Option<u64>,
    pub prefer_device: Option<DeviceKind>,
    pub budget: Budget,             // max_usd, max_seconds, max_vram_mb
    pub allow_spend: bool,
    pub idempotency_key: Option<String>,
}
```

The planner may **rewrite** `quality` downward (Fine → Preview) only when
`mode=auto` **and** the degrade is written into the snapshot (`degrade` field).
`mode=local` / `mode=remote` never silently change quality.

### 3.3 Errors (machine-readable, face-mapped)

```text
NotConfigured     missing key / missing sidecar binary / sibling URL unset
WeightsMissing    named weight id + expected path + bytes
FeatureOff        cargo feature not compiled
DeviceMissing     requested CUDA, probe says CPU
VramShort         need_mb vs have_mb
DiskShort         need_mb vs free_mb
LicenseBlocked    hunyuan / cgal / dinov3-unaccepted
SpendGated        gate closed or cap exceeded (before POST)
Unsupported       plane cannot do this route/tier
Rejected          spec invalid (n views, AR, …)
Upstream          HTTP 4xx/5xx after submit, with status + snippet
Timeout           OUR wait budget expired; see state (may still be running)
Cancelled
EngineCrash       local child died; job failed
Io                artifact store
Internal
```

MCP: these are `isError` **results** with `error_type`. JSON-RPC errors are
protocol breakage only.

HTTP: 4xx for `NotConfigured` / `SpendGated` / `Rejected`; 200 + `ok:false` is
forbidden for “I pretended to mesh.” Job poll may 200 with `status=failed`.

---

## 4. Planner: Local vs Remote vs Auto

### 4.1 Modes

| Mode | Behaviour |
|------|-----------|
| `local` | Run on a local plane or **fail**. Never fall through to a paid remote. |
| `remote` | Run on a remote plane or **fail**. Never silently use mock/local quality (surprise quality is a lie). |
| `auto` | Probe → pick. Never fake a pick. |

Config default: `auto`. CLI/MCP may override per job.

### 4.2 Auto decision (pure function — unit-test this)

Input: `JobSpec` + `ProbeSnapshot` (union of all plane probes) + `SpendPolicy`.

```
1. If route == Analytic:
     if Cadre provider live → Local(analytic) or Remote(cadre-lan)
     else Degrade(NotConfigured { sibling: "cadre" })

2. If route needs T2I views (ViewContract) AND images not yet on disk:
     plan a *sub-job* for T2I (Imaginarium or local T2I or hosted).
     If that sub-job is paid, the spend gate applies to the PARENT before fire.
     Mesh plane is chosen independently (local mesh + remote views is legal).

3. Candidate local engines, in order:
     a. sidecar with handshake ok AND weights/licenses for requested tier
     b. in-process preview feature if quality == Preview
     c. (horizon) in-process quality if feature on AND VRAM/disk/license ok
     Mock is NEVER selected by auto for a user-facing generate
     unless TEXT2MESH_ALLOW_MOCK=1 (demo/dev).

4. Local feasibility:
     weights present for tier
     AND license flags accept every required card (DINOv3, …)
     AND (CPU ok OR a probed GPU with vram_mb >= tier.floor)
     AND free_disk_mb >= weights + working set
     AND feature compiled
     AND sidecar binary alive if that engine is chosen

5. If local feasible AND (mode=local OR mode=auto) → Local(engine, device)

6. If mode=local and not feasible → Degrade with the *first failing reason*
     (WeightsMissing before VramShort before FeatureOff — stable order).

7. Remote feasibility:
     at least one adapter with key present (length>0)
     AND catalog says it supports the route
     AND spend gate open
     AND estimate.usd <= max_usd_per_job
     AND daily_spent + estimate <= max_usd_per_day
     LAN colony token counts as “key”; usd may be 0.

8. If remote feasible AND (mode=remote OR mode=auto) → Remote(provider)

9. Else Degrade:
     enumerate every plane's reason. Do not collapse to "timeout" or "error".
```

Stable reason order (for tests):
`FeatureOff` → `NotConfigured` → `WeightsMissing` → `LicenseBlocked` →
`DeviceMissing` → `VramShort` → `DiskShort` → `SpendGated` → `Unsupported`.

### 4.3 Device pick (local)

Capability query at probe time, cached ~5s, invalidated on `system-check --refresh`.

| Probe | How (illustrative) | Honesty |
|-------|--------------------|---------|
| CPU | always | `cpu_ok=true`, `slow=true` for quality tiers |
| CUDA | `nvidia-smi` or runtime device count + VRAM | Absent binary → `cuda: unavailable`, not “error” |
| ROCm/HIP | `rocminfo` / hip device count | |
| Vulkan | instance + physical devices | Integrated GPU VRAM may be shared; report `shared=true` |
| Metal | macOS metal device | |

User `prefer_device=cuda` + probe CPU-only → `DeviceMissing`, not a silent CPU
run. Auto may pick CPU for Preview if `cpu_ok` and the user did not pin a GPU.

**No compile-time product flavors** (`text2mesh-cuda` vs `text2mesh-cpu` as
separate products). Cargo features may omit a *backend crate*; runtime still
probes and reports “compiled out.”

### 4.4 Quality tiers (functional targets, our names)

Public image-to-3D cards describe a coarse occupancy preview, a fine voxel
decode, and optional higher cascades (see §11). We name **our** tiers:

| Our tier | Functional target | Typical local need |
|----------|-------------------|--------------------|
| `preview` | Coarse occupancy / feedforward mesh, fast, honest “draft” | ≤6 GB VRAM class or CPU |
| `fine` | Default quality textured GLB | Official quality cards: 16–24 GB class |
| `cascade` | Higher-res optional | ≥24 GB official; community offload [unofficial] |
| `hosted` | Provider default | $ / credits, no local VRAM |

Mesh counters are **not** an accuracy metric (public principle we may share).
Manifest records vertex/face counts as *size*, not *score*.

---

## 5. Job state machine (never stuck pending)

### 5.1 States

```text
                    persist job_id
                         │
                         ▼
                      queued
                         │  director picks plane / preflight
            ┌────────────┼────────────────────┐
            ▼            ▼                    ▼
     failed         submitted              running
   (preflight)     (remote only;            (local engine
                    upstream_id set)         or remote executing)
                         │                    │
                         └─────────┬──────────┘
                                   ▼
                              running
                     ┌─────────────┼──────────────┬──────────────┐
                     ▼             ▼              ▼              ▼
                succeeded      failed        cancelled    waiting_upstream
                (artifacts)   (reason)                    (remote + upstream_id
                                                           + poll window expired)
```

| State | Terminal? | Meaning |
|-------|-----------|---------|
| `queued` | no | Row exists; not yet handed to an engine. Watchdog bound (see §5.3). |
| `submitted` | no | Remote POST accepted; `upstream_id` known. |
| `running` | no | Engine or upstream is working. Heartbeat required. |
| `waiting_upstream` | **no** (recoverable) | **Our** wait/poll budget expired; paid/remote work may still be live. Must have `upstream_id` + resume recipe. |
| `succeeded` | yes | Artifacts on disk; `ok=true` only if GLB exists and parser-accepts. |
| `failed` | yes | Structured `error_type`. |
| `cancelled` | yes | User/agent cancel. |

**Banned:** a status string `pending` with no owner, no id, and no next action.
If a face must show “pending” in English, it maps from `queued|submitted|running`
and the JSON still uses the precise state.

### 5.2 Doctrine #3 vs #9 (resolved)

- **Local** work is ours. If the child dies, the GPU OOMs, or the process
  crashes: `failed` (`EngineCrash` / `Timeout` / `Io`). Nothing to resume at a
  vendor. Do **not** leave `running` across reboot.
- **Remote paid** work is theirs. If *our* poll window dies after `upstream_id`
  exists: `waiting_upstream` + `error_type=timeout` on the **wait call**, job
  row stays non-terminal. `job_status` later may still become `succeeded`.
  This is doctrine #9 (never orphan spend).
- If remote POST never happened (gate, key, network before id): `failed`.
  There is no vendor job to recover.
- Missing key / spend gate / missing weights: `failed` from `queued` in
  milliseconds. Never `waiting_upstream`.

### 5.3 Watchdog (jobs cannot rot)

SQLite (or equivalent) is truth. On director start and on a 15s tick:

| Condition | Action |
|-----------|--------|
| `queued` older than `queue_stale_secs` (default 60s) with no worker | `failed` `error_type=timeout` `message=queue watchdog` |
| `running` local, heartbeat older than `hb_secs` (default 30s) | `failed` `EngineCrash` |
| `running`/`submitted` remote, heartbeat stale, `upstream_id` present | `waiting_upstream`; next poll resumes |
| `waiting_upstream` older than `recover_ttl` (default 24h) | `failed` `Timeout` (spend already happened; record it) |
| Process boot, local `running` | `failed` (in-process/sidecar state is gone) |
| Process boot, remote `submitted`/`running`/`waiting_upstream` | resume poll |

Heartbeats are the engine’s progress events (stage name + pct). Silence is
failure for local, resume for remote.

### 5.4 Cancel

| Plane | Cancel |
|-------|--------|
| mock | immediate `cancelled` |
| sidecar | SIGTERM then SIGKILL after `cancel_grace` (≥30s); if the child ignores, `failed` `EngineCrash` after grace, not hung `running` |
| remote | call vendor cancel if catalog has it; else `CancelSupport::Unsupported` and state stays `running`/`waiting_upstream` with `cancel_requested=true` so UI is honest |

### 5.5 Nested jobs (View Contract)

A text route may spawn: `t2i_1 … t2i_N` then `mesh`. Parent is `running` until
children are terminal. If a paid T2I child is `waiting_upstream`, parent is
`running` (not a second pending hole). Parent fail-closes if any required view
fails the consistency gate after the retry budget.

---

## 6. Spend gates

Paid operations: hosted mesh APIs, hosted T2I for View Contract (Imaginarium /
xAI Imagine / other), GPU rental if we ever add it. **Local inference is $0**
but still has VRAM/disk/time gates.

### 6.1 Preflight (always free)

Tool/CLI: `estimate` / `text2mesh_estimate`.

```json
{
  "ok": true,
  "plane": "remote.tripo",
  "usd": 0.30,
  "usd_uncertain": false,
  "credits": 30,
  "credit_unit": "tripo",
  "seconds_p50": 40,
  "tier": "fine",
  "views": 6,
  "breakdown": [
    { "step": "t2i.imaginarium", "usd": 0.24, "n": 6 },
    { "step": "mesh.tripo", "usd": 0.30, "n": 1 }
  ],
  "caps": { "max_usd_per_job": 2.0, "max_usd_per_day": 10.0, "spent_today": 1.2 },
  "gate": "closed"
}
```

Catalog numbers (public, 2026-08 — **re-read at implement time**):

| Provider | Public ballpark | Notes |
|----------|-----------------|-------|
| Tripo API | 1 credit = **$0.01**. Image-to-3D 20 / 30 credits (no tex / standard tex). Text-to-3D 10 / 20. Multiview 20 / 30. HD tex add-on +10. | [developers.tripo3d.ai/en/pricing](https://developers.tripo3d.ai/en/pricing) |
| Meshy API | Credit-priced; Image-to-3D Meshy-6/7: 20 without texture, 30 with, 35 with 8K; +5 Ultra. Text-to-3D preview 20. | [docs.meshy.ai](https://docs.meshy.ai/en/api/pricing) — convert credits→USD from the live subscription catalog, do not hardcode a fake FX |
| Imaginarium / xAI Imagine | ~$0.02 / image, ~$0.04 image-2.0, quality ~$0.05–0.07 (garden catalog) | View Contract is N× this **before** mesh |
| Local / LAN colony | $0 | Still estimate **seconds** and VRAM |

`usd_uncertain=true` when we only have credits and no FX. Gate still applies if
`max_credits_per_job` is set; otherwise refuse auto-fire (`SpendGated` with
“cannot convert credits to USD”).

### 6.2 Gate rules

1. Default **closed**. Open with env `TEXT2MESH_ALLOW_SPEND=1` **or** CLI
   `--allow-spend` **or** tool arg `allow_spend: true`. All three are equivalent
   tokens; MCP must pass the arg (agents should not inherit a process-wide open
   gate by accident — prefer per-call).
2. Local $0 mesh **does not** need the gate. Paid **sub-jobs** (T2I) still do.
3. Estimate is checked **before** the upstream POST (Imaginarium pattern:
   `[limits] max_usd_per_job`, `max_usd_per_day`; 0/omit = off).
4. Daily sum includes `queued`+`submitted`+`running`+`waiting_upstream`+`succeeded`
   estimated USD. Failed preflight does not count. Failed *after* POST counts
   (money may be gone).
5. Tests never hit live paid APIs (doctrine #8). `#[cfg(test)]` planes are mocks.
6. Secrets: keys in 0600 env files; logs print **length + head only**.
7. INSTALLED ≠ ACTIVE: a compiled Meshy adapter with no key is inert and says so.

### 6.3 Rate limits

Optional process-local token bucket for paid calls (Imaginarium:
`paid_rpm` / `paid_burst`). Polls, estimate, system-check, mock are free.
HTTP 429 + `error_type=rate_limit` + `Retry-After`.

---

## 7. `system-check` honesty

Always free. Never paid. Safe with no keys, no GPU, no weights.

CLI: `text2mesh system-check [--json] [--refresh]`
MCP: `text2mesh_system_check`
HTTP: `GET /v1/system-check`

### 7.1 Report shape (sketch)

```json
{
  "ok": true,
  "product": "text2mesh",
  "version": "0.0.0",
  "features": {
    "compiled": ["remote-http", "sidecar"],
    "not_compiled": ["preview-onnx", "quality-candle", "quality-ggml"]
  },
  "devices": [
    { "kind": "cpu", "ok": true, "slow": true },
    { "kind": "cuda", "ok": false, "reason": "nvidia-smi not found" },
    { "kind": "vulkan", "ok": true, "name": "AMD Radeon", "vram_mb": 16384, "shared": false }
  ],
  "weights": [
    {
      "id": "preview.triposr",
      "present": false,
      "want_bytes": 1803550720,
      "path": "~/.local/share/text2mesh/weights/preview.triposr",
      "license": "MIT"
    },
    {
      "id": "encoder.dinov3_vitl16",
      "present": true,
      "have_bytes": 636000000,
      "sha256_head": "dcb2e451…",
      "license": "DINOv3",
      "accepted": false
    }
  ],
  "licenses": {
    "dinov3_accepted": false,
    "hunyuan_community_accepted": false,
    "cgal_gpl_accepted": false
  },
  "keys": [
    { "id": "MESHY_API_KEY", "present": false },
    { "id": "TRIPO_API_KEY", "present": true, "len": 48, "head": "tsk_" }
  ],
  "sidecars": [
    { "id": "local.sidecar", "path": "/usr/local/bin/mesh-engine", "ok": false, "reason": "binary not found" }
  ],
  "siblings": [
    { "id": "imaginarium", "url": null, "ok": false, "reason": "not configured" },
    { "id": "cadre", "url": null, "ok": false, "reason": "not configured" }
  ],
  "planner": {
    "mode": "auto",
    "would_pick": null,
    "degrade": {
      "error_type": "WeightsMissing",
      "message": "no local quality weights; no remote key with spend gate open"
    }
  },
  "spend": { "gate": "closed", "spent_today_usd": 0.0 }
}
```

### 7.2 Honesty table

| Condition | Must report | Must not |
|-----------|-------------|----------|
| CUDA crate compiled, no GPU | `cuda.ok=false` | “GPU ready” |
| Weights dir empty | each id `present:false` + bytes | “models installed” because the folder exists |
| DINOv3 file on disk, license flag off | `present:true`, `accepted:false`, planner degrades `LicenseBlocked` | auto-run the encoder |
| Key length 0 | `present:false` | timeout later |
| Sidecar path set, handshake fail | `ok:false` + stderr tail (no secrets) | treat as local-ready |
| Feature off | listed under `not_compiled` | register a tool that always 500s |
| Mock allowed | `planner.would_pick` mentions mock only if allow-mock | auto-pick mock for users |
| CPU-only quality | `slow:true`, estimated seconds in the minutes–hours class | imply interactive |

Exit codes: `0` if the binary can *speak* (report produced). Non-zero only on
probe crash. Readiness for generate is `planner.would_pick != null`, not exit 0.
Agents must inspect JSON.

---

## 8. Nano-first and cargo features

Garden rule (Launchpad `docs/stack.md`): build defaults for the smallest tier;
Occipital pattern — heavy runtimes are **opt-in features**, not just env
toggles, so the default binary does not link ONNX/CUDA/ggml.

### 8.1 Feature matrix

| Feature | Default | Links | What it enables |
|---------|---------|-------|-----------------|
| (none extra) | yes | tokio, reqwest, rusqlite, serde | Job director, mock engine, planner, system-check, remote HTTP **client** |
| `remote-http` | **in default** | reqwest rustls | Meshy/Tripo/custom adapters (inert without keys) |
| `sidecar` | off | std process | Local quality via user binary |
| `preview-onnx` | off | `ort` / ONNX Runtime | Small feedforward preview if a MIT ONNX exists |
| `preview-candle` | off | candle-core (CPU) | Tiny encoder / preview in Rust |
| `quality-candle` | off | candle + optional cuda/metal | Horizon in-process quality |
| `quality-ggml` | off | ggml FFI (named CHARTER exception) | Horizon GGUF quality |
| `cuda` / `metal` / `vulkan` | off | backend crates | Device kernels; still probed at runtime |

Default `cargo build` on a Pi-class Nano: **no** ORT, **no** ggml, **no** CUDA
toolkit, **no** 14 GB download. Remote generate still works if a key exists and
the spend gate is open (timeouts ≥ 30s).

### 8.2 Timeouts

| Call | Default | Floor |
|------|---------|-------|
| `probe` / `system-check` | 5s per device probe, 20s total | don't spin on nvidia-smi hang; kill + `unavailable` |
| `estimate` | local catalog 0ms; remote refresh 10s stale-ok | |
| `job_status` | 30s HTTP | 30s |
| `job_wait` | 600s (mesh is slow) | 30s |
| sidecar handshake | 30s | 30s |
| sidecar generate | no implicit short timeout; heartbeat instead | |

MCP: tools/list and ping stay live while `job_wait` runs (status/wait split,
Imaginarium lesson).

### 8.3 Weight fetch

Never auto-pull multi-GB weights on first `generate`. Command:
`text2mesh weights pull <id> --accept-license <tag>`. Disk gate: refuse if
`free_mb < want_mb * 1.1`. Progress on stderr.

---

## 9. Sidecar vs in-process

### 9.1 Decision

| Path | v1 | Why |
|------|----|-----|
| In-process **mock** | **yes** | CI, Nano, demo. Deterministic GLB. |
| In-process **preview** | optional feature | Fast draft; only if MIT weights + a runtime we already feature-gate. |
| In-process **quality DiT** | **horizon** | Months of work; Nano cannot compile it by accident. |
| **Sidecar** quality | **v1 local quality** | Process isolation (OOM kills the child, not MCP). User may bring any engine that speaks our protocol. We are not a wrapper product. |
| Shell-out to Python | **no** | Python is test dumps only (briefing §9). |

This is briefing OQ-2 option **(c)**: sidecar v1 + independent Rust engine as
horizon. PRD should lock (c) unless CHARTER picks (b).

### 9.2 Sidecar protocol (`meshplane/1`) — ours, not a C ABI clone

Transport (pick one in design.md; both may exist):

1. **stdio NDJSON** (preferred for a local binary): one JSON object per line on
   stdout; logs on stderr. Mirrors MCP stdout-sacred.
2. **loopback HTTP** `127.0.0.1:<port>` with bearer from a temp file 0600.

Handshake (first message):

```json
{
  "protocol": "meshplane/1",
  "engine": "user-engine-name",
  "version": "1.2.3",
  "caps": { "image_to_mesh": true, "pbr": true, "tiers": ["preview", "fine"] },
  "licenses": ["MIT", "DINOv3"],
  "devices": ["cpu", "vulkan"]
}
```

Job messages: `submit` (full `JobSpec`), `progress` (stage, pct, message),
`artifact` (path inside a scratch dir we created), `fail` (error_type), `pong`.
We create the scratch dir; the child cannot pick arbitrary filesystem paths
(confinement: no `..`, canonical, under scratch).

Rules:

- Protocol version mismatch → `Unsupported`, not a hang.
- Child exit ≠ 0 → `EngineCrash`; job `failed`.
- No handshake in 30s → `NotConfigured`.
- We never import their types, `.t2mesh` / `.dinodata` names, or `t2_*` C ABI.
  If a user wraps a community binary, **their** adapter translates to
  `meshplane/1`.
- GPL features inside the child are the user's license problem; we refuse to
  *bundle* GPL. `system-check` can warn if the handshake advertises `cgal`.

### 9.3 In-process mock

- Feature: always compiled (tiny).
- Input: image bytes or prompt string.
- Output: a **valid GLB** (triangle, vertex color) whose contents are a
  deterministic function of `sha256(input) || seed`. Same spec → same hash.
- Completes in <50ms. State machine still goes `queued → running → succeeded`.
- Manifest `engine=mock`, `quality=preview`, `ok=true` with
  `disclaimer=not-a-model`.
- Auto planner will not select it unless `TEXT2MESH_ALLOW_MOCK=1`.

---

## 10. Inference runtimes (candle / burn / ggml-FFI / ONNX)

Nano **must** build with none of them. This section is for optional features
and the horizon engine.

### 10.1 Comparison (public, 2026)

| | Candle | Burn | ggml via FFI | ONNX Runtime (`ort`) |
|---|--------|------|--------------|----------------------|
| Home | Hugging Face, MIT/Apache | Tracel, MIT/Apache | ggml-org C/C++, MIT | Microsoft ORT + pyke `ort` crate |
| Rust purity | High (CUDA kernels are C++) | High (CubeCL) | **C FFI** — named exception | C++ runtime behind bindings |
| Backends | CPU (MKL/Accelerate), CUDA, Metal, WASM; optional ONNX eval | CUDA, ROCm, Metal, **Vulkan**, WebGPU, CPU, `no_std` | CPU, CUDA, HIP/ROCm, **Vulkan**, Metal, SYCL, OpenCL | CPU, CUDA EP, DirectML, CoreML, … vendor EPs |
| GGUF | quantized LLM loaders; not a 3D DiT zoo | weights via burn-store / safetensors | **Native GGUF** — public TRELLIS.2-class GGUFs exist | No; needs ONNX export |
| Sparse 3D DiT | We would write the graph | We would write the graph | We would still write **our** graph (clean-room: do not copy a community graph) | Sparse voxel DiT export is unlikely as a public ONNX |
| Garden fit | HF safetensors/tokenizers already in ecosystem | wgpu/Vulkan story matches house GPU stack | Matches public f16 GGUF cards (~14–16 GB class) | Already used via `fastembed` (opt-in) |
| Nano | CPU feature possible; CUDA feature needs toolkit | wgpu/Vulkan may pull shader compile | CMake/cc in the build; easy to bloat default | ORT download/link; Occipital already learned “opt-in or the binary grows” |
| Training | secondary | first-class | no | no |

### 10.2 Recommendation — **(a)** hybrid (lock this)

1. **v1 default:** no tensor runtime.
2. **v1 local quality:** sidecar (`meshplane/1`).
3. **v1 optional preview:** `preview-onnx` **or** `preview-candle` (pick at Sx
   when a MIT feedforward weight is actually wired). Prefer **candle CPU** if we
   reimplement a tiny model; prefer **ONNX** if a public MIT `.onnx` already
   exists (TripoSR-class is a 1.68 GB PyTorch ckpt today, **not** an ONNX — do
   not pretend it drops in).
4. **Horizon in-process quality tensor lib of record: candle.** Pure-Rust
   preference, safetensors, CUDA/Metal, serverless-sized binaries. We implement
   stages from the **papers** (arXiv:2412.01506, 2512.14692) + a GGUF/safetensors
   layout **we** document. Do not ingest a community ggml graph.
5. **Named CHARTER C exception: `quality-ggml`.** Use only if we need ggml
   kernels / GGUF mmap that candle does not have yet. Isolated crate. Default
   off. License MIT of ggml itself; DINOv3 card still needs its own accept flag.
6. **ONNX:** small encoders and embeddings, garden-proven. Not the 4B quality
   path.
7. **Burn: not v1.** Strongest Vulkan/CubeCL story; revisit if candle’s GPU
   story blocks AMD/Intel and sidecar is insufficient. Do not carry two tensor
   cores in v1.

**(b)** ggml-FFI as the only quality backend (faster GGUF day-one, worse Nano
and house “pure Rust” taste).

**(c)** Burn-only (best vendor-neutral GPU, weaker GGUF/HF examples, training
weighted when we are inference-only).

### 10.3 Device vs runtime

`system-check` devices are **independent** of which tensor crate is compiled.
A sidecar may use Vulkan while the Rust binary has no wgpu. Report both:
`runtime_compiled` and `sidecar.devices`.

---

## 11. VRAM / disk ballparks (public cards only)

Numbers age. Re-read cards before locking defaults. Official floors beat
anecdote; anecdote is labelled **[unofficial]**.

### 11.1 Disk (weights)

**TRELLIS.2-4B** (`microsoft/TRELLIS.2-4B`, MIT, image-to-3D, 4B params):

Public `ckpts/` safetensors (HF file sizes, 2026-08):

| File (public name) | Size |
|--------------------|------|
| `ss_flow_img_dit_1_3B_64_bf16.safetensors` | 2.58 GB |
| `slat_flow_img2shape_dit_1_3B_512_bf16.safetensors` | 2.58 GB |
| `slat_flow_img2shape_dit_1_3B_1024_bf16.safetensors` | 2.58 GB |
| `slat_flow_imgshape2tex_dit_1_3B_512_bf16.safetensors` | 2.58 GB |
| `slat_flow_imgshape2tex_dit_1_3B_1024_bf16.safetensors` | 2.58 GB |
| `shape_dec_next_dc_f16c32_fp16.safetensors` | 948 MB |
| `shape_enc_next_dc_f16c32_fp16.safetensors` | 709 MB |
| `tex_dec_next_dc_f16c32_fp16.safetensors` | 948 MB |
| `tex_enc_next_dc_f16c32_fp16.safetensors` | 709 MB |

≈ **16.2 GB** for those stages alone.

Public f16 GGUF companions (LocalAI-io cards; MIT except DINOv3):

| Card | Role | Public size |
|------|------|-------------|
| `TRELLIS.2-4B-GGUF` | 8 GGUF files (flows + decoders) | ~0.95 + 0.71 + 5×2.62 + 0.95 GB ≈ **15.7 GB** |
| `TRELLIS-image-large-GGUF` | sparse-structure decoder | **147 MB** |
| `dinov3-vitl16-pretrain-lvd1689m-GGUF` | ViT-L/16 encoder | **607 MB** GGUF; source safetensors **1,212,559,808 B** (~1.13 GiB) |

Community README figure for “download the GGUFs” ≈ **14 GB**. Use **14–17 GB**
as the quality-stack disk ballpark; `system-check` should sum **actual** file
sizes.

**DINOv3 license** is **not** MIT. Planner must require an accept flag.
Redistribution must follow Meta’s agreement (public card).

**TRELLIS v1** (MIT): `TRELLIS-image-large` 1.2B; text-base **342M**, text-large
**1.1B**, text-xlarge **2.0B**. Authors publicly recommend T2I → image-3D
rather than native text-3D. Hardware note: NVIDIA **≥16 GB** (README).

**TripoSR** (`stabilityai/TripoSR`, MIT): `model.ckpt` **1.68 GB**. Default
options **~6 GB VRAM**. **~0.5 s** on A100. CPU allowed (slow). Good *preview*
class, not the quality ceiling.

**Hunyuan3D 2.1** (community license — **do not default**): public space README
states **10 GB** VRAM shape, **21 GB** texture, **29 GB** both. Geo/MAU limits
on the community license. Hosted 3.1 is ToS + geo. Planner: `LicenseBlocked`
unless a user flag we do not ship on.

### 11.2 VRAM (runtime)

| Engine class | Official | Field [unofficial] |
|--------------|----------|---------------------|
| TRELLIS.2-4B Python card | **≥24 GB** NVIDIA; verified A100/H100 | 16 GB attempts; offload claims down to 8 GB / even 6 GB in community UIs — **do not advertise** |
| TRELLIS v1 Python demo | **≥16 GB** NVIDIA; verified A100/A6000 | 12 GB reports exist |
| ggml-class public demo note (community README) | 512³ on 16 GB class ~minutes; 1024³ ~10 GB VRAM + host RAM spike | CPU works and is slow |
| TripoSR | ~**6 GB** | CPU ok |
| Hunyuan 2.1 | 10 / 21 / 29 GB | skip as default |
| Hosted APIs | 0 local VRAM | network + credits |
| Mock | ~0 | CI |

H100 public timings for TRELLIS.2 (card): 512³ ~3 s, 1024³ ~17 s, 1536³ ~60 s.
Consumer GPU is **not** that. Estimate seconds from device class in the catalog
we maintain, not from H100 rows.

### 11.3 Working set / scratch

Beyond weights: scratch for views (View Contract N× ~2–8 MB PNG), GLB out
(tens of MB), possible atlas. Disk gate: `weights + 2 GiB` free as a v1 floor
unless probe says otherwise. Host RAM spike on high-res decode is real —
`system-check` should report `ram_mb` and `slow` on low-RAM Nano rather than
OOM looping.

### 11.4 Planner floors (v1 defaults, amend with field truth)

| Pick | Disk present | VRAM floor | Else |
|------|--------------|------------|------|
| local preview | ≥2 GB preview weights | 6 GB GPU **or** CPU | remote or degrade |
| local fine | ≥12 GB quality stack | 16 GB (warn), 24 GB (official comfortable) | remote or degrade |
| local cascade | full stack incl. 1024 models | 24 GB official | degrade to fine or remote |
| remote | n/a | n/a | keys + spend gate |

CPU quality: allowed, `slow=true`, estimate in **hours** unless a field
measurement says otherwise. Never label it interactive.

---

## 12. Mockability for CI (dual-path gate)

Briefing success seed: the same `MeshJob` JSON round-trips a **local mock
engine** and an **HTTP mock provider**.

### 12.1 Must-have tests (pure + fixture)

| Test | Proves |
|------|--------|
| `planner_auto_picks_local_when_weights_and_vram` | Auto happy path |
| `planner_auto_falls_to_remote_when_weights_missing_and_key` | Dual path |
| `planner_auto_degrades_when_no_weights_no_key` | Stated degrade |
| `planner_local_mode_never_calls_remote` | Spend safety |
| `planner_remote_mode_never_uses_mock` | Quality honesty |
| `missing_key_is_not_configured_not_timeout` | Distinct errors |
| `spend_gate_blocks_before_post` | Doctrine #8 |
| `wait_timeout_remote_goes_waiting_upstream` | Doctrine #9 |
| `wait_timeout_local_goes_failed` | Doctrine #3 |
| `watchdog_queued_flips_failed` | Never stuck |
| `job_json_roundtrip_local_mock` | Schema |
| `job_json_roundtrip_http_mock` | Schema |
| `system_check_cpu_only_fixture` | Honesty |
| `feature_off_caps_hide_quality` | Nano |

Network: `wiremock` or an in-process axum stub. Captured JSON fixtures for
Meshy/Tripo **when** those adapters ship — parsers tested against truth.

Live tests: `#[ignore]` or env `TEXT2MESH_LIVE=1`, skip **loudly**
(`eprintln!("skip live: no key")`), never silent skip.

### 12.2 HTTP mock provider

Implements the **same** `/v1/jobs` surface as our API (so Colony LAN and CI
share a contract):

```text
POST /v1/jobs          → { job_id, status: "queued" }
GET  /v1/jobs/{id}     → snapshot
GET  /v1/jobs/{id}/artifact?kind=glb
POST /v1/jobs/{id}/cancel
GET  /v1/system-check
```

Stub sequence: `queued → running → succeeded` with a tiny committed GLB.
A second stub sequence: `queued → submitted → waiting_upstream → succeeded`
to test resume. A third: 401 missing-key **is not used** (we fail before POST);
use 402/403 only for “key present but vendor rejects.”

### 12.3 Golden GLB

Mock writes a GLB that `gltf` crate parses: 1 mesh, vertex colors, no
external bin. Hash pinned in the test. Faces must not inline the GLB in MCP
results (content URL / path only).

---

## 13. Manifest (provenance, per job)

Sidecar JSON next to the GLB (name we own, e.g. `manifest.json` in the job
dir). Record:

- `job_id`, `upstream_id`
- route, quality, seed
- input image hash(es), View Contract id
- plane id, engine id, device, runtime feature
- timings per stage
- licenses of every weight touched
- spend: estimated + actual if vendor returns it
- degrade list (empty on clean run)
- `ok`, `error_type`
- artifact hashes

This is how `system-check` and a later audit agree. No secrets.

---

## 14. Faces (thin)

| Face | Compute-plane duties |
|------|----------------------|
| core | trait, planner, watchdog, mocks |
| CLI | `generate`, `status`, `wait`, `estimate`, `system-check`, `weights pull` |
| MCP | same; `status`/`wait` split; stdout sacred |
| HTTP | job resource + SSE optional; loopback default; LAN token if bind ≠ loopback |

One schema. No face-specific job states.

---

## 15. Open questions this note does **not** silently close

These remain CHARTER OQs / PRD:

1. Product/crate name (Figment / Tessera / Loom / …). Protocol id `meshplane/1`
   is a placeholder.
2. Whether v1 local quality is sidecar-only **(a/c)** vs ggml-in-process **(b)** —
   this note recommends **(a/c)**.
3. Preview runtime: candle vs ONNX — wait until a real MIT weight is wired.
4. Gaussian / NeRF outputs: mesh-only v1 recommended (hosted APIs and local
   quality cards all speak GLB; extra formats are post-v1).
5. View Contract camera count (4/6/8) — spend scales linearly with T2I.
6. HTTP bind port (avoid 8791 / 8795 / 7411).
7. Watertight print: defer GPL wrap; do not feature-gate it on by default.

---

## 16. Sources (public)

- Launchpad-RS `docs/house-doctrine.md` (§3 honesty, §8 spend, §9 orphan spend), `docs/stack.md` (Nano-first, four-face)
- OmniOcular-RS CHARTER D4–D9 + `docs/design.md` provider/spend/system-check
- Imaginarium-RS `docs/ARCHITECTURE.md` (ULID jobs, estimate, caps, status/wait, timeout stays recoverable)
- Cadre-RS `docs/KERNEL_HONESTY.md` (mock vs real kernel, refuse silent stand-ins)
- TRELLIS v1 README + paper arXiv:2412.01506 — ≥16 GB, text vs image advice, model sizes
- TRELLIS.2-4B card + paper arXiv:2512.14692 — ≥24 GB, H100 timings, 4B, 512³–1536³
- HF `microsoft/TRELLIS.2-4B/tree/main/ckpts` file sizes
- LocalAI-io GGUF cards (sizes + DINOv3 license) — **cards only**
- Community C++/ggml README capability lines (backends, ~14 GB GGUF pull, 64³/512³/1024³/1536³) — **not source**
- TripoSR README + HF `model.ckpt` 1.68 GB, ~6 GB VRAM, 0.5 s A100, MIT
- Hunyuan3D-2.1 public space README VRAM 10/21/29 GB + community-license landmine
- Tripo API pricing; Meshy API pricing (credits)
- Candle README (backends, goals); Burn README (CubeCL backends); ggml/llama.cpp backend list

**Forbidden to implementers:** opening trellis2.cpp / TRELLIS Python / Hunyuan
trees for “how they did the graph.” Papers + this contract + glTF 2.0 + GGUF
spec + crates.io.

---

## 17. PRD lift checklist

Copy into `docs/design.md` / CHARTER as slices land:

- [ ] `ComputePlane` trait + `PlaneError` enum frozen
- [ ] Planner pure fn + the unit table in §12.1
- [ ] Job states including `waiting_upstream` (no orphan `pending`)
- [ ] Spend gate + estimate tool
- [ ] `system-check` JSON
- [ ] Cargo features: default Nano-clean
- [ ] Mock local + HTTP mock in CI
- [ ] Sidecar `meshplane/1` handshake
- [ ] License flags: DINOv3 / Hunyuan / CGAL fail closed
- [ ] Device probe, not product `#ifdef`s
