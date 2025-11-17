# USB Camera Capture Guide

Complete guide for setting up and using USB camera/video capture in VJ Animations.

## Overview

The VJCameraCapture system allows you to capture live video from:
- Webcams
- USB capture cards (HDMI/SDI to USB)
- Virtual cameras (OBS Virtual Camera, etc.)
- Any Video4Linux2 (V4L2) device on Linux
- AVFoundation devices on macOS
- Windows Media Foundation devices on Windows

## Quick Start

### 1. Basic Setup (5 minutes)

**Step 1: Add Camera Actor**
```
Place Actors panel → All Classes → Search "VJCameraCapture" → Drag to level
```

**Step 2: Configure Camera Settings**
```
Select VJCameraCapture actor in level
Details panel:
  - Camera Device Index: 0 (first camera)
  - Capture Width: 1280
  - Capture Height: 720
  - Capture Frame Rate: 30
  - Auto Start: ✓ (checked)
```

**Step 3: Create Display**
```
Add a Plane mesh to the scene
Scale it appropriately (e.g., X=16, Y=9 for 16:9 aspect ratio)
```

**Step 4: Test**
```
Press Play (Alt+P)
Camera should start automatically
Check Output Log for "Camera capture started successfully"
```

## Finding Your Camera

### Windows

**Method 1: Device Manager**
1. Open Device Manager
2. Expand "Cameras" or "Imaging Devices"
3. Your camera will be listed
4. First camera = Index 0, second = Index 1, etc.

**Method 2: Windows Settings**
1. Settings → Privacy → Camera
2. See which apps have access to which cameras

### Linux

**Method 1: List devices**
```bash
ls -la /dev/video*
```
Output: `/dev/video0`, `/dev/video1`, etc.

**Method 2: V4L2 tools**
```bash
v4l2-ctl --list-devices
```

**Method 3: FFmpeg**
```bash
ffmpeg -f v4l2 -list_devices true -i dummy
```

### macOS

**Method 1: System Preferences**
1. System Preferences → Security & Privacy → Camera
2. See available cameras

**Method 2: Terminal**
```bash
ffmpeg -f avfoundation -list_devices true -i ""
```

## Detailed Configuration

### Camera URLs by Platform

The VJCameraCapture class automatically builds the correct URL for your platform.

**Windows:**
```
Automatic: Uses device index
Manual: Set CameraDeviceURL to "video=Device Name"
Example: "video=Logitech Webcam"
```

**Linux:**
```
Automatic: v4l2:///dev/video{index}
Manual: Set CameraDeviceURL to specific device
Example: "v4l2:///dev/video0"
```

**macOS:**
```
Automatic: avfoundation://{index}
Manual: Set CameraDeviceURL to specific device
Example: "avfoundation://0"
```

### Resolution and Performance

**Recommended Settings by Use Case:**

**High Quality (for projection):**
- Resolution: 1920x1080
- Frame Rate: 30 fps
- Use case: Main visual, high-res displays
- Performance impact: High

**Balanced (recommended):**
- Resolution: 1280x720
- Frame Rate: 30 fps
- Use case: Most VJ scenarios
- Performance impact: Medium

**Performance Mode:**
- Resolution: 640x480 or 854x480
- Frame Rate: 24 fps
- Use case: Multiple cameras, lower-end hardware
- Performance impact: Low

**Creative Low-Res:**
- Resolution: 320x240 or 160x120
- Frame Rate: 15-24 fps
- Use case: Pixelated/retro aesthetic
- Performance impact: Very low

### Flip Options

- **Flip Horizontal**: Mirrors the camera (useful for selfie cameras)
- **Flip Vertical**: Flips upside down (rarely needed)

Set these in the VJCameraCapture actor's Details panel.

## Creating Materials for Camera Display

### Basic Unlit Material

**Material: M_CameraFeed_Basic**

1. Create new Material in Content/Materials/
2. Set Shading Model: Unlit
3. Add nodes:
   ```
   Texture Object Parameter "CameraTexture" → Texture Sample → Emissive Color
   ```
4. Save and compile

**Usage in Blueprint:**
```
Event BeginPlay:
  Get VJCameraCapture → Get Media Texture
  Create Dynamic Material Instance (M_CameraFeed_Basic)
  Set Texture Parameter Value ("CameraTexture", MediaTexture)
  Set Material (on Static Mesh)
```

### Audio-Reactive Material

**Material: M_CameraFeed_AudioReactive**

Add to M_CameraFeed_Basic:
```
Texture Sample → Multiply (with Scalar Parameter "Intensity") → Emissive Color
```

**Update from Blueprint:**
```
Event Tick:
  Get VJAudioAnalyzer → Get AudioIntensity
  Set Scalar Parameter Value ("Intensity", AudioIntensity * 2.0)
```

This makes the camera feed pulse with music!

## Blueprint Integration

### Basic Camera Scene Blueprint

**BP_Scene_BasicCamera**

**Components:**
- VJCameraCapture (Camera Capture Component)
- StaticMesh (Display Mesh)

**Event Graph:**
```cpp
Event BeginPlay:
  1. Load Material Asset
  2. Create Dynamic Material Instance
  3. Get MediaTexture from CameraCapture component
  4. Set Texture Parameter on Material Instance
  5. Apply Material to StaticMesh

Event EndPlay:
  1. Get CameraCapture component
  2. Stop Capture
```

### Advanced: Camera with Effects

**BP_Scene_CameraGlitch**

Extends basic camera scene with:
- Glitch effects on beat
- Color cycling with audio
- UV distortion
- Chromatic aberration

See: `Content/Blueprints/Scenes/ExampleCameraScene.md`

## Multiple Cameras

### Setup

1. Add multiple VJCameraCapture actors:
   - CameraCapture1: Device Index 0
   - CameraCapture2: Device Index 1
   - CameraCapture3: Device Index 2

2. Each gets its own MediaTexture output

3. Create separate materials or use one material with switchable textures

### Example: Dual Camera Scene

```
Scene Layout:
[Camera 1 Feed]  [Camera 2 Feed]
     Left             Right
```

**Implementation:**
- Two VJCameraCapture actors
- Two plane meshes with different materials
- Each material references different MediaTexture

## Performance Optimization

### Best Practices

1. **Lower Resolution First**:
   - Try 1280x720 before 1920x1080
   - 640x480 is great for retro/pixelated effects

2. **Reduce Frame Rate**:
   - 24 fps is often sufficient for VJ visuals
   - 30 fps for smoother motion
   - Avoid 60 fps unless necessary

3. **Material Optimization**:
   - Use Unlit shading when possible
   - Minimize texture samples
   - Avoid complex math in materials

4. **Stop Inactive Cameras**:
   - When scene switches, call StopCapture()
   - Restart when scene becomes active
   - Use VJSceneManager's OnSceneChanged event

5. **Use Material Instances**:
   - Create Material Instance instead of Dynamic Material when possible
   - Only use Dynamic Material if you need runtime parameter changes

### Monitoring Performance

**Console Commands:**
```
stat fps          - Show frame rate
stat unit         - Show frame time breakdown
stat media        - Show media playback stats
profilegpu        - Detailed GPU profiling
```

**Target Performance:**
- 60 FPS: Ideal
- 45+ FPS: Acceptable
- Below 30 FPS: Reduce resolution/effects

## Troubleshooting

### Camera Not Showing

**Check 1: Output Log**
```
Look for: "Camera capture started successfully"
Or errors like: "Failed to open camera URL"
```

**Check 2: Device Index**
```
Try different indices: 0, 1, 2
Some systems have virtual cameras that use indices
```

**Check 3: Camera In Use**
```
Close other apps using the camera (Zoom, OBS, etc.)
```

**Check 4: Permissions**
```
Windows: Settings → Privacy → Camera
macOS: System Preferences → Security → Camera
Linux: User must be in 'video' group
```

### Black Screen / No Video

**Possible Causes:**

1. **Material not set up correctly**:
   - Verify Texture Parameter is set
   - Check material is Unlit or has emissive
   - Confirm MediaTexture is assigned

2. **Camera not opened**:
   - Check bIsCapturing is true
   - Verify Auto Start is enabled
   - Try calling StartCapture() manually

3. **Wrong URL/Index**:
   - Try different Camera Device Index
   - Check platform-specific URL format

### Poor Quality / Stuttering

**Solutions:**

1. **Reduce resolution**: 1280x720 or lower
2. **Lower frame rate**: 24 or 30 fps
3. **Check CPU/GPU usage**: May be bottlenecked
4. **Close other applications**: Free up system resources
5. **Update drivers**: Especially GPU drivers

### Linux-Specific Issues

**Permission Denied:**
```bash
# Add user to video group
sudo usermod -a -G video $USER
# Logout and login again
```

**Device Busy:**
```bash
# Check what's using the camera
sudo lsof /dev/video0

# Kill the process if needed
sudo killall <process-name>
```

### Windows-Specific Issues

**WMF Codec Not Found:**
- Ensure "WmfMedia" plugin is enabled in .uproject
- Check Windows Media Feature Pack is installed
- Try Windows Update

### macOS-Specific Issues

**Camera Access Denied:**
- System Preferences → Security & Privacy → Camera
- Enable for "Unreal Engine" or your app

## Advanced Topics

### Virtual Cameras

Capture from software like OBS:

**OBS Virtual Camera:**
1. Install OBS with Virtual Camera
2. Start Virtual Camera in OBS
3. In VJCameraCapture, it appears as another device
4. Set Camera Device Index to virtual camera

**Use Cases:**
- Mix pre-recorded video with live camera
- Apply OBS filters before Unreal
- Multiple sources composited in OBS

### HDMI/SDI Capture Cards

**Supported Devices:**
- Elgato Capture Cards
- Blackmagic Design cards
- AVerMedia cards
- Any UVC-compatible capture device

**Setup:**
1. Install manufacturer drivers
2. Connect HDMI/SDI source
3. Device appears as camera in system
4. Use in VJCameraCapture like any camera

**Tips:**
- Check capture card resolution limits
- Some cards need specific drivers
- Test with VLC or OBS first

### Custom Camera URLs

**Windows Advanced:**
```cpp
CameraDeviceURL = "video=Logitech HD Pro Webcam C920:audio=Microphone (C920)"
```

**Linux V4L2 Options:**
```cpp
CameraDeviceURL = "v4l2:///dev/video0?width=1280&height=720&framerate=30"
```

**macOS AVFoundation:**
```cpp
CameraDeviceURL = "avfoundation://0?framerate=30"
```

### Recording Camera Output

To save camera feed to disk:

1. Use Media Capture (separate from VJCameraCapture)
2. Set up Render Target
3. Render camera material to Render Target
4. Export Render Target to video file
5. (Advanced topic - see Unreal Media documentation)

## Example Workflows

### Workflow 1: Simple Webcam Display

1. Add VJCameraCapture (Device Index 0)
2. Add Plane mesh
3. Create M_CameraFeed_Basic material
4. Apply to plane
5. Play - camera appears

**Time: 2 minutes**

### Workflow 2: Audio-Reactive Camera

1. Use Workflow 1 as base
2. Modify material to accept Intensity parameter
3. In Blueprint, drive Intensity from VJAudioAnalyzer
4. Camera now pulses with music

**Time: 5 minutes**

### Workflow 3: Multi-Camera Mixer

1. Add 3 VJCameraCapture actors (Index 0, 1, 2)
2. Create 3 display planes
3. Position in grid or creative layout
4. Each shows different camera
5. Use with VJSceneManager for live switching

**Time: 10 minutes**

### Workflow 4: Camera + Glitch Effects

1. Use Workflow 2 as base
2. Create M_CameraGlitch material
3. Add UV distortion, color separation
4. Subscribe to OnBeatDetected
5. Trigger glitch on beats

**Time: 15 minutes**

## Resources

- **Setup Guide**: `Content/Blueprints/CameraSetup_Instructions.md`
- **Example Scenes**: `Content/Blueprints/Scenes/ExampleCameraScene.md`
- **Material Examples**: `Content/Materials/CameraMaterial_Example.md`
- **Unreal Media Docs**: https://docs.unrealengine.com/5.4/en-US/media-framework-in-unreal-engine/

## Summary

The VJCameraCapture system provides:
- ✅ Easy USB camera integration
- ✅ Cross-platform support (Windows, Linux, macOS)
- ✅ Configurable resolution and frame rate
- ✅ Multiple simultaneous cameras
- ✅ Integration with VJ scene management
- ✅ Audio-reactive capabilities
- ✅ Blueprint-friendly API

Start with the basic setup, then experiment with audio-reactive effects and creative materials!
