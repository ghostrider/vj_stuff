# pi-recorder

Raspberry Pi 5 HDMI video recorder with a Waveshare 2.7" ePaper HAT display.

## Hardware

| Component | Details |
|-----------|---------|
| SBC | Raspberry Pi 5 |
| Display | Waveshare 2.7" ePaper HAT (264×176, B&W) |
| Display interface | SPI0, CE0 (MOSI=BCM10, CS=BCM8) |
| Capture | USB HDMI-to-USB capture card (v4l2) |

## Button Mapping

The HAT has four push buttons on the left side (top → bottom):

| Button | BCM Pin | Action |
|--------|---------|--------|
| KEY1 | 5 | **Start** recording |
| KEY2 | 6 | **Stop** recording and save file |
| KEY3 | 13 | **Pause** / **Resume** recording |
| KEY4 | 19 | **Toggle** stats view |

## Stats Views (KEY4 cycles through these)

| # | View | Shows |
|---|------|-------|
| 0 | Status | Recording state, elapsed time, file size, segment count |
| 1 | System | CPU%, temperature, RAM usage, free disk space |
| 2 | Capture | Device path, resolution, pixel format, audio |
| 3 | Files | Last 4 saved recordings with file sizes |

## Recording Behaviour

- **Start**: Opens a new recording session, begins writing to a `.mkv` (or chosen container) segment file.
- **Pause**: Gracefully stops ffmpeg (sealing the current segment). The session stays open.
- **Resume**: Starts a new ffmpeg process writing to a new segment file in the same session.
- **Stop**: Stops ffmpeg. If the session has multiple segments (from pause/resume cycles), they are concatenated into a single output file using `ffmpeg -f concat`.

Output files are named `YYYY-MM-DD_HH-MM-SS.mkv` and saved to `~/Videos/recordings/` by default.

## Installation

```bash
cd pi-recorder
bash install.sh
```

The script:
1. Installs `ffmpeg`, `v4l-utils`, and fonts via `apt`
2. Enables the SPI interface in `/boot/firmware/config.txt`
3. Creates a Python virtual environment
4. Clones and installs the Waveshare ePaper Python library
5. Installs Python dependencies

> **Note:** If SPI was not previously enabled, a reboot is required after running `install.sh`.

### Manual dependency install

```bash
sudo apt install ffmpeg v4l-utils python3-pip python3-venv libopenjp2-7 fonts-dejavu-core
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
# Waveshare library (from their GitHub repo):
pip install ./waveshare_epd_src/RaspberryPi_JetsonNano/python/
```

## Running

```bash
source venv/bin/activate
python main.py
```

### Options

```
python main.py --help

  --device /dev/videoN      Force a specific capture device (auto-detected by default)
  --output ~/Videos/        Output directory for saved recordings
  --container mkv|mp4|avi   Output container (default: mkv)
  --video-codec copy|...    Video codec (default: copy — no re-encoding)
  --audio-codec aac|...     Audio codec (default: aac)
  --debug                   Verbose logging
```

### Running without Raspberry Pi hardware

On a non-Pi machine (for development), the app falls back gracefully:

- **No gpiozero**: buttons are simulated via keyboard input (`1/2/3/4 + Enter`)
- **No waveshare_epd**: display output is printed to the terminal
- **No /dev/video***: specify a device with `--device` or check `v4l2-ctl --list-devices`

## Project Structure

```
pi-recorder/
├── main.py              Entry point, wires all components together
├── requirements.txt     Python dependencies
├── install.sh           Bootstrap script for Raspberry Pi OS
└── src/
    ├── recorder.py      FFmpeg recording engine (start/stop/pause/resume)
    ├── epaper.py        Waveshare 2.7" ePaper display manager
    ├── buttons.py       GPIO button handler (gpiozero)
    ├── capture.py       USB v4l2 capture device detection
    └── stats.py         System statistics (CPU, RAM, temp, disk)
```

## Enabling SPI manually

If you need to enable SPI without running install.sh:

```bash
sudo raspi-config
# Interfacing Options → SPI → Yes
sudo reboot
```

Or add `dtparam=spi=on` to `/boot/firmware/config.txt` and reboot.
