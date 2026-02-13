# VJ Materials

This directory contains materials for VJ visual effects.

## Camera Feed Materials

### Creating a Camera Feed Material

To display a USB camera feed in your scene:

1. **Create Material**:
   - Right-click in this folder → Create Material
   - Name it `M_CameraFeed` or similar

2. **Configure Material Settings**:
   - Material Domain: Surface
   - Blend Mode: Opaque (or Translucent if you want transparency)
   - Shading Model: Unlit (for direct camera feed) or Default Lit

3. **Add Texture Sample**:
   - In the material editor, add a `Texture Sample` node
   - Set the Texture type to `Media Texture`
   - Connect this to Base Color (for Unlit) or Emissive Color (for Lit)

4. **Connect to Output**:
   - For Unlit: Texture Sample → Emissive Color
   - For Lit with glow: Texture Sample → Emissive Color (multiply by intensity)

5. **Optional Flip Controls**:
   - Add parameters for flipping horizontal/vertical
   - Use `OneMinus` node on U coordinate to flip horizontally
   - Use `OneMinus` node on V coordinate to flip vertically

### Example Material Setup (Unlit Camera Feed)

```
Nodes:
1. Texture Sample (MediaTexture from VJCameraCapture)
   ↓
2. Emissive Color output

For flipped camera:
1. Texture Coordinate
   ↓
2. OneMinus (on U for horizontal flip, V for vertical flip)
   ↓
3. Texture Sample with custom UVs
   ↓
4. Emissive Color output
```

### Example Material Setup (Audio-Reactive Camera Feed)

```
Nodes:
1. Texture Sample (MediaTexture)
   ↓
2. Multiply (with Audio Intensity parameter)
   ↓
3. Emissive Color output

Add Parameter for Audio Intensity (0.0 to 2.0)
```

## Audio-Reactive Materials

Materials that respond to audio can read values from VJAudioAnalyzer:

- Create Material Parameter Collections
- Read audio values via Blueprint
- Update material parameters in real-time

### Common Material Parameters for VJ Effects

- **Intensity** (Scalar): Overall brightness/strength
- **Speed** (Scalar): Animation speed
- **Color** (Vector): Tint color
- **AudioReactivity** (Scalar): How much audio affects the material
- **Time** (Scalar): Custom time for animations

## Post Process Materials

For full-screen effects:

1. Create Material with Domain: Post Process
2. Add Scene Texture nodes to sample screen
3. Apply effects (glitch, color grading, distortion, etc.)
4. Use in Post Process Volume in your scene

## Material Instances

Always create Material Instances for performance:

1. Right-click on base material → Create Material Instance
2. Name it `MI_[BaseName]_[Variant]`
3. Adjust parameters without recompiling shaders
4. Use in scenes for best performance
