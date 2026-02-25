"""
GPIO button handler for the Waveshare 2.7" ePaper HAT.

Button layout (physical, top to bottom on the left side of the HAT):
  KEY1 → BCM 5  → Start recording
  KEY2 → BCM 6  → Stop recording
  KEY3 → BCM 13 → Pause / Resume recording
  KEY4 → BCM 19 → Toggle stats view

All buttons are active-low with the HAT's built-in pull-ups.
"""

import logging
import threading

logger = logging.getLogger(__name__)

# BCM pin numbers for the four keys (top to bottom)
KEY1_PIN = 5
KEY2_PIN = 6
KEY3_PIN = 13
KEY4_PIN = 19

BUTTON_PINS = [KEY1_PIN, KEY2_PIN, KEY3_PIN, KEY4_PIN]
BUTTON_NAMES = ["KEY1 (Start)", "KEY2 (Stop)", "KEY3 (Pause/Resume)", "KEY4 (Stats)"]

# Debounce period in seconds
DEBOUNCE_S = 0.2


class ButtonHandler:
    """
    Sets up the four HAT buttons and wires them to callbacks.

    Usage:
        handler = ButtonHandler(
            on_start=...,
            on_stop=...,
            on_pause_resume=...,
            on_toggle_stats=...,
        )
        handler.setup()
        # ... run your main loop ...
        handler.cleanup()
    """

    def __init__(
        self,
        on_start=None,
        on_stop=None,
        on_pause_resume=None,
        on_toggle_stats=None,
    ):
        self.callbacks = {
            KEY1_PIN: on_start,
            KEY2_PIN: on_stop,
            KEY3_PIN: on_pause_resume,
            KEY4_PIN: on_toggle_stats,
        }
        self._buttons = {}
        self._debounce_timers: dict[int, threading.Timer] = {}
        self._ready = False

    def setup(self):
        """Initialise GPIO and start listening for button presses."""
        try:
            from gpiozero import Button as GpioButton
            from gpiozero import Device
        except ImportError:
            logger.warning(
                "gpiozero not installed — buttons disabled. "
                "Install with: pip install gpiozero"
            )
            self._setup_mock()
            return

        for pin in BUTTON_PINS:
            btn = GpioButton(pin, pull_up=True, bounce_time=DEBOUNCE_S)
            btn.when_pressed = self._make_handler(pin)
            self._buttons[pin] = btn
            logger.debug("Button on BCM %d ready", pin)

        self._ready = True
        logger.info("All 4 HAT buttons initialised (BCM 5/6/13/19)")

    def cleanup(self):
        """Release GPIO resources."""
        for btn in self._buttons.values():
            try:
                btn.close()
            except Exception:
                pass
        self._buttons.clear()
        self._ready = False
        logger.debug("Button GPIO released")

    # ------------------------------------------------------------------ #
    # Internal helpers                                                     #
    # ------------------------------------------------------------------ #

    def _make_handler(self, pin: int):
        def handler():
            cb = self.callbacks.get(pin)
            if cb:
                logger.info("Button pressed: %s", BUTTON_NAMES[BUTTON_PINS.index(pin)])
                try:
                    cb()
                except Exception as e:
                    logger.error("Button callback error on pin %d: %s", pin, e)
        return handler

    def _setup_mock(self):
        """
        Install a simple keyboard fallback for development on non-Pi hardware.
        Reads single keypresses: 1=start, 2=stop, 3=pause, 4=stats.
        Runs in a background thread so it doesn't block the main loop.
        """
        logger.info(
            "Mock buttons active — press 1/2/3/4 + Enter in terminal "
            "(1=Start, 2=Stop, 3=Pause/Resume, 4=Stats)"
        )
        pin_map = {
            "1": KEY1_PIN,
            "2": KEY2_PIN,
            "3": KEY3_PIN,
            "4": KEY4_PIN,
        }

        def _read_loop():
            import sys
            while True:
                try:
                    key = input("Button> ").strip()
                    pin = pin_map.get(key)
                    if pin is not None:
                        handler = self._make_handler(pin)
                        handler()
                except EOFError:
                    break
                except KeyboardInterrupt:
                    break

        t = threading.Thread(target=_read_loop, daemon=True, name="mock-buttons")
        t.start()
