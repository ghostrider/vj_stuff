# GLIC-Wire

Port of [GLIC (Glitch Image Codec)](https://github.com/GlitchCodec/GLIC) to Resolume Wire as real-time GLSL/ISF video effects.

## Overview

GLIC is a glitch image codec built in Processing that uses colorspace conversion, prediction, wavelet transforms, quantization, and segmentation to produce glitch art. This project ports those algorithms to GPU-accelerated GLSL shaders in ISF format for use in Resolume Arena/Avenue via Wire.

## Structure

```
glic-wire/
  src/       # ISF shader files (.fs / .vs)
  docs/      # Documentation and analysis
  presets/   # Preset configurations
```

## Original GLIC Features (Port Targets)

- 16 colorspace conversions (RGB, YCbCr, HSV, LAB, etc.)
- 16 prediction algorithms (differential, gradient, median, etc.)
- 68 wavelet transforms (Haar, Daubechies, CDF, etc.)
- Quantization with configurable step sizes
- Segmentation modes (scanline, block, hilbert curve, etc.)
- Glitch visualization of codec internals

## Target Platform

- **Resolume Wire** (Arena/Avenue plugin)
- **Shader format**: ISF (Interactive Shader Format) - GLSL fragment shaders with JSON metadata
- **Real-time**: 60 FPS target for live performance
