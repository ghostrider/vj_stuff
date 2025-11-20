# VJ Blueprints

This directory contains Blueprint classes for VJ visual effects and scene management.

## Directory Structure

```
Blueprints/
├── Scenes/          # Individual scene blueprints
├── Effects/         # Reusable effect blueprints
├── AudioReactive/   # Audio-reactive visual components
└── Utilities/       # Helper blueprints
```

## Creating a New Scene

1. Right-click in `Blueprints/Scenes/`
2. Create Blueprint Class → Actor
3. Name it `BP_Scene_[YourSceneName]`
4. Add visual components (Static Meshes, Niagara Systems, etc.)
5. Add the scene actor to MainVJStage level
6. Add it to the VJSceneManager's Scenes array

## Audio Reactive Blueprints

To create audio-reactive effects:

1. Get reference to VJAudioAnalyzer
2. Read audio properties (AudioAmplitude, LowFrequencyEnergy, etc.)
3. Drive material parameters, transforms, or particle systems
4. Subscribe to OnBeatDetected event for beat-synced effects

Example nodes in Blueprint:
- Get VJAudioAnalyzer reference
- Get AudioAmplitude
- Multiply by scale factor
- Set material parameter or actor scale

## Material Parameters

Common dynamic material parameters for VJ effects:
- `Intensity` - Overall effect strength
- `Color` - Hue or color tint
- `Speed` - Animation speed
- `Scale` - Pattern scale
- `AudioReactivity` - Audio influence amount
