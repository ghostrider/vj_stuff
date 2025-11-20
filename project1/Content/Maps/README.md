# VJ Maps

This directory contains the main VJ stage maps and scene levels.

## MainVJStage.umap

The main VJ stage level. This level should contain:
- VJSceneManager actor (manages scene switching)
- VJAudioAnalyzer actor (handles audio processing)
- Post Process Volume (for master effects)
- Default lighting setup

To create this map in Unreal Engine:
1. File → New Level → Empty Level
2. Save as `MainVJStage` in this directory
3. Add the VJ system actors from the Place Actors panel

## Scene Organization

Individual scenes can be:
1. Separate levels loaded as streaming levels
2. Actor groups toggled on/off by the Scene Manager
3. Blueprint actors with self-contained visual effects

### Recommended Setup:

Create scene actors in `Content/Blueprints/Scenes/` and add them to the MainVJStage level.
The VJSceneManager can then toggle their visibility and activation.
