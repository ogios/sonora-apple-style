# GPUI glass rendering extension

This directory contains the four modified GPUI packages from
[sonorahq/gpui](https://github.com/sonorahq/gpui/tree/7eea897d4765f59dc9dac1ca468a91e84bb518a6),
revision `7eea897d4765f59dc9dac1ca468a91e84bb518a6`, under `LICENSE-APACHE`.
The font fixtures retain their licenses under `assets/fonts/`.

Sonora's root `Cargo.toml` patches these packages. `gpui` and `gpui_platform`
retain their matching upstream revision; unchanged sibling packages still come
from that revision. The local workspace manifest redirects those sibling paths
to the pinned Git source and omits upstream's unrelated crates.io patches.
Each local package explicitly identifies this nested workspace.

## Rendering changes

- `gpui/src/style.rs` and `styled.rs`: opt-in `GlassMaterial` and
  `backdrop_glass`, independent of the existing `backdrop_blur` setting.
- `gpui/src/window.rs` and `scene.rs`: scale optical dimensions into device
  pixels and carry the glass parameters with the existing backdrop primitive.
- `gpui_wgpu/src/shaders.wgsl`, `gpui_apple/src/shaders.metal`, and
  `gpui_windows/src/shaders.hlsl`: derive the bevel normal from the same signed
  distance function used to clip the rounded surface, displace the live
  sharp background sample using Snell's law through a curved bevel, and apply subtle
  dispersion and Fresnel-weighted directional highlights.
- The three renderer implementations retain a separate sharp backdrop texture alongside
  the optional blurred interior. The bevel mixes in the clear snapshot, so blur no longer
  erases the refractive detail. A zero-blur lens copies the background without blur passes.
- `gpui_wgpu/src/shaders_webgl.wgsl`: keep the unused WebGL backdrop stub's
  record constructor compatible with the extended primitive.

The interior of the surface keeps its ordinary background sample. Extra optical
sampling is confined to the bevel. A circular cross-section supplies the surface normal;
refracted rays project through its local thickness and meet the original sample at both
ends. The sharp sample blends continuously into the blurred interior, avoiding a plateau
of repeated pixels that looked like a stretched strip. Samples bend inward, staying inside the
existing blur region; clipping and the two-pass premultiplied-alpha composite
retain their original behavior. Zero refraction preserves ordinary blur.
Sonora enables the material only when its effects setting is enabled, retaining the
plain fallback when effects are disabled. Selection lenses render after their labels
with gentler optics so the glyphs themselves refract without losing their shapes; the lens
has no hitbox and leaves input to the controls.

This is a custom optical glass material, not Apple's system material. Lighting
has a fixed direction; pointer-tracked lighting and merging multiple surfaces
are not implemented. Existing renderer restrictions remain: WebGL omits
backdrops, and backdrops inside offscreen filter layers are skipped.

## Updating

Update the two upstream dependency pins together. Refresh these four packages
and the font fixtures from that revision, preserve the local manifest wiring,
and reapply the changes listed above. Do not edit Cargo's checkout cache.

Validate the WGSL storage-buffer, WebGL, and subpixel combinations with the Naga
revision used by wgpu, then check and build Sonora. Validate HLSL and Metal on
those platforms before distributing their builds. Linux runtime verification
does not establish native Metal or DirectX compatibility.
