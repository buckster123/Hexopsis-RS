# Gotchas — the invariant ledger

> **RULE: before modifying ANY subsystem, grep this file for it and read the matching
> entries.** These are load-bearing invariants.

- **MCP stdout is sacred.** JSON-RPC only. `tracing` → stderr. A stray `println!` on the MCP path is a protocol bug.

- **Mock is never `succeeded`.** `local.mock` always terminates `degraded` with `export.material_mode=vertex_color` and `disclaimer=not-a-model`. Auto-select requires `TEXT2MESH_ALLOW_MOCK=1`.

- **No orphan `pending`.** JSON status is never the string `pending`.

- **Wait/wall clock is one number.** MCP `timeout_s`, CLI `--timeout-s`, Route B `max_wall_s`: min 30, default 1800, max 86400.

- **Watchdog does not kill on silence.** Live child pid + no progress line = alive. `TEXT2MESH_HB_S` default 300.

- **No `XAI_API_KEY` in this process.** Imaginarium holds the Imagine key.

- **Paid T2I is estimate-then-fire.** `T2iProvider::estimate` always runs first (OQ-9). Do not hardcode 2× I2I. Closed spend gate → `needs_confirm`, never a silent POST. Edit sources are `library:{id}` or data-URLs — no bare filesystem paths.

- **Sidecar paths stay in the job dir.** `meshplane/1` artifact paths must canonicalize under `jobs/<id>/`. Escape → `engine.crash`. Handshake miss (30 s) → `not_configured`. Protocol mismatch → `unsupported`. Exit ≠ 0 → `engine.crash`. Auto never picks a sidecar binary as a substitute for VRAM.

- **Tests never hit live :8791.** `App::for_test` / injected `probe` skip Imaginarium health. Live estimate/generate is `TEXT2MESH_LIVE=1` only.

- **Missing vendor key never POSTs.** Empty/absent `MESHY_API_KEY` / `TRIPO_API_KEY` is `not_configured` before any HTTP. 401 is only “key present but rejected.” 402 → `spend.provider_402`. 429 → `rate_limit` + `Retry-After`. Remote poll expiry is `waiting_upstream`, not local `failed`.

- **Export honesty is the `gltf` crate.** Unparseable → `engine.crash`. Default-only factors + no COLOR_0 + no textures → `failed` `export.materials_missing`. Vertex colour / factors-only → `degraded`. True UV/PBR may `succeeded`. Mock is never succeeded.

- **WebUI is loopback-only.** `GET /` is the groutbench. Degraded is amber, never a green tick. Download on mock is “Download degraded GLB”. Off-loopback the studio 404s; `/v1` still uses the bearer.

- **Appendix B is not a clone list.** Implementers do not follow PRD Appendix B GitHub URLs.

- **Hunyuan never auto.** Even with D19 gates, auto prefers colony/tripo/meshy/local.

- **Shared iGPU is not 22 GiB VRAM.** Count `vram_mb` + `shared`. Krackan 512 MiB shared → remote or degrade. Never treat GTT, rocminfo pools, vulkan host heaps, or `/proc/meminfo` as VRAM.

- **No auto-pull on generate.** `text2mesh weights pull ID --accept-license TAG` is CLI-only. Hunyuan ids refuse. DINOv3 on disk without accept → `present:true`, `accepted:false`.

- **Idle unload.** API/MCP start with no sidecar child. `TEXT2MESH_IDLE_UNLOAD_S` default 120 kills leftovers. A sidecar *file* is not 24 GB of VRAM.
