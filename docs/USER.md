# Using text2mesh

Operator guide. Binding product rules: [`CHARTER.md`](CHARTER.md). Wire: [`design.md`](design.md). v1 close-out: [`STATUS.md`](STATUS.md).

Binary name is `text2mesh`. Garden name is **Hexopsis-RS**. Default bind is `127.0.0.1:8796`.

## Install

```sh
git clone https://github.com/buckster123/Hexopsis-RS
cd Hexopsis-RS
cargo build --release --workspace
# binaries: target/release/text2mesh  target/release/text2mesh-mcp  target/release/meshplane-fixture
```

Default `cargo test --workspace` needs no CUDA, ONNX, ggml, or multi-GB weights.

## First mesh (mock — always `degraded`)

```sh
TEXT2MESH_ALLOW_MOCK=1 text2mesh generate \
  --image crates/text2mesh/tests/fixtures/dot.png \
  --compute local --provider local.mock --json
```

CLI exit **1** is correct. The GLB is parser-valid vertex colour with `disclaimer=not-a-model`. It is **not** a quality mesh and never `succeeded`.

## Honesty probe (free)

```sh
text2mesh system-check --json
```

Read `report_complete` and `ready` (`planner.would_pick != null`). There is no `ok` field for readiness. `devices[].vram_mb` is **device VRAM**, never host RAM. Shared iGPU / `< 6 GiB` → auto is preview, remote, or degrade.

Estimate is also free and never POSTs paid work:

```sh
text2mesh estimate --prompt "a red fox wearing a yellow raincoat" --json
```

## Studio

```sh
TEXT2MESH_ALLOW_MOCK=1 text2mesh serve   # http://127.0.0.1:8796/
```

Loopback HTMX groutbench. Degraded jobs show an **amber** banner, never a green tick. Off-loopback the studio 404s; `/v1` then needs `Authorization: Bearer $TEXT2MESH_TOKEN`.

## Text path

Lattice classifies the prompt. Dimensioned CAD (`box 20x10x5 mm`) goes to Cadre or **refuses** (`analytic.unavailable`). Visual prompts compile a View Contract, run Hero-Orbit T2I (Imaginarium if live; mock T2I if `TEXT2MESH_ALLOW_MOCK=1`), then the same image-to-mesh plane.

CLIP G0–G2 need a gate encoder. Without it, set `TEXT2MESH_ALLOW_UNGATED=1` to run G3/G4 only (stated `gate.encoder_missing`).

Paid T2I is estimate-then-fire. Closed spend gate → `needs_confirm`, never a silent POST. This process **never** reads `XAI_API_KEY`.

```sh
text2mesh compile --prompt "a red fox wearing a yellow raincoat" --json
TEXT2MESH_ALLOW_SPEND=1 TEXT2MESH_ALLOW_UNGATED=1 text2mesh generate \
  --prompt "a red fox wearing a yellow raincoat" --allow-spend --json
```

## Remote quality (Meshy / Tripo)

```sh
export TRIPO_API_KEY=…          # or MESHY_API_KEY
TEXT2MESH_ALLOW_SPEND=1 text2mesh generate --image ./photo.png \
  --compute remote --provider remote.tripo --allow-spend --json
```

Missing key is `not_configured` in milliseconds — no HTTP. `402` → `spend.provider_402`. `429` → `rate_limit`. Poll expiry after `upstream_id` exists → `waiting_upstream`, not a fake local fail.

Live vendor POSTs stay behind `TEXT2MESH_LIVE=1` in tests.

## Local quality sidecar

v1 local quality is a child speaking **`meshplane/1`**, not an in-process DiT.

```sh
export TEXT2MESH_SIDECAR=./target/release/meshplane-fixture   # fixture: vertex-colour, degraded
text2mesh generate --image ./photo.png --compute local --provider local.sidecar --json
```

A real engine is whatever binary you point at. Handshake 30 s. Paths must stay under `jobs/<id>/`. Auto never treats “sidecar file on disk” as 24 GB of VRAM.

API/MCP start **without** a sidecar child. After the queue is idle, `TEXT2MESH_IDLE_UNLOAD_S` (default **120**) kills leftovers.

## Weights (CLI only)

Never auto-pulled on `generate`. No MCP tool.

```sh
text2mesh weights pull encoder.dinov3_vitl16 --accept-license dinov3
text2mesh weights pull quality.stack --accept-license mit
```

Refuses if `free_mb < want * 1.1`. Hunyuan ids refuse closed (`license.blocked`). DINOv3 file on disk with the flag off → `present:true`, `accepted:false`.

Catalog ids: `preview.feedforward`, `quality.stack`, `encoder.dinov3_vitl16`, `encoder.openclip_vit_b32`, `native.text_dit`.

## MCP

Point the harness at `text2mesh-mcp` (stdio, protocol `2024-11-05`). `text2mesh mcp` only explains that.

```json
{
  "mcpServers": {
    "text2mesh": {
      "command": "/path/to/text2mesh-mcp"
    }
  }
}
```

Tools: `text2mesh_system_check`, `text2mesh_estimate`, `text2mesh_compile_contract`, `text2mesh_submit`, `text2mesh_status`, `text2mesh_wait` (default `timeout_s=1800`), `text2mesh_cancel`, `text2mesh_artifact`, `text2mesh_list_jobs`. Wrapper `ok` means the RPC parsed, not “meshed.” Inspect `job.status`.

## Exit codes

| Code | When |
|---|---|
| 0 | `status=succeeded` only |
| 1 | `status=degraded` (stderr prints `DEGRADED`) |
| 2 | usage |
| 3 | `not_configured` / `weights_missing` / `feature_off` / `disk_short` / `vram_short` |
| 4 | spend / license |
| 5 | engine / upstream |
| 6 | view gates / analytic refuse |
| 7 | cancelled |
| 8 | wait budget ended (`wait_timed_out`) |
| 9 | internal |

`system-check` exits 0 if `report_complete=true`.

## Store

Default `$XDG_DATA_HOME/text2mesh` (usually `~/.local/share/text2mesh`). `TEXT2MESH_STORE=""` is ephemeral. Job ids are ULIDs. GLB + `manifest.json` land under `jobs/<id>/`.

## Env (short)

| Name | Default | Notes |
|---|---|---|
| `TEXT2MESH_BIND` | `127.0.0.1:8796` | non-loopback needs `TEXT2MESH_TOKEN` |
| `TEXT2MESH_STORE` | XDG data | `""` = temp |
| `TEXT2MESH_ALLOW_MOCK` | off | planner may pick mock |
| `TEXT2MESH_ALLOW_SPEND` | off | or `--allow-spend` / per-call |
| `TEXT2MESH_ALLOW_UNGATED` | off | G3/G4 without CLIP |
| `TEXT2MESH_ACCEPT_DINOV3` | off | or `weights pull … --accept-license dinov3` |
| `TEXT2MESH_SIDECAR` | unset | `meshplane/1` binary |
| `TEXT2MESH_IDLE_UNLOAD_S` | `120` | sidecar child reaper |
| `TEXT2MESH_IMAGINARIUM_URL` | `http://127.0.0.1:8791` | T2I sibling |
| `MESHY_API_KEY` / `TRIPO_API_KEY` | unset | length/head only in logs |
| `XAI_API_KEY` | — | **must not be used here** |

Full table: design §22.
