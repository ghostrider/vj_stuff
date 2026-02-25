"""
USB HDMI capture device detection and management.

Detects v4l2 devices, queries supported formats/resolutions,
and builds the ffmpeg input arguments for recording.
"""

import subprocess
import re
import os
import logging

logger = logging.getLogger(__name__)

# Preferred capture formats in priority order
PREFERRED_FORMATS = ["mjpeg", "yuyv422", "nv12", "h264"]

# Preferred resolutions in priority order
PREFERRED_RESOLUTIONS = [
    (1920, 1080),
    (1280, 720),
    (720, 576),
    (720, 480),
    (640, 480),
]

# Preferred framerates
PREFERRED_FRAMERATES = [60, 30, 25]


class CaptureDevice:
    def __init__(self, device_path: str = None):
        self.device_path = device_path
        self.pixel_format: str = None
        self.width: int = None
        self.height: int = None
        self.framerate: int = None
        self.has_audio: bool = False
        self.audio_device: str = None
        self._detected = False

    def detect(self) -> bool:
        """Auto-detect the first available USB capture device."""
        if self.device_path and os.path.exists(self.device_path):
            logger.info(f"Using specified device: {self.device_path}")
            return self._probe(self.device_path)

        # Find all v4l2 video devices
        devices = self._list_video_devices()
        if not devices:
            logger.warning("No v4l2 video devices found")
            return False

        # Try each device; prefer ones whose card name suggests HDMI capture
        hdmi_keywords = ["hdmi", "capture", "usb", "uvc"]
        preferred = []
        fallback = []
        for dev, name in devices:
            name_lower = name.lower()
            if any(kw in name_lower for kw in hdmi_keywords):
                preferred.append((dev, name))
            else:
                fallback.append((dev, name))

        for dev, name in (preferred + fallback):
            logger.info(f"Trying device: {dev} ({name})")
            if self._probe(dev):
                self.device_path = dev
                return True

        return False

    def _list_video_devices(self) -> list[tuple[str, str]]:
        """Return list of (device_path, card_name) for /dev/video* devices."""
        devices = []
        try:
            result = subprocess.run(
                ["v4l2-ctl", "--list-devices"],
                capture_output=True, text=True, timeout=5
            )
            # Parse output: card name on one line, device paths on subsequent indented lines
            current_name = ""
            for line in result.stdout.splitlines():
                if line and not line.startswith("\t"):
                    current_name = line.rstrip(":")
                elif line.startswith("\t") and "/dev/video" in line:
                    dev = line.strip()
                    devices.append((dev, current_name))
        except (FileNotFoundError, subprocess.TimeoutExpired):
            # Fall back to scanning /dev/video*
            import glob
            for dev in sorted(glob.glob("/dev/video*")):
                devices.append((dev, os.path.basename(dev)))
        return devices

    def _has_capture_capability(self, device_path: str) -> bool:
        """
        Return True only if the device supports V4L2_CAP_VIDEO_CAPTURE.

        Checks sysfs first (no external tool needed), then falls back to
        v4l2-ctl --info. Metadata-only nodes expose bit 0x00400000
        (V4L2_CAP_META_CAPTURE) but NOT bit 0x00000001 (V4L2_CAP_VIDEO_CAPTURE).
        """
        dev_name = os.path.basename(device_path)

        # Primary: sysfs device capabilities (available since Linux 3.7)
        sysfs_caps = f"/sys/class/video4linux/{dev_name}/capabilities"
        if os.path.exists(sysfs_caps):
            try:
                caps = int(open(sysfs_caps).read().strip(), 16)
                has_cap = bool(caps & 0x00000001)  # V4L2_CAP_VIDEO_CAPTURE
                logger.debug("%s sysfs caps=0x%08x video_capture=%s", dev_name, caps, has_cap)
                return has_cap
            except (ValueError, OSError):
                pass

        # Fallback: v4l2-ctl --info — look for "Video Capture" in Device Caps section
        try:
            result = subprocess.run(
                ["v4l2-ctl", "-d", device_path, "--info"],
                capture_output=True, text=True, timeout=5
            )
            in_device_caps = False
            for line in result.stdout.splitlines():
                if "Device Caps" in line or "Device Capabilities" in line:
                    in_device_caps = True
                # "Video Capture" must appear under Device Caps, not "Metadata Capture"
                if in_device_caps and "Video Capture" in line and "Metadata" not in line:
                    return True
            # v4l2-ctl ran but found no Video Capture capability
            return False
        except FileNotFoundError:
            # v4l2-ctl not installed — can't verify, let the probe attempt continue
            return True
        except subprocess.TimeoutExpired:
            return False

    def _probe(self, device_path: str) -> bool:
        """Probe a device for supported formats and pick the best one."""
        if not self._has_capture_capability(device_path):
            logger.debug("Skipping %s — no V4L2_CAP_VIDEO_CAPTURE", device_path)
            return False

        formats = self._query_formats(device_path)
        if not formats:
            return False

        # Pick best pixel format
        pixel_format = None
        for pref in PREFERRED_FORMATS:
            if pref in formats:
                pixel_format = pref
                break
        if not pixel_format:
            pixel_format = next(iter(formats))

        # Pick best resolution
        resolutions = formats.get(pixel_format, {})
        width, height = None, None
        for pw, ph in PREFERRED_RESOLUTIONS:
            if (pw, ph) in resolutions:
                width, height = pw, ph
                break
        if width is None and resolutions:
            width, height = next(iter(resolutions))

        if width is None:
            return False

        # Pick best framerate
        available_fps = resolutions.get((width, height), [])
        framerate = None
        for pref_fps in PREFERRED_FRAMERATES:
            if pref_fps in available_fps:
                framerate = pref_fps
                break
        if framerate is None:
            framerate = available_fps[0] if available_fps else 30

        self.pixel_format = pixel_format
        self.width = width
        self.height = height
        self.framerate = framerate
        self._detected = True

        # Detect audio device (ALSA card matching the USB device)
        self.audio_device = self._find_audio_device()
        self.has_audio = self.audio_device is not None

        logger.info(
            f"Detected: {device_path} {pixel_format} {width}x{height}@{framerate}fps "
            f"audio={'yes: ' + str(self.audio_device) if self.has_audio else 'no'}"
        )
        return True

    def _query_formats(self, device_path: str) -> dict:
        """
        Query supported formats via v4l2-ctl.
        Returns dict: { pixel_format: { (w,h): [fps, ...], ... }, ... }
        """
        formats: dict = {}
        try:
            result = subprocess.run(
                ["v4l2-ctl", "-d", device_path, "--list-formats-ext"],
                capture_output=True, text=True, timeout=5
            )
            current_fmt = None
            current_res = None
            for line in result.stdout.splitlines():
                # Format line: [0]: 'MJPG' (Motion-JPEG, compressed)
                fmt_match = re.search(r"'(\w+)'", line)
                if "Type:" in line or ("compressed" in line or "YUYV" in line
                                       or "NV12" in line or "H264" in line
                                       or "MJPG" in line or "YUYV" in line):
                    if fmt_match:
                        current_fmt = fmt_match.group(1).lower()
                        if current_fmt == "mjpg":
                            current_fmt = "mjpeg"
                        formats.setdefault(current_fmt, {})

                # Resolution line: Size: Discrete 1920x1080
                res_match = re.search(r"(\d+)x(\d+)", line)
                if "Size:" in line and res_match and current_fmt:
                    current_res = (int(res_match.group(1)), int(res_match.group(2)))
                    formats[current_fmt].setdefault(current_res, [])

                # FPS line: Interval: Discrete 0.033s (30.000 fps)
                fps_match = re.search(r"\((\d+\.\d+) fps\)", line)
                if "Interval:" in line and fps_match and current_fmt and current_res:
                    fps = int(round(float(fps_match.group(1))))
                    if fps not in formats[current_fmt][current_res]:
                        formats[current_fmt][current_res].append(fps)

        except FileNotFoundError:
            # v4l2-ctl not installed: use a generic fallback so recording can
            # still be attempted on a device that passed the capability check.
            logger.warning("v4l2-ctl not found — assuming MJPEG 1280x720@30fps")
            formats = {"mjpeg": {(1280, 720): [30]}}
        except subprocess.TimeoutExpired:
            logger.warning("v4l2-ctl timed out probing %s — skipping", device_path)
            formats = {}

        # Strip any format entries with no resolutions (e.g. metadata-type entries
        # that slipped through the parser, like 'uvch' from metadata capture nodes).
        formats = {
            fmt: res_map
            for fmt, res_map in formats.items()
            if res_map and fmt in PREFERRED_FORMATS
        }

        return formats

    def _find_audio_device(self) -> str | None:
        """Find ALSA audio device for USB capture card."""
        try:
            result = subprocess.run(
                ["arecord", "-l"],
                capture_output=True, text=True, timeout=5
            )
            for line in result.stdout.splitlines():
                lower = line.lower()
                if any(kw in lower for kw in ["hdmi", "capture", "usb audio"]):
                    # card N: ..., device M
                    m = re.search(r"card\s+(\d+).*device\s+(\d+)", line, re.IGNORECASE)
                    if m:
                        return f"hw:{m.group(1)},{m.group(2)}"
        except (FileNotFoundError, subprocess.TimeoutExpired):
            pass
        return None

    def build_ffmpeg_input_args(self) -> list[str]:
        """Return the ffmpeg input arguments list for this capture device."""
        if not self._detected:
            raise RuntimeError("Device not probed — call detect() first")

        args = [
            "-f", "v4l2",
            "-thread_queue_size", "1024",
            "-input_format", self.pixel_format,
            "-video_size", f"{self.width}x{self.height}",
            "-framerate", str(self.framerate),
            "-i", self.device_path,
        ]

        if self.has_audio:
            args += [
                "-f", "alsa",
                "-thread_queue_size", "1024",
                "-i", self.audio_device,
            ]

        return args

    @property
    def info_str(self) -> str:
        if not self._detected:
            return "No device"
        dev_name = os.path.basename(self.device_path) if self.device_path else "?"
        return (
            f"{dev_name}\n"
            f"{self.width}x{self.height}@{self.framerate}fps\n"
            f"{(self.pixel_format or '?').upper()}\n"
            f"Audio: {'yes' if self.has_audio else 'no'}"
        )
