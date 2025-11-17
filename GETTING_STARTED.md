# Getting Started with VJ Animations

This guide will help you get up and running with the VJ Animations project.

## Quick Start

1. **Open the Project**
   - Double-click `VJAnimations.uproject`
   - Unreal Engine will open and compile shaders on first launch (this takes time)

2. **Explore the Main Stage**
   - The default map is `MainVJStage` (will be created in Content/Maps/)
   - This is your primary workspace for creating visual content

3. **Create Your First Effect**
   - Navigate to Content/Blueprints
   - Right-click → Blueprint Class → Actor
   - Name it "BP_MyFirstEffect"
   - Add a Particle System component or Niagara System

## Common VJ Workflows

### Real-Time Audio Reactive Visuals

1. Import audio files to Content/Audio
2. Use Audio Spectrum analysis
3. Drive material parameters or particle systems with audio data
4. Test with real-time audio input

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

- `Alt + P`: Play in editor
- `Esc`: Stop playing
- `F11`: Fullscreen
- `Ctrl + Space`: Content Browser
- `` ` ``: Console commands
- `Ctrl + Shift + ,`: GPU Visualizer

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
