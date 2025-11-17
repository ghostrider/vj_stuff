# Camera Capture Setup Instructions

This guide explains how to set up USB camera capture for VJ scenes.

## Quick Setup

1. **Add VJCameraCapture Actor to Level**:
   - Open your scene (e.g., MainVJStage)
   - Place Actors panel → All Classes → search "VJCameraCapture"
   - Drag into your level

2. **Configure Camera Settings**:
   - Select the VJCameraCapture actor
   - In Details panel, configure:
     - **Camera Device Index**: 0 for first camera, 1 for second, etc.
     - **Capture Width/Height**: Resolution (e.g., 1920x1080)
     - **Capture Frame Rate**: FPS (e.g., 30)
     - **Auto Start**: Enable to start on play
     - **Flip Horizontal/Vertical**: Mirror camera if needed

3. **Create Display Material**:
   - Content/Materials → Right-click → Material
   - Name it `M_CameraDisplay`
   - Set Material Domain: Surface
   - Set Shading Model: Unlit
   - Add Texture Sample node (will reference MediaTexture)
   - Connect to Emissive Color
   - Save and compile

4. **Create Material Instance**:
   - Right-click on M_CameraDisplay → Create Material Instance
   - Name it `MI_CameraDisplay_Instance`

5. **Apply to Mesh**:
   - Add a Plane or Screen mesh to your scene
   - Apply MI_CameraDisplay_Instance material
   - In the material instance, set the texture parameter to the MediaTexture from VJCameraCapture

## Platform-Specific Camera URLs

### Windows
```
Format: "video=Device Name"
Examples:
- "video=USB Video Device"
- "video=Integrated Webcam"
- "video=0" (first camera by index)
```

### Linux
```
Format: "v4l2:///dev/videoN"
Examples:
- "v4l2:///dev/video0"
- "v4l2:///dev/video1"
```

### macOS
```
Format: "avfoundation://N"
Examples:
- "avfoundation://0"
- "avfoundation://1"
```

## Finding Camera Devices

### Windows
Use Device Manager → Cameras to see available cameras

### Linux
Run: `v4l2-ctl --list-devices` or `ls /dev/video*`

### macOS
System Preferences → Camera

## Blueprint Usage

### Getting the Media Texture in Blueprint

1. Get reference to VJCameraCapture actor
2. Call `GetMediaTexture()` function
3. Use the returned Media Texture in:
   - Material parameters
   - UI widgets
   - Dynamic materials

### Example Blueprint Nodes

```
Get Actor of Class (VJCameraCapture)
  ↓
Get Media Texture
  ↓
Create Dynamic Material Instance
  ↓
Set Texture Parameter Value (TextureParam = "CameraTexture")
  ↓
Set Material (on Static Mesh Component)
```

### Starting/Stopping Capture

```
Get VJCameraCapture reference
  ↓
Start Capture / Stop Capture / Restart Capture
```

### Switching Cameras

```
Get VJCameraCapture reference
  ↓
Set Camera Device (DeviceIndex = 0, 1, 2, etc.)
```

## Troubleshooting

### Camera Not Showing
1. Check Output Log for errors
2. Verify camera is not in use by another app
3. Try different Camera Device Index (0, 1, 2...)
4. Check camera permissions on your OS

### Poor Performance
1. Lower Capture Width/Height (try 1280x720 or 640x480)
2. Reduce Capture Frame Rate (try 24 or 15 fps)
3. Use Material Instances instead of base materials
4. Disable unused post-processing effects

### Camera URL Not Working
1. Check platform-specific URL format above
2. Use Camera Device Index instead of URL
3. Check Output Log for exact error messages

## Advanced: Multiple Cameras

To use multiple cameras simultaneously:

1. Add multiple VJCameraCapture actors to your scene
2. Set different Camera Device Index for each (0, 1, 2, etc.)
3. Each will have its own MediaTexture output
4. Use different materials for each camera feed

## Integration with VJ Scenes

### Audio-Reactive Camera Feed

1. Get reference to VJAudioAnalyzer
2. Read audio properties (AudioAmplitude, frequency bands)
3. Apply to material parameters:
   - Brightness/Intensity
   - Color tint
   - Glitch effects
   - Displacement

### Scene-Specific Camera

Create a camera feed as part of a VJ scene:

1. Create Scene Blueprint (BP_Scene_CameraEffect)
2. Add VJCameraCapture as component
3. Add display mesh with material
4. Add to VJSceneManager scenes array
5. Camera will activate/deactivate with scene

This allows different scenes to use different cameras or effects!
