<div align="center">

<img src="assets/banner.jpg" alt="Tessera-RS / text2mesh" width="100%">

<h1>Tessera-RS <em>(text2mesh)</em></h1>

<p><strong>Image or text in. Honest glTF 2.0 GLB out.</strong><br>
Local sidecar <em>or</em> a networked provider. One <code>MeshJob</code>. Three faces. No fake success.</p>

<p>
<img alt="license" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue">
<img alt="rust" src="https://img.shields.io/badge/rust-2021-orange?logo=rust&logoColor=white">
<img alt="ci" src="https://img.shields.io/github/actions/workflow/status/buckster123/Tessera-RS/ci.yml?label=ci">
<img alt="status" src="https://img.shields.io/badge/status-v0.1%20·%20shipped-brightgreen">
</p>

</div>

---

> [!NOTE]
> **v1 (S0–S12) is on `main`.** Quality meshes come from a user sidecar or a paid remote. The in-process mock is a **valid** GLB that terminates **`degraded`** (vertex colour, `not-a-model`) — never a green tick. Shared iGPUs (this project's field box is a 512 MiB AMD 840M) are counted as device VRAM, not host RAM.

## Why

Most “text-to-3D” tools fire N independent T2I samples at an image-to-mesh model and hope the views agree. **Tessera-RS** treats the text path as a compiler:

1. **Lattice Router** — analytic CAD vs visual vs native text-3D
2. **View Contract** — typed subject lock, camera ring, lighting, negatives, seeds
3. **Hero-Orbit** — one hero T2I, then I2I orbit (not N independent samples)
4. **Gates G0–G4** — fail-closed before the 3D plane is called
5. **The same image-to-mesh plane** the still-image path uses

Dual compute is one planner over probes: local sidecar / preview, or Meshy / Tripo / colony. `auto` is not a third engine. Paid work never auto-fires.

Garden name **Tessera-RS**. Working crate and binary prefix stay `text2mesh` until a crates.io rename sweep.

## Surfaces

| Surface | For | How |
|---|---|---|
| **CLI** | you & scripts | `text2mesh generate` / `estimate` / `system-check` |
| **MCP** | agents | stdio `text2mesh-mcp`, protocol `2024-11-05` |
| **HTTP + studio** | loopback humans | `text2mesh serve` → `http://127.0.0.1:8796/` |

One job schema (`text2mesh.job.v1`) on all three. Off-loopback HTTP needs a bearer; the studio 404s there on purpose.

## Honesty

This is the product, not a disclaimer.

| Terminal | Means | CLI |
|---|---|---|
| `succeeded` | parser-accepted GLB **and** the materials we claimed (PBR) | exit **0** |
| `degraded` | valid GLB, but mock / vertex-colour / stated step-down | exit **1** |
| `failed` | no honest mesh (grey default, missing key, license, crash, …) | 3–9 |
| `waiting_upstream` | paid remote still has an `upstream_id`; we do not fake-fail it | wait / poll |

`system-check` uses `report_complete` + `ready` — there is no `ok` for readiness. Manifest `ok=true` only when `status=succeeded`. Face-wrapper `ok` means “this call parsed,” never “meshed.”

## Quick start

```sh
git clone https://github.com/buckster123/Tessera-RS
cd Tessera-RS
cargo build --release --workspace

# free honesty probe — VRAM is the GPU carve-out, not your 22 GiB of RAM
./target/release/text2mesh system-check --json

# mock path (CI / Nano / this laptop). status=degraded, exit 1 — that is correct
TEXT2MESH_ALLOW_MOCK=1 ./target/release/text2mesh generate \
  --image crates/text2mesh/tests/fixtures/dot.png \
  --compute local --provider local.mock --json

# loopback studio (amber degrade banner, hex camera ring)
TEXT2MESH_ALLOW_MOCK=1 ./target/release/text2mesh serve
# → http://127.0.0.1:8796/
```

Need a real textured mesh? Point `TEXT2MESH_SIDECAR` at a `meshplane/1` engine, **or** set `TRIPO_API_KEY` / `MESHY_API_KEY` and `--allow-spend`. Estimate first — it is free.

Operator detail: [`docs/USER.md`](docs/USER.md).

## What runs where

```
  still image ──────────────────────────────┐
  text ─► Lattice ─► View Contract          │
                 │        │                 │
                 │        ▼                 ▼
                 │   Hero-Orbit T2I    Image3dPlane
                 │   (Imaginarium)     · local.mock      (always degraded)
                 ▼                     · local.sidecar   (your meshplane/1)
            Cadre or refuse            · remote.tripo
            (analytic CAD)             · remote.meshy
                                       · remote.colony
                                              │
                                              ▼
                                       glTF 2.0 GLB + manifest
```

On a Nano or a shared iGPU under 6 GiB, `auto` picks **preview, remote, or degrade** — never silent `standard`. Krackan field probe (2026-08-19): `vram_mb=512`, `shared=true`, `AMD Radeon 840M`.

v1 does **not** ship an in-process 4B DiT. That is horizon. A sidecar binary on disk is not a 24 GB GPU.

## Weights & licenses

```sh
text2mesh weights pull encoder.dinov3_vitl16 --accept-license dinov3
text2mesh weights pull quality.stack --accept-license mit
```

CLI only. Never auto-pulled on generate. Disk must have `want * 1.1` free. Hunyuan community weights are **blocked by default** (EU/UK/KR territory, MAU cap, no-train). DINOv3 on disk without accept → `present:true`, `accepted:false`. This process never reads `XAI_API_KEY`.

## MCP

```json
{
  "mcpServers": {
    "text2mesh": {
      "command": "/absolute/path/to/text2mesh-mcp"
    }
  }
}
```

`text2mesh_wait` default timeout is **1800 s** (same as the CLI). `text2mesh_artifact` returns a **path**, not a blob. There is no weights-pull tool on MCP.

## What this is not

- Not a CAD kernel (that's Cadre-RS) and not a T2I keyholder (that's Imaginarium-RS)
- Not a port of TRELLIS, Hunyuan3D, TripoSR, or Meshy source
- Not a wrapper whose identity is someone else's binary
- Not multi-tenant SaaS, not telemetry, not Python at runtime
- Not animation, rigging, or “Gaussian/NeRF as success”

## Docs

| File | What's in it |
|---|---|
| [`docs/USER.md`](docs/USER.md) | Runbook — CLI, studio, remotes, sidecar, env, exit codes |
| [`docs/STATUS.md`](docs/STATUS.md) | v1 close-out and field truth |
| [`docs/CHARTER.md`](docs/CHARTER.md) | Binding D1–D30 |
| [`docs/design.md`](docs/design.md) | Wire / job / HTTP / MCP contract |
| [`docs/prd.md`](docs/prd.md) | Product requirements |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Clean-room + PR rules |

## License

MIT OR Apache-2.0 — [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

<sub>Banner: <a href="https://github.com/buckster123/Imaginarium-RS">Imaginarium-RS</a> job <code>01M0DESM366XFHP7RMCBTQZTF8</code>.</sub>
