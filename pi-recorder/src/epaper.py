"""
Waveshare 2.7" ePaper HAT display manager.

Display specs: 264 x 176 pixels, black & white.
SPI0, CE0 (MOSI=BCM10, CS=BCM8) — waveshare_epd handles the SPI wiring.

Four stat views (cycled by KEY4):
  0 — Recording status  (state, elapsed, file size)
  1 — System stats      (CPU %, temp, RAM, disk)
  2 — Capture device    (device path, resolution, format)
  3 — Recent files      (last recordings saved to disk)

Full refresh is slow (~2 s) on ePaper; this driver does a full refresh on
state change and a partial refresh for ticking counters.
"""

import logging
import os
import time
import threading
from pathlib import Path

logger = logging.getLogger(__name__)

# Display dimensions for the Waveshare 2.7" HAT
DISPLAY_W = 264
DISPLAY_H = 176

# How often to refresh the live counters (seconds)
LIVE_REFRESH_INTERVAL = 5.0

NUM_VIEWS = 4
VIEW_NAMES = ["Status", "System", "Capture", "Files"]


class DisplayManager:
    """
    Renders status information to the Waveshare 2.7" ePaper display.

    The display is updated:
      - Immediately on a recorder state change (full refresh)
      - Every LIVE_REFRESH_INTERVAL seconds while recording (partial refresh)
      - Immediately when the view is toggled (full refresh)
    """

    def __init__(self, recorder, capture_dev, stats_collector):
        self.recorder = recorder
        self.capture_dev = capture_dev
        self.stats = stats_collector

        self._view = 0
        self._epd = None
        self._font_large = None
        self._font_med = None
        self._font_small = None
        self._lock = threading.Lock()
        self._refresh_thread: threading.Thread | None = None
        self._stop_event = threading.Event()
        self._ready = False

    # ------------------------------------------------------------------ #
    # Setup / teardown                                                     #
    # ------------------------------------------------------------------ #

    def setup(self):
        """Initialise the ePaper display and fonts."""
        try:
            from waveshare_epd import epd2in7
            self._epd = epd2in7.EPD()
            self._epd.init()
            self._epd.Clear(0xFF)  # White background
            logger.info("ePaper display initialised (264x176)")
        except ImportError:
            logger.warning(
                "waveshare_epd not installed — display disabled. "
                "Install from: https://github.com/waveshare/e-Paper "
                "or: pip install waveshare-epaper"
            )
            self._epd = None
        except Exception as e:
            logger.error("ePaper init failed: %s", e)
            self._epd = None

        self._load_fonts()
        self._ready = True

        # Start background refresh thread
        self._stop_event.clear()
        self._refresh_thread = threading.Thread(
            target=self._live_refresh_loop,
            daemon=True,
            name="epaper-refresh",
        )
        self._refresh_thread.start()

    def cleanup(self):
        """Put the display to sleep and stop background thread."""
        self._stop_event.set()
        if self._refresh_thread:
            self._refresh_thread.join(timeout=3)

        if self._epd:
            try:
                self._epd.init()
                self._epd.Clear(0xFF)
                self._epd.sleep()
            except Exception as e:
                logger.warning("ePaper sleep failed: %s", e)

    def next_view(self):
        """Cycle to the next stats view and force a full refresh."""
        self._view = (self._view + 1) % NUM_VIEWS
        logger.info("View → %s", VIEW_NAMES[self._view])
        self._full_refresh()

    def on_state_change(self, new_state):
        """Called by the recorder on every state transition."""
        self._full_refresh()

    # ------------------------------------------------------------------ #
    # Rendering                                                            #
    # ------------------------------------------------------------------ #

    def _full_refresh(self):
        with self._lock:
            image = self._render()
            self._push_to_display(image, partial=False)

    def _partial_refresh(self):
        with self._lock:
            image = self._render()
            self._push_to_display(image, partial=True)

    def _render(self):
        """Build a PIL Image for the current view."""
        try:
            from PIL import Image, ImageDraw, ImageFont
        except ImportError:
            return None

        img = Image.new("1", (DISPLAY_W, DISPLAY_H), 255)  # white background
        draw = ImageDraw.Draw(img)

        view = self._view
        if view == 0:
            self._draw_status_view(draw)
        elif view == 1:
            self._draw_system_view(draw)
        elif view == 2:
            self._draw_capture_view(draw)
        elif view == 3:
            self._draw_files_view(draw)

        # View indicator bar at the bottom
        self._draw_view_bar(draw)

        return img

    def _draw_status_view(self, draw):
        """View 0: Recording status."""
        from .recorder import RecorderState
        from .stats import format_duration, format_bytes

        state = self.recorder.state
        session = self.recorder.session

        # State badge
        state_text = state.name
        state_fill = 0  # black
        if state == RecorderState.RECORDING:
            # Draw filled rectangle for recording indicator
            draw.rectangle([0, 0, DISPLAY_W, 28], fill=0)
            draw.text((8, 4), f"● {state_text}", font=self._font_large, fill=255)
        elif state == RecorderState.PAUSED:
            draw.rectangle([0, 0, DISPLAY_W, 28], fill=0)
            draw.text((8, 4), f"⏸ {state_text}", font=self._font_large, fill=255)
        else:
            draw.rectangle([0, 0, DISPLAY_W, 28], outline=0, fill=255)
            draw.text((8, 4), f"○ {state_text}", font=self._font_large, fill=0)

        y = 34
        if session:
            elapsed = format_duration(session.elapsed)
            size = format_bytes(session.file_size)
            segs = len(session.segments)

            draw.text((8, y), f"Time:  {elapsed}", font=self._font_med, fill=0)
            y += 20
            draw.text((8, y), f"Size:  {size}", font=self._font_med, fill=0)
            y += 20
            draw.text((8, y), f"Segs:  {segs}", font=self._font_med, fill=0)
            y += 20
            # File name (truncated)
            fname = os.path.basename(session.output_path)
            if len(fname) > 26:
                fname = fname[:23] + "..."
            draw.text((8, y), fname, font=self._font_small, fill=0)
        else:
            draw.text((8, y), "Press KEY1 to start", font=self._font_med, fill=0)

    def _draw_system_view(self, draw):
        """View 1: System stats."""
        self.stats.refresh()
        draw.text((4, 2), "SYSTEM", font=self._font_large, fill=0)
        draw.line([0, 24, DISPLAY_W, 24], fill=0, width=1)

        lines = self.stats.summary_lines()
        y = 30
        for line in lines:
            draw.text((8, y), line, font=self._font_med, fill=0)
            y += 22

    def _draw_capture_view(self, draw):
        """View 2: Capture device info."""
        draw.text((4, 2), "CAPTURE", font=self._font_large, fill=0)
        draw.line([0, 24, DISPLAY_W, 24], fill=0, width=1)

        y = 30
        for line in self.capture_dev.info_str.splitlines():
            draw.text((8, y), line, font=self._font_med, fill=0)
            y += 22

    def _draw_files_view(self, draw):
        """View 3: Recent recording files."""
        draw.text((4, 2), "RECENT FILES", font=self._font_large, fill=0)
        draw.line([0, 24, DISPLAY_W, 24], fill=0, width=1)

        output_dir = os.path.expanduser(self.recorder.output_dir)
        y = 30
        try:
            files = sorted(
                Path(output_dir).glob("*.mkv"),
                key=lambda p: p.stat().st_mtime,
                reverse=True,
            )[:4]
            if not files:
                draw.text((8, y), "No recordings yet", font=self._font_small, fill=0)
            else:
                from .stats import format_bytes
                for f in files:
                    name = f.name
                    if len(name) > 22:
                        name = name[:19] + "..."
                    size = format_bytes(f.stat().st_size)
                    draw.text((8, y), f"{name}", font=self._font_small, fill=0)
                    draw.text((8, y + 12), f"  {size}", font=self._font_small, fill=0)
                    y += 28
        except OSError:
            draw.text((8, y), "Cannot read dir", font=self._font_small, fill=0)

    def _draw_view_bar(self, draw):
        """Draw small dots at the bottom indicating active view."""
        dot_r = 3
        spacing = 14
        total_w = NUM_VIEWS * spacing
        start_x = (DISPLAY_W - total_w) // 2
        y = DISPLAY_H - 10

        for i in range(NUM_VIEWS):
            cx = start_x + i * spacing + dot_r
            if i == self._view:
                draw.ellipse([cx - dot_r, y - dot_r, cx + dot_r, y + dot_r], fill=0)
            else:
                draw.ellipse([cx - dot_r, y - dot_r, cx + dot_r, y + dot_r], outline=0)

    def _push_to_display(self, image, partial: bool = False):
        """Send a PIL image to the physical ePaper display."""
        if self._epd is None or image is None:
            # Log to console when no hardware
            self._log_to_console()
            return

        try:
            buf = self._epd.getbuffer(image)
            if partial:
                # epd2in7 doesn't have a native partial update, use full
                self._epd.display(buf)
            else:
                self._epd.display(buf)
        except Exception as e:
            logger.error("ePaper display update failed: %s", e)

    def _log_to_console(self):
        """Print status to console when no ePaper hardware is present."""
        lines = self.recorder.get_status_lines()
        print(f"\n[Display:{VIEW_NAMES[self._view]}] " + " | ".join(lines))

    def _live_refresh_loop(self):
        """Background thread: refresh live stats periodically."""
        while not self._stop_event.is_set():
            self._stop_event.wait(timeout=LIVE_REFRESH_INTERVAL)
            if not self._stop_event.is_set():
                from .recorder import RecorderState
                if self.recorder.state != RecorderState.IDLE:
                    self._partial_refresh()

    # ------------------------------------------------------------------ #
    # Font loading                                                         #
    # ------------------------------------------------------------------ #

    def _load_fonts(self):
        """Load fonts from common system paths or fall back to default."""
        try:
            from PIL import ImageFont
        except ImportError:
            return

        search_paths = [
            # Raspberry Pi OS fonts
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            # macOS / generic fallbacks
            "/Library/Fonts/Arial.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
        ]

        font_path = None
        for path in search_paths:
            if os.path.exists(path):
                font_path = path
                break

        try:
            if font_path:
                self._font_large = ImageFont.truetype(font_path, 18)
                self._font_med = ImageFont.truetype(font_path, 14)
                self._font_small = ImageFont.truetype(font_path, 11)
                logger.debug("Loaded font: %s", font_path)
            else:
                raise OSError("No TTF font found")
        except (OSError, IOError):
            logger.warning("Using PIL default font — text may look pixelated")
            default = ImageFont.load_default()
            self._font_large = default
            self._font_med = default
            self._font_small = default
