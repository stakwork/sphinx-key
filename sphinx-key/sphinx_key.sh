#!/bin/bash

set -e

check_exists() {
    command -v "$1" > /dev/null
}
if ! check_exists esptool.py
then
    echo "esptool.py not installed!"
    echo "install with this command: pip install esptool"
    exit 1
fi
if ! check_exists ldproxy
then
    echo "ldproxy not installed!"
    echo "install with this command: cargo install ldproxy"
    exit 1
fi
if ! check_exists cargo-espflash
then
    echo "cargo-espflash not installed!"
    echo "install with this command: cargo install cargo-espflash"
    exit 1
fi
if ! check_exists espflash
then
    echo "espflash not installed!"
    echo "install with this command: cargo install espflash"
    exit 1
fi
cargo espflash save-image --bin sphinx-key --release --chip esp32c3 sphinx-key.bin
espflash write-bin 0x50000 sphinx-key.bin
cargo espflash monitor
