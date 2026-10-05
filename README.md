# LaserCAD 2D

A browser-based 2D CAD drawing web app designed for CNC laser-cutting machine shops. Embed it in your shop's website so customers can draw laser-cut DXF files in-situ — no software install required.

## Features

- **2D Drawing Tools** — Lines, arcs, circles, ellipses, polylines, splines, points, hatches, text, dimensions
- **Parametric Constraints** — Coincident, concentric, equal, perpendicular, tangent, symmetric, fixed, horizontal, dimensional constraints
- **DXF/DWG Support** — Open and save industry-standard DXF and DWG files
- **Snap & OSCAP** — Object snap (endpoint, midpoint, center, intersection, perpendicular, tangent, nearest)
- **Annotation** — Linear, aligned, angular, radial, diameter, ordinate dimensions; leaders and multileaders
- **Modify Tools** — Move, copy, rotate, mirror, scale, trim, extend, fillet, chamfer, offset, array, stretch
- **Layers** — Full layer management with colors, linetypes, and lineweights
- **Blocks** — Create, insert, and edit reusable block definitions
- **PDF Export** — Print/export drawings to PDF
- **Web-First** — Runs in the browser via WebAssembly (also builds as a native desktop app)
- **Localization** — 20+ languages supported

## Target Use Case

Machine shops running CNC laser cutters for metal, wood, or plastic. This web app is designed to be embedded in your shop's website, giving customers a way to:

1. Draw or upload a 2D design
2. Add dimensions and annotations
3. Export a production-ready DXF file
4. Submit it for laser cutting

## Building

### Web (WASM)

```bash
# Prerequisites
rustup target add wasm32-unknown-unknown
cargo install trunk wasm-bindgen-cli

# Development
trunk serve

# Production build
trunk build --release
```

### Native Desktop

```bash
cargo build --release
```

## Architecture

Built with Rust + [iced](https://github.com/iced-rs/iced) GUI framework. Uses WebGPU/WebGL for rendering.

- `src/modules/draw/` — Drawing tools (line, arc, circle, polyline, etc.)
- `src/modules/parametric/` — Parametric constraint system
- `src/modules/annotate/` — Dimensions and annotation tools
- `src/modules/insert/` — Blocks, references, imports
- `src/modules/view/` — Viewport navigation and layout
- `src/scene/` — Rendering pipeline, tessellation, entity conversion
- `src/snap.rs` — Object snap engine
- `src/io/` — DXF/DWG read/write, PDF export

## License

See [LICENSE](LICENSE) for details.
