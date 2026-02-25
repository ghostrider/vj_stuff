"""
System statistics collection for the status display.
"""

import os
import time
import logging

logger = logging.getLogger(__name__)


def _read_file(path: str) -> str:
    try:
        with open(path) as f:
            return f.read().strip()
    except OSError:
        return ""


def cpu_percent() -> float:
    """Return CPU usage as a percentage (0-100)."""
    try:
        import psutil
        return psutil.cpu_percent(interval=0.1)
    except ImportError:
        pass
    # Manual calculation from /proc/stat
    try:
        def _read_stat():
            line = _read_file("/proc/stat").splitlines()[0]
            vals = list(map(int, line.split()[1:]))
            idle = vals[3]
            total = sum(vals)
            return idle, total

        idle1, total1 = _read_stat()
        time.sleep(0.1)
        idle2, total2 = _read_stat()
        delta_total = total2 - total1
        delta_idle = idle2 - idle1
        if delta_total == 0:
            return 0.0
        return 100.0 * (1.0 - delta_idle / delta_total)
    except Exception:
        return 0.0


def cpu_temp_celsius() -> float | None:
    """Return CPU temperature in Celsius, or None if unavailable."""
    # Raspberry Pi thermal zone
    for path in [
        "/sys/class/thermal/thermal_zone0/temp",
        "/sys/devices/virtual/thermal/thermal_zone0/temp",
    ]:
        raw = _read_file(path)
        if raw:
            try:
                return int(raw) / 1000.0
            except ValueError:
                pass
    return None


def ram_usage() -> tuple[int, int]:
    """Return (used_bytes, total_bytes) of RAM."""
    try:
        import psutil
        vm = psutil.virtual_memory()
        return vm.used, vm.total
    except ImportError:
        pass
    try:
        mem = {}
        for line in _read_file("/proc/meminfo").splitlines():
            parts = line.split()
            if len(parts) >= 2:
                mem[parts[0].rstrip(":")] = int(parts[1]) * 1024
        total = mem.get("MemTotal", 0)
        available = mem.get("MemAvailable", mem.get("MemFree", 0))
        return total - available, total
    except Exception:
        return 0, 0


def disk_usage(path: str = "/") -> tuple[int, int]:
    """Return (used_bytes, total_bytes) for the filesystem at path."""
    try:
        st = os.statvfs(path)
        total = st.f_blocks * st.f_frsize
        free = st.f_bfree * st.f_frsize
        return total - free, total
    except OSError:
        return 0, 0


def format_bytes(n: int) -> str:
    """Human-readable byte count."""
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if n < 1024:
            return f"{n:.1f}{unit}"
        n /= 1024
    return f"{n:.1f}PB"


def format_duration(seconds: float) -> str:
    """Format elapsed seconds as HH:MM:SS."""
    secs = int(seconds)
    h = secs // 3600
    m = (secs % 3600) // 60
    s = secs % 60
    if h:
        return f"{h:02d}:{m:02d}:{s:02d}"
    return f"{m:02d}:{s:02d}"


def get_file_size(path: str) -> int:
    """Return file size in bytes, or 0 if file doesn't exist."""
    try:
        return os.path.getsize(path)
    except OSError:
        return 0


class StatsCollector:
    """Collects and caches system stats with a configurable refresh interval."""

    def __init__(self, refresh_interval: float = 3.0):
        self.refresh_interval = refresh_interval
        self._last_refresh = 0.0
        self._cache: dict = {}

    def refresh(self, force: bool = False):
        now = time.monotonic()
        if not force and (now - self._last_refresh) < self.refresh_interval:
            return
        self._last_refresh = now

        cpu = cpu_percent()
        temp = cpu_temp_celsius()
        ram_used, ram_total = ram_usage()
        disk_used, disk_total = disk_usage("/")

        self._cache = {
            "cpu_pct": cpu,
            "cpu_temp": temp,
            "ram_used": ram_used,
            "ram_total": ram_total,
            "disk_used": disk_used,
            "disk_total": disk_total,
        }

    @property
    def cpu_pct(self) -> float:
        return self._cache.get("cpu_pct", 0.0)

    @property
    def cpu_temp(self) -> float | None:
        return self._cache.get("cpu_temp")

    @property
    def ram_used(self) -> int:
        return self._cache.get("ram_used", 0)

    @property
    def ram_total(self) -> int:
        return self._cache.get("ram_total", 0)

    @property
    def disk_used(self) -> int:
        return self._cache.get("disk_used", 0)

    @property
    def disk_total(self) -> int:
        return self._cache.get("disk_total", 0)

    def summary_lines(self) -> list[str]:
        self.refresh()
        lines = [f"CPU: {self.cpu_pct:.0f}%"]
        if self.cpu_temp is not None:
            lines.append(f"Temp: {self.cpu_temp:.1f}C")
        if self.ram_total:
            pct = 100 * self.ram_used // self.ram_total
            lines.append(
                f"RAM: {format_bytes(self.ram_used)}/{format_bytes(self.ram_total)} {pct}%"
            )
        if self.disk_total:
            free = self.disk_total - self.disk_used
            lines.append(f"Disk free: {format_bytes(free)}")
        return lines
