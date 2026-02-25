#!/usr/bin/env bash
# install.sh — Bootstrap pi-recorder on a fresh Raspberry Pi OS install
set -euo pipefail

echo "=== pi-recorder installer ==="

# ── System packages ────────────────────────────────────────────────────
echo "Installing system packages..."
sudo apt-get update -q
sudo apt-get install -y \
    ffmpeg \
    v4l-utils \
    python3-pip \
    python3-venv \
    python3-pil \
    libopenjp2-7 \
    fonts-dejavu-core

# ── Enable SPI interface ────────────────────────────────────────────────
if ! grep -q "^dtparam=spi=on" /boot/config.txt 2>/dev/null && \
   ! grep -q "^dtparam=spi=on" /boot/firmware/config.txt 2>/dev/null; then
    echo "Enabling SPI..."
    BOOT_CFG=/boot/firmware/config.txt
    [ -f /boot/config.txt ] && BOOT_CFG=/boot/config.txt
    echo "dtparam=spi=on" | sudo tee -a "$BOOT_CFG"
    echo "  ⚠  SPI enabled — reboot required before running pi-recorder"
fi

# ── Python virtual environment ─────────────────────────────────────────
VENV_DIR="$(dirname "$0")/venv"
echo "Creating Python virtual environment at $VENV_DIR..."
python3 -m venv "$VENV_DIR"
source "$VENV_DIR/bin/activate"

# ── Waveshare ePaper library ────────────────────────────────────────────
WAVESHARE_DIR="$(dirname "$0")/waveshare_epd_src"
if [ ! -d "$WAVESHARE_DIR" ]; then
    echo "Cloning Waveshare e-Paper library..."
    git clone --depth=1 https://github.com/waveshare/e-Paper.git "$WAVESHARE_DIR"
fi
echo "Installing Waveshare ePaper Python library..."
pip install "$WAVESHARE_DIR/RaspberryPi_JetsonNano/python/" --quiet

# ── Python dependencies ─────────────────────────────────────────────────
echo "Installing Python dependencies..."
pip install -r "$(dirname "$0")/requirements.txt" --quiet

echo ""
echo "=== Installation complete ==="
echo ""
echo "To run pi-recorder:"
echo "  source venv/bin/activate"
echo "  python main.py"
echo ""
echo "Options:"
echo "  python main.py --help"
echo "  python main.py --device /dev/video0 --output ~/Videos/recordings --debug"
