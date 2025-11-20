# VJ Stuff - Multi-Project Repository

This repository contains multiple Unreal Engine 5.4 projects for various visual and procedural generation experiments.

## Projects

### Project 1: VJ Animations
Location: `project1/`

The original VJ (Video Jockey) animations project featuring real-time visual effects, audio processing, and camera capture functionality.

**Key Features:**
- Real-time visual effects and animations
- Audio processing and visualization
- USB camera/video capture integration
- DMX, OSC, and MIDI support
- Keyboard shortcuts for scene switching

**Documentation:**
- [Main README](project1/README.md)
- [Getting Started Guide](project1/GETTING_STARTED.md)
- [Camera Capture Guide](project1/CAMERA_CAPTURE_GUIDE.md)
- [Keyboard Shortcuts](project1/KEYBOARD_SHORTCUTS.md)

### Project 2: Procedural Cubes
Location: `project2/`

A simple demonstration project for procedural generation, creating a 10x10x10 cube structure made of individual box meshes.

**Key Features:**
- Procedural cube generation actor
- 10x10x10 grid of boxes with customizable spacing
- Gap of 0.25 units between boxes
- Real-time regeneration in editor

**Documentation:**
- [Project 2 README](project2/README.md)

## Requirements

- Unreal Engine 5.4
- C++ compiler:
  - Windows: Visual Studio 2022
  - Mac: Xcode
  - Linux: Clang

## Getting Started

Each project is self-contained and can be opened independently:

1. Navigate to the desired project folder (`project1/` or `project2/`)
2. Double-click the `.uproject` file to open in Unreal Engine
3. If prompted, allow Unreal Engine to build the project modules

## Repository Structure

```
vj_stuff/
├── project1/              # VJ Animations project
│   ├── Config/
│   ├── Content/
│   ├── Source/
│   ├── Plugins/
│   └── VJAnimations.uproject
├── project2/              # Procedural Cubes project
│   ├── Config/
│   ├── Content/
│   ├── Source/
│   └── ProceduralCubes.uproject
└── README.md              # This file
```

## Development

Each project maintains its own:
- Source code
- Content assets
- Configuration files
- Build settings
- Documentation

Projects can be developed independently without affecting each other.
