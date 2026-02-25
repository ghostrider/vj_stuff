#!/usr/bin/env python3
"""
pi-recorder — Raspberry Pi 5 HDMI video recorder
with Waveshare 2.7" ePaper HAT display.

Hardware:
  - Raspberry Pi 5
  - Waveshare 2.7" ePaper HAT (SPI0, CE0 — BCM10/8)
  - USB HDMI capture card (/dev/video*)

Button mapping (HAT left side, top to bottom):
  KEY1 (BCM 5)  → Start recording
  KEY2 (BCM 6)  → Stop recording
  KEY3 (BCM 13) → Pause / Resume recording
  KEY4 (BCM 19) → Toggle stats view

Usage:
  python main.py [--device /dev/videoN] [--output ~/Videos/recordings] [--debug]
"""

import argparse
import logging
import signal
import sys
import time

from src.capture import CaptureDevice
from src.recorder import RecordingEngine, RecorderState
from src.epaper import DisplayManager
from src.buttons import ButtonHandler
from src.stats import StatsCollector


def parse_args():
    parser = argparse.ArgumentParser(
        description="Raspberry Pi HDMI video recorder with ePaper display"
    )
    parser.add_argument(
        "--device", "-d",
        default=None,
        help="Video capture device path (e.g. /dev/video0). Auto-detected if omitted.",
    )
    parser.add_argument(
        "--output", "-o",
        default="~/Videos/recordings",
        help="Output directory for recordings (default: ~/Videos/recordings)",
    )
    parser.add_argument(
        "--container",
        default="mkv",
        choices=["mkv", "mp4", "avi"],
        help="Output container format (default: mkv)",
    )
    parser.add_argument(
        "--video-codec",
        default="copy",
        help="Video codec (default: copy — passthrough, no re-encoding)",
    )
    parser.add_argument(
        "--audio-codec",
        default="aac",
        help="Audio codec (default: aac)",
    )
    parser.add_argument(
        "--debug",
        action="store_true",
        help="Enable verbose debug logging",
    )
    return parser.parse_args()


def setup_logging(debug: bool):
    level = logging.DEBUG if debug else logging.INFO
    logging.basicConfig(
        level=level,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )


def main():
    args = parse_args()
    setup_logging(args.debug)

    logger = logging.getLogger("main")
    logger.info("pi-recorder starting up")

    # ------------------------------------------------------------------ #
    # 1. Detect capture device                                            #
    # ------------------------------------------------------------------ #
    capture = CaptureDevice(device_path=args.device)
    if not capture.detect():
        logger.error(
            "No USB capture device found. "
            "Ensure your HDMI capture card is connected and recognised by v4l2.\n"
            "Check with: v4l2-ctl --list-devices"
        )
        sys.exit(1)

    logger.info("Capture device: %s", capture.info_str.replace("\n", " | "))

    # ------------------------------------------------------------------ #
    # 2. System stats collector                                           #
    # ------------------------------------------------------------------ #
    stats = StatsCollector(refresh_interval=3.0)
    stats.refresh(force=True)

    # ------------------------------------------------------------------ #
    # 3. Recording engine                                                 #
    # ------------------------------------------------------------------ #
    # on_state_change is wired to the display after the display is created
    recorder = RecordingEngine(
        capture=capture,
        output_dir=args.output,
        container=args.container,
        video_codec=args.video_codec,
        audio_codec=args.audio_codec,
    )

    # ------------------------------------------------------------------ #
    # 4. ePaper display                                                   #
    # ------------------------------------------------------------------ #
    display = DisplayManager(
        recorder=recorder,
        capture_dev=capture,
        stats_collector=stats,
    )
    # Wire recorder state changes to display updates
    recorder.on_state_change = display.on_state_change

    display.setup()

    # ------------------------------------------------------------------ #
    # 5. Button callbacks                                                 #
    # ------------------------------------------------------------------ #
    def on_start():
        if recorder.state == RecorderState.IDLE:
            logger.info("KEY1: Start recording")
            recorder.start()
        else:
            logger.info("KEY1: Already recording or paused — ignored")

    def on_stop():
        if recorder.state != RecorderState.IDLE:
            logger.info("KEY2: Stop recording")
            final = recorder.stop()
            if final:
                logger.info("Saved: %s", final)
        else:
            logger.info("KEY2: Not recording — ignored")

    def on_pause_resume():
        if recorder.state == RecorderState.RECORDING:
            logger.info("KEY3: Pause recording")
            recorder.pause()
        elif recorder.state == RecorderState.PAUSED:
            logger.info("KEY3: Resume recording")
            recorder.resume()
        else:
            logger.info("KEY3: Idle — ignored")

    def on_toggle_stats():
        logger.info("KEY4: Toggle stats view")
        display.next_view()

    buttons = ButtonHandler(
        on_start=on_start,
        on_stop=on_stop,
        on_pause_resume=on_pause_resume,
        on_toggle_stats=on_toggle_stats,
    )
    buttons.setup()

    # ------------------------------------------------------------------ #
    # 6. Graceful shutdown                                                #
    # ------------------------------------------------------------------ #
    def shutdown(signum, frame):
        logger.info("Shutdown signal received (sig %d)", signum)
        if recorder.state != RecorderState.IDLE:
            logger.info("Stopping active recording before exit...")
            recorder.stop()
        display.cleanup()
        buttons.cleanup()
        logger.info("pi-recorder stopped")
        sys.exit(0)

    signal.signal(signal.SIGINT, shutdown)
    signal.signal(signal.SIGTERM, shutdown)

    # ------------------------------------------------------------------ #
    # 7. Initial display render                                           #
    # ------------------------------------------------------------------ #
    display.on_state_change(RecorderState.IDLE)

    # ------------------------------------------------------------------ #
    # 8. Main loop                                                        #
    # ------------------------------------------------------------------ #
    logger.info("Ready. Waiting for button input...")
    print("\n  KEY1 = Start | KEY2 = Stop | KEY3 = Pause/Resume | KEY4 = Stats\n")

    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        shutdown(signal.SIGINT, None)


if __name__ == "__main__":
    main()
