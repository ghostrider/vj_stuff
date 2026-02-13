# VJ Keyboard Shortcuts Reference

Quick reference guide for VJ performance keyboard shortcuts.

## Scene Control

| Key | Action | Description |
|-----|--------|-------------|
| `1` | Switch to Scene 1 | Instantly switch to scene 1 |
| `2` | Switch to Scene 2 | Instantly switch to scene 2 |
| `3` | Switch to Scene 3 | Instantly switch to scene 3 |
| `4` | Switch to Scene 4 | Instantly switch to scene 4 |
| `5` | Switch to Scene 5 | Instantly switch to scene 5 |
| `6` | Switch to Scene 6 | Instantly switch to scene 6 |
| `7` | Switch to Scene 7 | Instantly switch to scene 7 |
| `8` | Switch to Scene 8 | Instantly switch to scene 8 |
| `9` | Switch to Scene 9 | Instantly switch to scene 9 |
| `0` | Switch to Scene 10 | Instantly switch to scene 10 |
| `,` | Previous Scene | Cycle to previous scene |
| `.` | Next Scene | Cycle to next scene |

## Audio Control

| Key | Action | Description |
|-----|--------|-------------|
| `A` | Toggle Audio Reactive | Turn audio reactive mode on/off |
| `Mouse Wheel` | Audio Intensity | Adjust audio intensity parameter |

## Master Controls

| Key | Action | Description |
|-----|--------|-------------|
| `B` | Blackout Toggle | Instantly hide/show all scenes |
| `=` or `+` | Master Fade In | Increase master fade level |
| `-` | Master Fade Out | Decrease master fade level |

## Tips for Live Performance

1. **Prepare Scene Order**: Arrange scenes 1-10 in the order you'll perform them
2. **Use Sequential Keys**: For smooth transitions, use adjacent number keys
3. **Practice Transitions**: Rehearse scene switches before live performance
4. **Blackout for Safety**: Use `B` to instantly cut all visuals if needed
5. **Audio Reactive Toggle**: Use `A` to sync or un-sync visuals from audio
6. **Master Fade**: Use `=` and `-` for gradual intensity changes

## Customizing Shortcuts

To customize these shortcuts:
1. Open `Config/DefaultInput.ini`
2. Modify the `+ActionMappings` entries
3. Restart Unreal Engine for changes to take effect

Example:
```ini
+ActionMappings=(ActionName="SwitchToScene1",bShift=False,bCtrl=False,bAlt=False,bCmd=False,Key=One)
```

Change `Key=One` to any other key you prefer.

## MIDI/OSC Control

For external hardware control (MIDI controllers, OSC devices):
- Configure OSC receivers in the VJPlayerController Blueprint
- Map MIDI/OSC messages to scene switching functions
- Use the provided C++ classes as a foundation for custom control schemes
