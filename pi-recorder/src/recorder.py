"""
FFmpeg-based video recording engine.

Recording lifecycle:
  IDLE -> start() -> RECORDING -> pause() -> PAUSED -> resume() -> RECORDING
                                           -> stop()  -> IDLE
                  -> stop()  -> IDLE

Pause is implemented by stopping ffmpeg (saving the segment) and then
starting a fresh ffmpeg process on resume. When stop() is called with
multiple segments, they are concatenated into a single output file.
"""

import collections
import os
import signal
import subprocess
import threading
import time
import logging
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum, auto
from pathlib import Path

from .capture import CaptureDevice
from .stats import format_bytes, format_duration, get_file_size

logger = logging.getLogger(__name__)


class RecorderState(Enum):
    IDLE = auto()
    RECORDING = auto()
    PAUSED = auto()


@dataclass
class RecordingSession:
    start_time: float = field(default_factory=time.monotonic)
    pause_time: float | None = None
    paused_duration: float = 0.0          # accumulated paused seconds
    segments: list[str] = field(default_factory=list)  # segment file paths
    output_path: str = ""
    session_id: str = ""

    @property
    def elapsed(self) -> float:
        """Active recording time (excludes paused time)."""
        now = time.monotonic()
        base = now - self.start_time - self.paused_duration
        if self.pause_time is not None:
            # Currently paused
            base -= (now - self.pause_time)
        return max(0.0, base)

    @property
    def file_size(self) -> int:
        """Total size of all segments on disk."""
        return sum(get_file_size(s) for s in self.segments)


class RecordingEngine:
    """
    Manages ffmpeg processes to record from a USB capture device.

    Supports start, stop, pause and resume operations.
    Fires on_state_change(new_state) callback on every state transition.
    """

    def __init__(
        self,
        capture: CaptureDevice,
        output_dir: str = "~/Videos/recordings",
        container: str = "mkv",
        video_codec: str = "copy",
        audio_codec: str = "aac",
        on_state_change=None,
    ):
        self.capture = capture
        self.output_dir = os.path.expanduser(output_dir)
        self.container = container
        self.video_codec = video_codec
        self.audio_codec = audio_codec
        self.on_state_change = on_state_change

        self._state = RecorderState.IDLE
        self._session: RecordingSession | None = None
        self._process: subprocess.Popen | None = None
        self._lock = threading.Lock()
        self._stderr_lines: collections.deque = collections.deque(maxlen=80)

        os.makedirs(self.output_dir, exist_ok=True)

    # ------------------------------------------------------------------ #
    # Public API                                                           #
    # ------------------------------------------------------------------ #

    @property
    def state(self) -> RecorderState:
        return self._state

    @property
    def session(self) -> RecordingSession | None:
        return self._session

    def start(self) -> bool:
        """Begin a new recording session. Returns True on success."""
        with self._lock:
            if self._state != RecorderState.IDLE:
                logger.warning("start() called in state %s — ignored", self._state)
                return False

            session_id = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
            output_path = os.path.join(self.output_dir, f"{session_id}.{self.container}")
            session = RecordingSession(session_id=session_id, output_path=output_path)

            seg_path = self._segment_path(session, 0)
            if not self._launch_ffmpeg(seg_path):
                return False

            session.segments.append(seg_path)
            self._session = session
            self._set_state(RecorderState.RECORDING)
            logger.info("Recording started → %s", output_path)
            return True

    def stop(self) -> str | None:
        """
        Stop recording and finalise the output file.
        Returns the final output path, or None on failure.
        """
        with self._lock:
            if self._state == RecorderState.IDLE:
                logger.warning("stop() called while IDLE — ignored")
                return None

            self._stop_ffmpeg()
            session = self._session

            if len(session.segments) == 1:
                seg = session.segments[0]
                if not os.path.exists(seg):
                    logger.error(
                        "Segment file was never created: %s\nffmpeg output:\n%s",
                        seg,
                        "".join(self._stderr_lines) or "(no output captured)",
                    )
                    self._session = None
                    self._set_state(RecorderState.IDLE)
                    return None
                # Single segment — just move it to the final path
                final = session.output_path
                try:
                    os.rename(seg, final)
                except OSError as e:
                    logger.error("Rename failed: %s", e)
                    final = seg
            else:
                final = self._concatenate_segments(session)

            self._session = None
            self._set_state(RecorderState.IDLE)
            logger.info("Recording saved → %s", final)
            return final

    def pause(self) -> bool:
        """Pause recording (stops ffmpeg, keeps segment). Returns True on success."""
        with self._lock:
            if self._state != RecorderState.RECORDING:
                logger.warning("pause() called in state %s — ignored", self._state)
                return False

            self._stop_ffmpeg()
            self._session.pause_time = time.monotonic()
            self._set_state(RecorderState.PAUSED)
            logger.info("Recording paused after segment %d", len(self._session.segments))
            return True

    def resume(self) -> bool:
        """Resume recording from a paused state. Returns True on success."""
        with self._lock:
            if self._state != RecorderState.PAUSED:
                logger.warning("resume() called in state %s — ignored", self._state)
                return False

            session = self._session
            seg_idx = len(session.segments)
            seg_path = self._segment_path(session, seg_idx)

            if not self._launch_ffmpeg(seg_path):
                return False

            # Accumulate paused duration
            if session.pause_time is not None:
                session.paused_duration += time.monotonic() - session.pause_time
                session.pause_time = None

            session.segments.append(seg_path)
            self._set_state(RecorderState.RECORDING)
            logger.info("Recording resumed → segment %d", seg_idx)
            return True

    def get_status_lines(self) -> list[str]:
        """Return human-readable status lines for the display."""
        lines = [f"State: {self._state.name}"]
        if self._session:
            lines.append(f"Time: {format_duration(self._session.elapsed)}")
            lines.append(f"Size: {format_bytes(self._session.file_size)}")
            lines.append(f"Segs: {len(self._session.segments)}")
        return lines

    # ------------------------------------------------------------------ #
    # Internal helpers                                                     #
    # ------------------------------------------------------------------ #

    def _segment_path(self, session: RecordingSession, idx: int) -> str:
        return os.path.join(
            self.output_dir,
            f"{session.session_id}_seg{idx:03d}.{self.container}",
        )

    def _launch_ffmpeg(self, output_path: str) -> bool:
        """Start an ffmpeg process writing to output_path."""
        try:
            input_args = self.capture.build_ffmpeg_input_args()
        except RuntimeError as e:
            logger.error("Cannot build ffmpeg args: %s", e)
            return False

        video_codec_args = ["-c:v", self.video_codec]
        audio_codec_args = []
        if self.capture.has_audio:
            audio_codec_args = ["-c:a", self.audio_codec, "-b:a", "192k"]

        cmd = (
            ["ffmpeg", "-y"]
            + input_args
            + video_codec_args
            + audio_codec_args
            + [output_path]
        )

        logger.debug("ffmpeg cmd: %s", " ".join(cmd))
        self._stderr_lines.clear()
        try:
            self._process = subprocess.Popen(
                cmd,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
            )
        except FileNotFoundError:
            logger.error("ffmpeg not found — install with: sudo apt install ffmpeg")
            return False
        except OSError as e:
            logger.error("Failed to launch ffmpeg: %s", e)
            return False

        # Drain stderr in a background thread to prevent the 64 KB pipe buffer
        # from filling up and blocking ffmpeg mid-recording.
        threading.Thread(
            target=self._drain_stderr,
            args=(self._process, self._stderr_lines),
            daemon=True,
        ).start()

        # Give ffmpeg a moment to fail fast (e.g. device not found)
        time.sleep(0.3)
        if self._process.poll() is not None:
            time.sleep(0.1)  # let drain thread collect remaining lines
            logger.error(
                "ffmpeg exited immediately:\n%s",
                "".join(self._stderr_lines),
            )
            self._process = None
            return False

        return True

    @staticmethod
    def _drain_stderr(proc: subprocess.Popen, lines: collections.deque) -> None:
        """Read ffmpeg stderr continuously so the pipe buffer never fills up."""
        try:
            for raw in proc.stderr:
                lines.append(raw.decode(errors="replace"))
        except Exception:
            pass

    def _stop_ffmpeg(self):
        """Gracefully stop the running ffmpeg process."""
        proc = self._process
        self._process = None
        if proc is None or proc.poll() is not None:
            return
        try:
            proc.send_signal(signal.SIGINT)
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            logger.warning("ffmpeg did not exit on SIGINT — killing")
            proc.kill()
            proc.wait()
        except OSError:
            pass

    def _concatenate_segments(self, session: RecordingSession) -> str:
        """
        Concatenate multiple segments into a single file using ffmpeg concat.
        Returns the final output path.
        """
        concat_list = os.path.join(
            self.output_dir, f"{session.session_id}_concat.txt"
        )
        try:
            with open(concat_list, "w") as f:
                for seg in session.segments:
                    f.write(f"file '{seg}'\n")

            cmd = [
                "ffmpeg", "-y",
                "-f", "concat",
                "-safe", "0",
                "-i", concat_list,
                "-c", "copy",
                session.output_path,
            ]
            logger.info("Concatenating %d segments → %s", len(session.segments), session.output_path)
            result = subprocess.run(cmd, capture_output=True, timeout=300)
            if result.returncode != 0:
                logger.error("Concat failed:\n%s", result.stderr.decode(errors="replace"))
                # Return the first segment as fallback
                return session.segments[0]

            # Clean up segment files
            for seg in session.segments:
                try:
                    os.remove(seg)
                except OSError:
                    pass

        finally:
            try:
                os.remove(concat_list)
            except OSError:
                pass

        return session.output_path

    def _set_state(self, new_state: RecorderState):
        self._state = new_state
        if self.on_state_change:
            try:
                self.on_state_change(new_state)
            except Exception as e:
                logger.error("on_state_change callback raised: %s", e)
