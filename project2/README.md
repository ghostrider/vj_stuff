# Project 2: Procedural Cubes

A simple Unreal Engine 5.4 project demonstrating procedural generation of a cube structure.

## Description

This project contains a simple procedural generation system that creates a cube made up of smaller boxes. The system generates a 10x10x10 arrangement of individual cube meshes with customizable spacing.

## Features

- **Procedural Cube Generator Actor**: An actor that automatically generates a cube structure
- **10x10x10 Grid**: Creates 1000 individual cube meshes arranged in a cubic pattern
- **Customizable Parameters**:
  - Cube Size: Number of boxes in each dimension (default: 10)
  - Box Size: Size of each individual box in cm (default: 100cm = 1m)
  - Gap Size: Space between boxes in cm (default: 25cm = 0.25m)

## Usage

1. Open the project in Unreal Engine 5.4
2. In the editor, place a `ProceduralCubeGenerator` actor in your level
3. The cube will automatically generate with the default settings (10x10x10 boxes with 0.25 unit gaps)
4. Adjust the parameters in the Details panel:
   - `Cube Size`: Change the dimensions of the cube grid
   - `Box Size`: Adjust the size of individual boxes
   - `Gap Size`: Modify the spacing between boxes

## Technical Details

- **Module Name**: ProceduralCubes
- **Main Actor**: AProceduralCubeGenerator
- **Generation Method**: OnConstruction (runs in editor and at runtime)
- **Mesh Used**: Engine's default cube mesh (`/Engine/BasicShapes/Cube`)

## Code Structure

```
Source/ProceduralCubes/
├── Public/
│   ├── ProceduralCubes.h           # Module header
│   └── ProceduralCubeGenerator.h    # Generator actor header
└── Private/
    ├── ProceduralCubes.cpp          # Module implementation
    └── ProceduralCubeGenerator.cpp  # Generator actor implementation
```

## Requirements

- Unreal Engine 5.4
- C++ compiler (Visual Studio 2022 on Windows, Clang on Mac/Linux)
