<div align="center">

<img src="assets/banner.jpg" alt="Tessera-RS / text2mesh" width="100%">

<h1>text2mesh <em>(Tessera-RS)</em></h1>

<p><strong>Image or text in. Honest glTF 2.0 GLB out.</strong><br>
Local/onboard inference <em>or</em> a networked provider, one <code>MeshJob</code>, three faces.</p>

<p>
<img alt="license" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue">
<img alt="rust" src="https://img.shields.io/badge/rust-2021-orange?logo=rust&logoColor=white">
<img alt="ci" src="https://img.shields.io/github/actions/workflow/status/buckster123/Tessera-RS/ci.yml?label=ci">
<img alt="status" src="https://img.shields.io/badge/status-v0.1%20·%20S11-brightgreen">
</p>

</div>

---

> [!NOTE]
> Clean-room. Implement from `docs/prd.md` + `docs/design.md` + `docs/CHARTER.md`.
> Do **not** clone or open AI_trellis2cpp / TRELLIS / Hunyuan / TripoSR source.

## What it is

**text2mesh** (garden name **Tessera-RS**) turns a still image or a text prompt into a
portable glTF 2.0 GLB with core PBR. The text path is an original **Lattice Router +
typed View Contract + Hero-Orbit** loop that feeds the **same** image-to-mesh plane —
not a fire-and-forget T2I call. Dual compute: local sidecar/preview **and** remote
(Meshy / Tripo / colony). Mock and vertex-colour paths terminate **`degraded`**, never
a fake success.

## Install

```sh
git clone https://github.com/buckster123/Tessera-RS
cd Tessera-RS
cargo build --release --workspace
```

## Use

```sh
# honesty probe (always free)
text2mesh system-check --json

# mock path (CI / Nano) — status is degraded, vertex colour, exit 1
TEXT2MESH_ALLOW_MOCK=1 text2mesh generate --image ./photo.png \
  --compute local --provider local.mock --json

# local sidecar (meshplane/1). Fixture child ships as meshplane-fixture.
TEXT2MESH_SIDECAR=./target/release/meshplane-fixture \
  text2mesh generate --image ./photo.png --compute local --provider local.sidecar --json

# text path with Imaginarium T2I (spend gated; estimate is free)
text2mesh estimate --prompt "a red fox wearing a yellow raincoat" --json
TEXT2MESH_ALLOW_SPEND=1 TEXT2MESH_ALLOW_UNGATED=1 text2mesh generate \
  --prompt "a red fox wearing a yellow raincoat" --allow-spend --json

# remote (Meshy / Tripo) — spend gated; fixtures in CI; live needs a real key
TEXT2MESH_ALLOW_SPEND=1 text2mesh generate --image ./photo.png \
  --compute remote --provider remote.tripo --allow-spend --json
```

## How it works

One `MeshJob` schema (`text2mesh.job.v1`) is shared by CLI, MCP, and HTTP.
`auto` is a pure planner over probes. Spend is gated. See [`docs/design.md`](docs/design.md).

Loopback studio (HTMX, amber degrade, hex View Contract ring):

```sh
text2mesh serve   # http://127.0.0.1:8796/
```

## Docs

| File | What's in it |
|------|--------------|
| [`docs/CHARTER.md`](docs/CHARTER.md) | Binding D1–D30 |
| [`docs/design.md`](docs/design.md) | The contract |
| [`docs/prd.md`](docs/prd.md) | Product requirements |
| [`BACKLOG.md`](BACKLOG.md) | Slice ledger |

## License

MIT OR Apache-2.0 — see [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

<sub>Banner generated with <a href="https://github.com/buckster123/Imaginarium-RS">Imaginarium-RS</a> (job 01M0DESM366XFHP7RMCBTQZTF8). Alt candidate 01M0DESM36GT3AWJM8XGYX2DBG.</sub>
