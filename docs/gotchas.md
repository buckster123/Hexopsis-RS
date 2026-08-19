# Gotchas — the invariant ledger

> **RULE: before modifying ANY subsystem, grep this file for it and read the matching
> entries.** These are load-bearing invariants.

- **MCP stdout is sacred.** JSON-RPC only. `tracing` → stderr. A stray `println!` on the MCP path is a protocol bug.

- **Mock is never `succeeded`.** `local.mock` always terminates `degraded` with `export.material_mode=vertex_color` and `disclaimer=not-a-model`. Auto-select requires `TEXT2MESH_ALLOW_MOCK=1`.

- **No orphan `pending`.** JSON status is never the string `pending`.

- **Wait/wall clock is one number.** MCP `timeout_s`, CLI `--timeout-s`, Route B `max_wall_s`: min 30, default 1800, max 86400.

- **Watchdog does not kill on silence.** Live child pid + no progress line = alive. `TEXT2MESH_HB_S` default 300.

- **No `XAI_API_KEY` in this process.** Imaginarium holds the Imagine key.

- **Appendix B is not a clone list.** Implementers do not follow PRD Appendix B GitHub URLs.

- **Hunyuan never auto.** Even with D19 gates, auto prefers colony/tripo/meshy/local.

- **Shared iGPU is not 22 GiB VRAM.** Count `vram_mb` + `shared`. Krackan 512 MiB shared → remote or degrade.
