# VJ Animations - Unreal Engine Project

A real-time VJ (Video Jockey) animation and visual effects system built with Unreal Engine 5.4.

## Overview

This project provides a foundation for creating stunning real-time visual effects, animations, and interactive displays for live performances, concerts, events, and installations.

## Features

- Real-time rendering optimized for live performance (60 FPS target)
- **Audio processing and analysis** for reactive visuals
- **Keyboard shortcuts** for instant scene switching (1-0 keys)
- Support for DMX lighting control
- OSC (Open Sound Control) integration for external control
- Niagara particle systems for advanced effects
- Media playback and processing capabilities
- Modular blueprint system for easy customization
- Scene management system with fade and blackout controls

## Project Structure

```
VJAnimations/
├── Config/              # Engine and project configuration
├── Content/             # Project assets
│   ├── Maps/            # Level maps
│   ├── Materials/       # Material assets
│   ├── Particles/       # Niagara particle systems
│   ├── Blueprints/      # Blueprint classes
│   ├── Audio/           # Audio files
│   └── Video/           # Video assets
├── Plugins/             # Third-party and custom plugins
├── Source/              # C++ source code
│   └── VJAnimations/    # Main game module
└── Saved/               # Generated files and logs
```

## Prerequisites

- Unreal Engine 5.4 or later
- Windows, Linux, or macOS
- Recommended: NVIDIA RTX GPU for optimal real-time performance
- Git LFS (for asset storage)

## Getting Started

### Initial Setup

1. Clone this repository:
   ```bash
   git clone <repository-url>
   cd vj_stuff
   ```

2. Install Git LFS if not already installed:
   ```bash
   git lfs install
   ```

3. Right-click on `VJAnimations.uproject` and select "Generate Visual Studio project files" (Windows) or run:
   ```bash
   # On Linux/Mac
   /path/to/UnrealEngine/Engine/Build/BatchFiles/Linux/GenerateProjectFiles.sh VJAnimations.uproject
   ```

4. Open `VJAnimations.uproject` in Unreal Engine

### Building from Source

To compile the C++ code:

1. Open the generated solution file in your IDE (Visual Studio, Rider, etc.)
2. Build the project in Development Editor configuration
3. Launch the editor from your IDE or open the .uproject file

## Enabled Plugins

- **DMX Protocol**: Control DMX lighting systems
- **Media Plate**: Advanced media playback
- **Media Framework**: Video and audio processing
- **Niagara**: Next-generation VFX system
- **OSC**: Open Sound Control for external device integration
- **Audio Capture**: Real-time audio input capture
- **Audio Synesthesia**: Frequency spectrum analysis
- **Audio Modulation**: Advanced audio processing
- **Synthesis**: Audio synthesis capabilities

## Usage

### Keyboard Shortcuts

The VJ system includes the following keyboard shortcuts for live performance:

**Scene Switching:**
- `1-9`, `0` - Switch directly to scene 1-10
- `,` (Comma) - Previous scene
- `.` (Period) - Next scene

**Audio Control:**
- `A` - Toggle audio reactive mode on/off

**Master Controls:**
- `B` - Toggle blackout (instantly hide all scenes)
- `=` (Plus/Equal) - Master fade in
- `-` (Minus/Hyphen) - Master fade out
- `Mouse Wheel` - Adjust audio intensity (when applicable)

### Creating Visual Effects

1. Open the MainVJStage map in `Content/Maps/`
2. Use Blueprint actors to create interactive visual elements
3. Configure Niagara systems for particle effects
4. Set up materials with dynamic parameters for real-time control
5. Make effects audio-reactive by reading values from VJAudioAnalyzer

### Audio Reactive Visuals

The VJAudioAnalyzer provides real-time audio analysis:
- **AudioAmplitude**: Overall volume level
- **LowFrequencyEnergy**: Bass frequencies
- **MidFrequencyEnergy**: Mid-range frequencies
- **HighFrequencyEnergy**: Treble frequencies
- **AudioIntensity**: Combined intensity value
- **OnBeatDetected**: Event triggered on beat detection

Access these values in Blueprints to drive material parameters, particle systems, transforms, and more.

### Scene Management

1. Create scene actors in `Content/Blueprints/Scenes/`
2. Place VJSceneManager actor in your main level
3. Add scene actors to the SceneManager's Scenes array
4. Use keyboard shortcuts to switch between scenes during performance

### External Control

The project supports OSC for external control via software like TouchDesigner, Max/MSP, or custom applications.

## Development Workflow

1. Create new content in the appropriate Content subdirectories
2. Use Blueprints for rapid prototyping
3. Implement performance-critical features in C++
4. Test in PIE (Play In Editor) mode with realistic performance settings

## Performance Considerations

- Target framerate: 60 FPS (configurable in DefaultEngine.ini)
- Use LODs for complex meshes
- Optimize materials for real-time rendering
- Profile regularly using Unreal Insights

## Contributing

1. Create a feature branch from main
2. Make your changes
3. Test thoroughly in editor and packaged builds
4. Submit a pull request

## Troubleshooting

### Project won't open
- Verify Unreal Engine 5.4+ is installed
- Check that all plugins are available
- Try regenerating project files

### Performance issues
- Check GPU driver is up to date
- Reduce visual quality settings in Project Settings
- Profile using GPU Visualizer (Ctrl+Shift+,)

## Resources

- [Unreal Engine Documentation](https://docs.unrealengine.com/)
- [Niagara Visual Effects](https://docs.unrealengine.com/5.4/en-US/creating-visual-effects-in-niagara-for-unreal-engine/)
- [DMX Protocol](https://docs.unrealengine.com/5.4/en-US/dmx-in-unreal-engine/)
- [OSC Plugin](https://docs.unrealengine.com/5.4/en-US/osc-open-sound-control-in-unreal-engine/)

## License

[Add your license here]

## Contact

[Add contact information here]
