# Camera Material Examples

Step-by-step guides for creating camera feed materials in Unreal Engine.

## M_CameraFeed_Basic - Simple Camera Display

### Material Setup

**Settings:**
- Material Domain: Surface
- Blend Mode: Opaque
- Shading Model: Unlit

**Node Graph:**
```
[Texture Object Parameter: CameraTexture] → Texture Sample
                                              ↓
                                         Emissive Color
```

**Parameters:**
- `CameraTexture` - Texture Object Parameter (Media Texture)

**Usage:**
1. Create this material in Content/Materials/
2. Create Material Instance: `MI_CameraFeed_Basic`
3. In Blueprint, set CameraTexture parameter to VJCameraCapture's MediaTexture
4. Apply to mesh

---

## M_CameraFeed_Flipped - Mirrored Camera

### Material Setup

**Settings:**
- Material Domain: Surface
- Shading Model: Unlit

**Node Graph:**
```
[Texture Coordinate] → [Component Mask: R] → [One Minus] ─┐
                                                           ├→ [Append] → [Custom UVs]
[Texture Coordinate] → [Component Mask: G] ───────────────┘              ↓
                                                         [Texture Object Parameter] → Texture Sample
                                                                                      ↓
                                                                                 Emissive Color
```

**Parameters:**
- `CameraTexture` - Texture Object Parameter
- `FlipHorizontal` - Static Bool (default: True)
- `FlipVertical` - Static Bool (default: False)

**Improved Node Graph with Parameters:**
```
[Texture Coordinate]
  ↓
[Component Mask: R] → [Branch: FlipHorizontal] → [One Minus (if true)] ─┐
                                                                         ├→ [Append]
[Texture Coordinate]                                                     │     ↓
  ↓                                                                      │  [Texture Sample]
[Component Mask: G] → [Branch: FlipVertical] → [One Minus (if true)] ──┘     ↓
                                                                         Emissive Color
```

---

## M_CameraFeed_AudioReactive - Intensity Pulse

### Material Setup

**Settings:**
- Material Domain: Surface
- Shading Model: Unlit

**Node Graph:**
```
[Texture Object Parameter: CameraTexture] → Texture Sample
                                              ↓
                                         [Multiply]
                                              ↑
                                    [Scalar Parameter: Intensity]
                                              ↓
                                         Emissive Color
```

**Parameters:**
- `CameraTexture` - Texture Object Parameter
- `Intensity` - Scalar Parameter (default: 1.0, range: 0.0 to 3.0)

**Blueprint Integration:**
```
Event Tick
  ↓
Get VJAudioAnalyzer
  ↓
Get AudioIntensity
  ↓
Lerp (from: 0.5, to: 1.5, alpha: AudioIntensity)
  ↓
Set Scalar Parameter Value (MaterialInstance, "Intensity", value)
```

---

## M_CameraFeed_ColorShift - Audio-Driven Color

### Material Setup

**Node Graph:**
```
[Texture Sample: Camera] → [Component Mask: RGB]
                              ↓
                         [Hue Shift]
                              ↑
                    [Scalar Parameter: HueShift]
                              ↓
                         Emissive Color
```

**Additional Nodes for Hue Shift:**
```
RGB Input
  ↓
[RGB to HSV]
  ↓
[Add: Hue + HueShift parameter]
  ↓
[Frac] (wrap to 0-1)
  ↓
[HSV to RGB]
  ↓
Output
```

**Parameters:**
- `CameraTexture` - Texture Object Parameter
- `HueShift` - Scalar Parameter (default: 0.0, range: 0.0 to 1.0)

**Blueprint Integration:**
```
Event Tick
  ↓
Get VJAudioAnalyzer
  ↓
Get LowFrequencyEnergy
  ↓
Multiply by Time (slowly rotating hue)
  ↓
Set Scalar Parameter (HueShift)
```

---

## M_CameraFeed_Glitch - Beat-Reactive Glitch

### Material Setup

**Node Graph:**
```
[Texture Coordinate]
  ↓
[Add] ← [Noise Texture * GlitchStrength]
  ↓
[Texture Sample: Camera]
  ↓
[Channel Separation on beats] ← [Add RGB offset based on GlitchStrength]
  ↓
Emissive Color
```

**Detailed Glitch Effect:**
```
1. UV Distortion:
   [Texture Coordinate] → [Add: Noise * GlitchStrength] → [Custom UVs]

2. Color Separation:
   [Texture Sample: R channel with offset] ─┐
   [Texture Sample: G channel normal]       ├→ [Append RGB]
   [Texture Sample: B channel with offset] ─┘       ↓
                                              Emissive Color

3. Scanlines (optional):
   [Texture Coordinate V] → [Multiply: 100] → [Frac] → [Step: 0.5]
   Multiply with final color for scanline effect
```

**Parameters:**
- `CameraTexture` - Texture Object Parameter
- `GlitchStrength` - Scalar Parameter (default: 0.0, range: 0.0 to 1.0)
- `ChromaticAberration` - Scalar Parameter (default: 0.01)
- `NoiseScale` - Scalar Parameter (default: 10.0)

**Blueprint Integration:**
```
AudioAnalyzer → OnBeatDetected event
  ↓
Set Scalar Parameter (GlitchStrength = BeatStrength)
  ↓
Timeline (0.0 to 1.0 over 0.2 seconds, ease out)
  ↓
Lerp GlitchStrength (from BeatStrength to 0.0)
  ↓
Set Scalar Parameter continuously
```

---

## M_CameraFeed_Pixelated - Retro Effect

### Material Setup

**Node Graph:**
```
[Texture Coordinate]
  ↓
[Multiply: PixelCount] → [Floor] → [Divide: PixelCount]
  ↓
[Texture Sample: Camera]
  ↓
Emissive Color
```

**Parameters:**
- `CameraTexture` - Texture Object Parameter
- `PixelationAmount` - Scalar Parameter (default: 100.0, range: 10.0 to 500.0)

**Audio Reactive Version:**
```
Blueprint drives PixelationAmount:
- High audio = low pixelation (100-200)
- Low audio = high pixelation (10-50)
```

---

## M_CameraFeed_EdgeDetection - Outline Effect

### Material Setup

**Node Graph:**
```
[Texture Sample: Camera at UV]
[Texture Sample: Camera at UV + offset right]
[Texture Sample: Camera at UV + offset down]
  ↓
[Sobel Filter / Edge Detection]
  ↓
[Multiply: EdgeIntensity parameter]
  ↓
Emissive Color
```

**Simplified Edge Detection:**
```
Center = Sample at UV
Right = Sample at UV + (0.01, 0)
Down = Sample at UV + (0, 0.01)

EdgeX = abs(Center - Right)
EdgeY = abs(Center - Down)
Edge = EdgeX + EdgeY

Output = Edge * EdgeIntensity
```

**Parameters:**
- `CameraTexture` - Texture Object Parameter
- `EdgeIntensity` - Scalar Parameter (default: 1.0)
- `EdgeThickness` - Scalar Parameter (default: 1.0)

---

## Post Process: PP_CameraEffects - Full Screen

### Post Process Material Setup

**Settings:**
- Material Domain: Post Process
- Blend Mode: Opaque

**Node Graph:**
```
[Scene Texture: PostProcessInput0]
  ↓
[Apply effects: vignette, color grading, etc.]
  ↓
Emissive Color
```

**Usage:**
1. Create Post Process Volume in level
2. Set this material in Settings → Rendering Features → Post Process Materials
3. Adjust Blend Weight for intensity

---

## Applying Materials in Blueprints

### Dynamic Material Instance Creation

```cpp
Event BeginPlay
  ↓
Load Material: M_CameraFeed_AudioReactive
  ↓
Create Dynamic Material Instance
  ↓
Store in variable: DynamicMaterial
  ↓
Get VJCameraCapture → Get Media Texture
  ↓
Set Texture Parameter Value (DynamicMaterial, "CameraTexture", MediaTexture)
  ↓
Set Material (StaticMeshComponent, DynamicMaterial)

Event Tick
  ↓
Update parameters (Intensity, HueShift, etc.) based on audio
```

---

## Performance Optimization

1. **Use Material Instances** instead of dynamic materials when possible
2. **Reduce texture samples** - each sample is expensive
3. **Use simpler math** - avoid complex calculations in materials
4. **Lower camera resolution** for better performance
5. **Disable unused features** - don't add glitch if you don't need it

## Testing Materials

1. Create a simple test level with:
   - VJCameraCapture actor
   - Plane mesh with your material
   - VJAudioAnalyzer for reactive effects

2. Press Play and verify:
   - Camera feed displays correctly
   - Audio reactivity works (if applicable)
   - Performance is acceptable (check `stat fps`)

3. Iterate on parameters until you get the desired look!
