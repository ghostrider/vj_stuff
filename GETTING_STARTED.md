# Getting Started with VJ Animations

This guide will help you get up and running with the VJ Animations project.

## Quick Start

1. **Open the Project**
   - Double-click `VJAnimations.uproject`
   - Unreal Engine will open and compile shaders on first launch (this takes time)

2. **Set Up the Main Stage**
   - Create a new Empty Level: File → New Level → Empty Level
   - Save it as `MainVJStage` in Content/Maps/
   - Add a VJSceneManager actor from Place Actors → All Classes
   - Add a VJAudioAnalyzer actor from Place Actors → All Classes
   - Add basic lighting (DirectionalLight, SkyLight) for visibility

3. **Create Your First Scene**
   - Navigate to Content/Blueprints/Scenes
   - Right-click → Blueprint Class → Actor
   - Name it "BP_Scene_Test"
   - Add visual components (Static Mesh, Niagara System, etc.)
   - Place the scene actor in your MainVJStage level
   - Select the VJSceneManager and add your scene to its Scenes array

4. **Test Scene Switching**
   - Press Play (Alt+P)
   - Use number keys 1-9, 0 to switch between scenes
   - Press B for blackout, A to toggle audio reactive mode

## Common VJ Workflows

### Real-Time Audio Reactive Visuals

1. Ensure VJAudioAnalyzer is in your level
2. In your Blueprint, get a reference to the Audio Analyzer
3. Read audio properties:
   - AudioAmplitude - overall volume
   - LowFrequencyEnergy - bass
   - MidFrequencyEnergy - mids
   - HighFrequencyEnergy - treble
4. Use these values to drive:
   - Material parameters (intensity, color, speed)
   - Actor transforms (scale, rotation)
   - Niagara particle parameters
   - Light intensity and color
5. Subscribe to OnBeatDetected event for beat-synced effects
6. Toggle audio reactive mode with the `A` key during performance

### DMX Lighting Control

1. Enable DMX plugin (already enabled in this project)
2. Configure DMX Universe settings
3. Create DMX fixtures
4. Control lights from Blueprints or Sequencer

### Video Playback and Processing

1. Import video files to Content/Video
2. Create Media Source assets
3. Use Media Player actors
4. Apply real-time effects via materials

### OSC External Control

1. Configure OSC settings in Project Settings
2. Set up OSC receivers
3. Map OSC messages to Blueprint events
4. Control from external software (TouchDesigner, Max/MSP, etc.)

## Project Organization Tips

- **Materials**: Create master materials for VJ effects, use material instances for variations
- **Blueprints**: Use Blueprint Interfaces for modular control systems
- **Content Browser**: Organize by effect type or performance set
- **Level Streaming**: Use for different performance scenes

## Performance Optimization

### Frame Rate Management
- Target: 60 FPS minimum
- Monitor with `stat fps` console command
- Use `stat unit` to identify bottlenecks

### GPU Optimization
- Use `profilegpu` console command
- Optimize shader complexity
- Balance visual quality vs performance

### Memory Management
- Stream textures and videos
- Unload unused assets
- Monitor with `stat memory`

## Testing Your Content

1. **PIE (Play In Editor)**
   - Press Alt+P to play
   - Fastest iteration time
   - May not reflect final performance

2. **Standalone Game**
   - Press Alt+Shift+P
   - Closer to packaged performance
   - Better for testing

3. **Packaged Build**
   - File → Package Project
   - Most accurate performance testing
   - Required for deployment

## Keyboard Shortcuts

### Editor Shortcuts
- `Alt + P`: Play in editor
- `Esc`: Stop playing
- `F11`: Fullscreen
- `Ctrl + Space`: Content Browser
- `` ` ``: Console commands
- `Ctrl + Shift + ,`: GPU Visualizer

### VJ Performance Shortcuts (in Play mode)
- `1-9`, `0`: Switch to scene 1-10
- `,` (Comma): Previous scene
- `.` (Period): Next scene
- `A`: Toggle audio reactive mode
- `B`: Blackout toggle
- `=` (Plus): Master fade in
- `-` (Minus): Master fade out
- `Mouse Wheel`: Audio intensity control

## Next Steps

1. Create your first VJ scene in MainVJStage
2. Experiment with Niagara particle systems
3. Set up external control via OSC
4. Build a library of reusable effects
5. Create performance-ready packages

## Need Help?

- Check the main README.md for detailed information
- Refer to Unreal Engine documentation
- Explore example content in the Content folder

Happy VJing!
