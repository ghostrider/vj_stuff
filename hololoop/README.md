# Hololoop

Port of [Hololoop](https://github.com/wabisabit/Hololoop) to Resolume Wire as real-time GLSL/ISF video effects.

## Overview

Hololoop is an audio-reactive visual performance tool originally built in Processing (P3D). It features three visual modes — ray grids, golden-ratio rectangle paths, and expanding pulse rings — plus a psychedelic color modifier. This project ports those visual systems to GPU-accelerated GLSL shaders in ISF format for use in Resolume Arena/Avenue via Wire.

The original is a mouse-driven drawing tool; this port reimagines each mode as an **image-reactive video effect** where input video content drives the visuals that were previously controlled by mouse interaction.

## Structure

```
hololoop/
  src/       # ISF shader files (.fs)
  docs/      # Documentation and analysis
  presets/   # Preset configurations
```

## Shaders

| Shader | Original Mode | Description |
|--------|---------------|-------------|
| `hololoop-rays.fs` | Mode 1 (Nodes/Rays) | Grid of nodes emitting rays toward bright regions. Ray count and length driven by luminance. Animated rotation, audio-reactive growth. |
| `hololoop-paths.fs` | Mode 2 (Paths) | Golden-ratio (1.618) rectangle mosaic oriented along image gradients. 7 color palettes from the original. Dark fill / light stroke rendering. |
| `hololoop-pulses.fs` | Mode 3 (Pulses) | Expanding concentric circle pulses from a grid of points. Activity driven by image brightness. Per-ring random opacity and expansion rate. |
| `hololoop-psycho.fs` | Psycho mode (`p` key) | Psychedelic color chaos, block displacement, palette tinting, and Z-depth brightness variation. Standalone effect combining the original's psycho modifier. |

## Original Feature Mapping

| Original Feature | ISF Equivalent |
|-----------------|----------------|
| Mouse drawing | Image luminance / gradient drives visuals |
| Audio amplitude (`rms.analyze()`) | `audioReactivity` parameter (simulated from image content) |
| Rotation toggle (`r` key) | `rotationSwitch` boolean input |
| Color toggle (`c` key) | `colorSwitch` boolean input |
| Psycho mode (`p` key) | `psychoSwitch` boolean or standalone `hololoop-psycho.fs` |
| Eraser mode (backspace) | N/A (stateless shader) |
| 7 color palettes | `palette` selector (0-6) with original hex colors preserved |
| Golden ratio (1.618) | Rectangle proportions: `width = height * 1.618` |
| P3D Z-depth | Simulated via displacement and brightness variation |

## Color Palettes

All 7 original palettes are preserved:

| Index | Name | Colors |
|-------|------|--------|
| 0 | Warm Sand | `#BBBB88` `#CCC68D` `#EEDD99` `#EEC290` `#EEAA88` |
| 1 | Coral Garden | `#FF4242` `#F4FAD2` `#D4EE5E` `#E1EDB9` `#F0F2EB` |
| 2 | Watermelon | `#D1F2A5` `#EFFAB4` `#FFC48C` `#FF9F80` `#F56991` |
| 3 | Mint Noir | `#CFFFDD` `#B4DEC1` `#5C5863` `#A85163` `#FF1F4C` |
| 4 | Autumn | `#B3CC57` `#ECF081` `#FFBE40` `#EF746F` `#AB3E5B` |
| 5 | Fiesta | `#CC0C39` `#E6781E` `#C8CF02` `#F8FCC1` `#1693A7` |
| 6 | Ocean Neon | `#1693A5` `#02AAB0` `#00CDAC` `#7FFF24` `#C3FF68` |

## Target Platform

- **Resolume Wire** (Arena/Avenue plugin)
- **Shader format**: ISF (Interactive Shader Format) — GLSL fragment shaders with JSON metadata
- **Real-time**: 60 FPS target for live performance
