# CLAUDE.md — VJ Stuff Monorepo

## Repository Structure

This is a monorepo with two independent subprojects:

```
vj_stuff/
├── vj-unreal/    # Unreal Engine 5.4 VJ system (inactive)
├── glic-wire/    # GLIC → Resolume Wire port (active)
├── CLAUDE.md
└── README.md
```

## Subprojects

### vj-unreal/
Real-time VJ animation system built with Unreal Engine 5.4. C++ source in `Source/VJAnimations/`. Audio-reactive visuals, USB camera capture, DMX, OSC. **Not actively developed** — exists as reference.

### glic-wire/ (Active)
Port of [GLIC (Glitch Image Codec)](https://github.com/GlitchCodec/GLIC) to Resolume Wire as real-time GLSL video effects.

- **Source format**: ISF (Interactive Shader Format) — `.fs` fragment shaders with JSON metadata header
- **Target**: Resolume Arena/Avenue via Wire
- **Language**: GLSL (fragment shaders)
- **Structure**:
  - `src/` — ISF shader files (.fs, .vs)
  - `docs/` — Analysis and documentation
  - `presets/` — Preset configurations

## ISF Shader Conventions

ISF files use this structure:
```glsl
/*{
  "ISFVSN": "2",
  "DESCRIPTION": "...",
  "CATEGORIES": ["GLIC"],
  "INPUTS": [
    { "NAME": "inputImage", "TYPE": "image" },
    { "NAME": "intensity", "TYPE": "float", "DEFAULT": 0.5, "MIN": 0.0, "MAX": 1.0 }
  ]
}*/

void main() {
  vec4 color = IMG_NORM_PIXEL(inputImage, isf_FragNormCoord);
  gl_FragColor = color;
}
```

Key ISF uniforms: `isf_FragNormCoord` (normalized coords), `TIME`, `RENDERSIZE`, `PASSINDEX`, `TIMEDELTA`.

Use `IMG_NORM_PIXEL(sampler, coord)` to sample textures (not raw `texture2D`).

## GLIC Reference

Original GLIC features to port:
- 16 colorspace conversions (RGB, YCbCr, LAB, HSB, XYZ, LUV, OHTA, etc.)
- 16 prediction algorithms (Paeth, median, JPEG-LS, TrueMotion, etc.)
- 68 wavelet transforms (Haar, Daubechies, Symlets, Biorthogonal, CDF, etc.)
- Quantization, segmentation, glitch visualization

Source repo: https://github.com/GlitchCodec/GLIC

## Git Conventions

- Branch prefix: `claude/`
- Commit messages in English
- Push target: `origin claude/glsl-video-effect-ol9my`
