# Example Camera Scene Blueprint

This guide walks through creating a complete camera-based VJ scene.

## BP_Scene_Camera - Basic Camera Feed Scene

### Creating the Blueprint

1. **Create Blueprint**:
   - Right-click in Content/Blueprints/Scenes/
   - Blueprint Class → Actor
   - Name: `BP_Scene_Camera`

2. **Add Components**:

   **Root Component:**
   - Scene Component (root)

   **Camera Capture:**
   - Add Child Component → Search "VJCameraCapture"
   - Name: "CameraCapture"

   **Display Mesh:**
   - Add Child Component → Plane (or Static Mesh)
   - Name: "DisplayPlane"
   - Transform: Scale to desired size (e.g., X=20, Y=11.25 for 16:9 ratio)

3. **Configure Camera Capture Component**:
   - Camera Device Index: 0
   - Capture Width: 1920
   - Capture Height: 1080
   - Capture Frame Rate: 30
   - Auto Start: True

4. **Create Material for Display**:

   In Content/Materials/:
   - Create Material: `M_CameraFeed`
   - Material Domain: Surface
   - Shading Model: Unlit
   - Add Texture Sample Parameter: "CameraTexture"
   - Connect to Emissive Color
   - Compile and save

5. **Blueprint Event Graph**:

   **On Begin Play:**
   ```
   Event BeginPlay
   ↓
   Get Component (CameraCapture)
   ↓
   Get Media Texture
   ↓
   Create Dynamic Material Instance (M_CameraFeed)
   ↓
   Set Texture Parameter Value (Name: "CameraTexture", Value: MediaTexture)
   ↓
   Set Material (DisplayPlane, MaterialInstance)
   ```

   **Optional - On End Play:**
   ```
   Event EndPlay
   ↓
   Get Component (CameraCapture)
   ↓
   Stop Capture
   ```

## BP_Scene_CameraAudioReactive - Audio-Reactive Camera

### Extending the Basic Scene

1. **Start with BP_Scene_Camera** (duplicate it)
2. **Rename to** `BP_Scene_CameraAudioReactive`

3. **Add Variables**:
   - `AudioAnalyzer` (VJAudioAnalyzer reference)
   - `BaseIntensity` (Float, default: 1.0)
   - `AudioReactivity` (Float, default: 0.5)

4. **Enhanced Event Graph**:

   **On Begin Play:**
   ```
   Event BeginPlay
   ↓
   Get All Actors of Class (VJAudioAnalyzer)
   ↓
   Get (index 0)
   ↓
   Set AudioAnalyzer variable
   ↓
   [Continue with material setup from basic scene]
   ↓
   Set Scalar Parameter Value on Material (Name: "Intensity", Value: 1.0)
   ```

   **On Tick:**
   ```
   Event Tick
   ↓
   Branch: Is AudioAnalyzer Valid?
     ↓ True
   Get AudioIntensity
   ↓
   Multiply (AudioIntensity * AudioReactivity)
   ↓
   Add (BaseIntensity + result)
   ↓
   Set Scalar Parameter Value (Name: "Intensity", Value: result)
   ```

5. **Update Material**:

   Modify M_CameraFeed:
   - Add Scalar Parameter: "Intensity" (default: 1.0)
   - Multiply texture sample by Intensity
   - Connect to Emissive Color
   - This makes the camera feed pulse with audio

## BP_Scene_CameraGlitch - Glitchy Camera Effect

### Adding Glitch Effects

1. **Duplicate BP_Scene_CameraAudioReactive**
2. **Rename to** `BP_Scene_CameraGlitch`

3. **Create Glitch Material** (`M_CameraGlitch`):

   Material nodes:
   - Texture Sample (CameraTexture)
   - Add UV distortion based on audio
   - Add color separation on beats
   - Add scanlines
   - Optional: Digital noise overlay

4. **Blueprint - On Beat Response**:
   ```
   Subscribe to: AudioAnalyzer → OnBeatDetected
   ↓
   When beat fires:
   ↓
   Set Scalar Parameter (Name: "GlitchStrength", Value: BeatStrength)
   ↓
   Delay 0.1 seconds
   ↓
   Set Scalar Parameter (Name: "GlitchStrength", Value: 0.0)
   ```

## BP_Scene_DualCamera - Two Cameras Side by Side

### Multiple Camera Feeds

1. **Create new Blueprint**: `BP_Scene_DualCamera`

2. **Add Components**:
   - CameraCapture1 (VJCameraCapture, Device Index: 0)
   - CameraCapture2 (VJCameraCapture, Device Index: 1)
   - DisplayPlane1 (Plane, position left)
   - DisplayPlane2 (Plane, position right)

3. **Event Graph**:
   ```
   Event BeginPlay
   ↓
   Create materials for both displays
   ↓
   Get MediaTexture from CameraCapture1 → Set to DisplayPlane1
   ↓
   Get MediaTexture from CameraCapture2 → Set to DisplayPlane2
   ```

This creates a split-screen effect with two different cameras!

## Integration with Scene Manager

After creating your camera scene blueprint:

1. Place the scene actor in MainVJStage level
2. Select VJSceneManager actor
3. Add your scene to the Scenes array
4. Assign it to a number key (e.g., Scene 5)
5. During performance, press 5 to show camera scene

## Performance Tips

- Use lower resolutions for better performance (1280x720 instead of 1920x1080)
- Set frame rate to 24 or 30 instead of 60
- Use Material Instances instead of dynamic materials when possible
- Disable camera capture when scene is not active (implement in OnSceneChanged event)

## Creative Ideas

- **Kaleidoscope Camera**: Mirror and repeat camera feed
- **Trails Effect**: Blend current frame with previous frames
- **Color Cycling**: Shift hue over time with audio
- **Pixelation**: Reduce resolution based on audio
- **Edge Detection**: Show only edges, beat-reactive thickness
- **Feedback Loop**: Feed output back to input with delay
