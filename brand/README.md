# Trellis — brand assets

The mark is a trellis panel that resolves into a **T**: a top rail, a centre post,
and two tapering rungs. The lowest rung is amber — the newest growth, the part
the protocol adds. Four strokes, one weight, round caps.

## Files

| File | Use |
| --- | --- |
| `trellis-mark.svg` | Primary mark, full colour, light backgrounds |
| `trellis-mark-dark.svg` | Primary mark tuned for dark backgrounds |
| `trellis-mark-mono.svg` | Single-colour mark (ink) — stamps, embroidery, fax-grade reproduction |
| `trellis-lockup.svg` | Mark + wordmark, horizontal, light backgrounds |
| `trellis-lockup-dark.svg` | Mark + wordmark, dark backgrounds |
| `favicon.svg` | Mark knocked out of a vine tile, 14/64 corner radius |
| `favicon.ico` | 16 / 32 / 48 / 64 / 128 / 256 px bundle |
| `png/` | Raster fallbacks at the sizes listed in each filename |

Wordmark text in the lockup SVGs is converted to outlines, so the files render
identically without the font installed.

## Colour

| Token | Hex | Role |
| --- | --- | --- |
| Vine | `#1C6B55` | Primary. The structure. |
| Amber | `#E39A3C` | Accent. Use once per composition. |
| Ink | `#14201C` | Wordmark and body text on light |
| Paper | `#F7F5F0` | Light ground |
| Vine Light | `#4FBF9B` | Primary on dark grounds |
| Amber Light | `#F0B460` | Accent on dark grounds |

Vine on Paper is 6.4:1. Ink on Paper is 15.8:1. Both pass AA at any size.
Amber is a graphic accent, never body text.

## Rules

- **Clear space**: the width of the lower rung — 12 units of the 64-unit grid,
  roughly 19% of the mark's width — on all four sides.
- **Minimum size**: 16px for the mark, 88px wide for the lockup. Below 24px use
  `favicon.svg` rather than the bare mark; the tile holds the form together.
- Do not recolour the rungs individually, rotate the mark, add a gradient, or
  outline it. If the background is busy, use the favicon tile.
- On photography, use the tile or the dark lockup — never the bare mark.

## Type

Wordmark: **Poppins Medium**, tracking −1.2%. Poppins is geometric and monoline,
which matches the mark's construction. Cap height sits at 70% of the mark height,
baseline optically centred against the mark.

## Regenerating

`gen.py` in the design working files builds every asset from the four stroke
coordinates. Edit the geometry there rather than by hand, so the SVG, PNG and ICO
outputs stay in sync.
