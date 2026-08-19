# v1 status — S0–S12 shipped

| Field | Value |
|---|---|
| **Date** | 2026-08-19 |
| **Repo** | [`buckster123/Tessera-RS`](https://github.com/buckster123/Tessera-RS) `main` |
| **Close-out** | after PR #6 (`5400a18`) |
| **Charter** | D1–D30 unchanged; phases S0–S12 **implemented** |

This is the close-out note, not a second charter. Binding decisions stay in [`CHARTER.md`](CHARTER.md). Wire stays in [`design.md`](design.md). How to run the product: [`USER.md`](USER.md).

## What “v1 shipped” means

The CHARTER slices S0–S12 are on `main`, CI green, and the Krackan field probe matches the contract. It does **not** mean this box can run a 24 GB quality DiT. v1 quality is a **user sidecar** (`meshplane/1`) or a **paid remote** (Meshy / Tripo / colony). Mock and vertex-colour paths stay **`degraded`**.

| Slice | Evidence |
|---|---|
| S0–S4 | `main` bootstrap: mock GLB, planner 12-row table, honesty surfaces |
| S5–S6 | PR #1 — View Contract goldens + mock Hero-Orbit / G3–G4 |
| S7 | PR #2 — Lattice; Cadre refuse-if-absent |
| S8 Imaginarium T2I | PR #3 — estimate-then-fire; no `XAI_API_KEY` here; live skip-loud unless `TEXT2MESH_LIVE=1` |
| S9 sidecar | PR #3 — `meshplane-fixture` e2e; confinement + cancel |
| S10 Meshy / Tripo | PR #4 — 402/429 mapped; live ignored unless `TEXT2MESH_LIVE=1` |
| S11 export + studio | PR #5 — `gltf` crate honesty; loopback HTMX groutbench; operator browser-checked 2026-08-19 |
| S12 weights + idle | PR #6 — live Krackan `system-check`: `vram_mb=512`, `shared=true`, `name=AMD Radeon 840M` |

## Field truth (Krackan, 2026-08-19)

AMD Radeon 840M, **512 MiB** dedicated carve-out, **shared** iGPU, ~22 GiB host RAM, no NVIDIA. Probe uses sysfs `mem_info_vram_total`. It does **not** treat GTT, rocminfo pools, vulkan host heaps, or `/proc/meminfo` as VRAM.

Live `text2mesh system-check --json` (no quality weights, no remote key):

- `gpu.vulkan` / `amd.rocm`: `vram_mb=512`, `shared=true`, `slow=true`
- `planner.would_pick=null`, `degrade.error_type=weights_missing`
- `tier=nano`
- Hunyuan `blocked_by_default` (`territory_eu_uk_kr`, `mau_cap`, `no_train_on_outputs`, `hk_law`)

With a Tripo/Meshy key and spend open, auto picks **remote**, never local standard on this GPU.

## Still open (not v1 blockers)

- Cadre compose — analytic prompts still `analytic.unavailable` until Cadre is live (D8)
- CLIP G0–G2 — `feature_off` unless `TEXT2MESH_ALLOW_UNGATED=1` (G3/G4 only)
- Crates.io + trademark sweep — garden name **Tessera-RS**, crates stay `text2mesh` (D1)
- Horizon in-process quality DiT — **not scheduled** (`quality-candle` / `quality-ggml` stay out of `Cargo.toml`)
- Native/Slint viewer, print wrap, Gaussian/NeRF-as-success — CHARTER “out of v1”

## PRs

| # | Slice |
|---|---|
| [#1](https://github.com/buckster123/Tessera-RS/pull/1) | S5 View Contract + S6 Hero-Orbit |
| [#2](https://github.com/buckster123/Tessera-RS/pull/2) | S7 Lattice |
| [#3](https://github.com/buckster123/Tessera-RS/pull/3) | S8 Imaginarium + S9 sidecar |
| [#4](https://github.com/buckster123/Tessera-RS/pull/4) | S10 Meshy + Tripo |
| [#5](https://github.com/buckster123/Tessera-RS/pull/5) | S11 export + groutbench |
| [#6](https://github.com/buckster123/Tessera-RS/pull/6) | S12 weights, licenses, idle unload, Krackan probe |
