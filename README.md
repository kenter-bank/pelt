# pelt

**SVG "pelt" skins as a UI appearance layer.** A *pelt* is a full SVG canvas
stretched over an element's bounding rect — the visual surface, independent of
layout. Where CSS uses property/value pairs, a pelt is **drawn** (in Inkscape or
any SVG editor) and carries a tiny `pelt:` convention set. Your layout engine
positions/sizes elements; the pelt renders their look; content draws on top.

The core rasterizes on the CPU (`usvg` + `resvg`) to a
premultiplied-RGBA buffer, which the host composites into its GPU surface (e.g. a
[Vello](https://github.com/linebender/vello) image) for a clean, controllable
per-frame loop.

## What a pelt gives you

- **Theme tokens** — `var(--pelt-<name>, fallback)` in the SVG, substituted before
  render, so one skin re-themes without touching the art.
- **State variants** — `data-pelt-state="default|hover|active|focus|disabled|
  checked"` groups; the app picks which state to draw.
- **9-slice scaling** — `pelt:scale="9-slice"` + `<pelt:slices …>`, so one
  button/panel skin scales to any size crisply.
- **Animation** *(host-driven)* — animatable params and state transitions the host
  ticks each frame (the abstraction native renderers need, since none play
  SMIL/CSS-in-SVG).

## Usage

```rust
let mut p = pelt::Pelt::load("skins/button-primary.svg")?;
p.set_token("accent", "#39ff14");   // theme
p.set_state("hover");               // state variant
let img = p.render(240, 60)?;       // img.rgba: premultiplied RGBA, ready to upload
```

See `samples/` for real pelts (`neon-button.svg`, `glass-card.svg`) and
`tests/render_sample.rs` for an end-to-end render.

## The workflow it enables

An app author wires **structure + behavior** in code and attaches a **named pelt**
(plus a placeholder `.svg`) to each element. A designer then opens each `.svg` in
Inkscape and **redraws the look** — no code changes. Appearance is drawn, not coded.

## Status

`0.1` — core renderer (tokens, state variants, 9-slice) working. Animation /
transitions controllers and the `pelt-kit` authoring CLI are being ported next.

## License

MPL-2.0.
