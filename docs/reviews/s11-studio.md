# S11 studio — design-team freeze (2026-08-19)

Three briefs, one look. Source files: visual / HTMX / honesty.

**Look:** Camera-ring groutbench. Warm charcoal tesserae, 2px grout, honesty as colour. Not Meshy purple, not OmniOcular navy glass.

**Palette:** bg `#141210` · surface `#1e1b18` · ink `#efe7d8` · amber `#e0a04a` (degraded) · teal `#3bb8c8` (true PBR succeeded only) · fail `#c45c4a` · grout `#2a2622`. **No green.**

**Signature:** hex View Contract stills orbit an empty well *before* `<model-viewer>`. Degraded turns grout + well rim amber. Download: `Download degraded GLB` vs `Download GLB`.

**HTMX:** always-on in `-api`. `GET /`, fragments `/ui/*`. Poll `every 2s` until terminal omits `hx-get`. Loopback only; off-loopback 404s the studio.

**Honesty:** system-check uses `ready` / `would_pick`, never `ok`. Estimate before submit. Mock/vertex-colour never a tick.
