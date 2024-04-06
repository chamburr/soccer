#!/bin/bash

set -e

echo "Intialising..."
probe-rs reset --chip RP2040
cargo build -p soccer-main --release

echo "Flashing bootloader..."
cargo flash -p soccer-bootloader --chip RP2040 --release

echo "Flashing firmware..."
probe-rs download firmware/43439A0.bin --format bin --chip RP2040 --base-address 0x10108000
probe-rs download firmware/43439A0_clm.bin --format bin --chip RP2040 --base-address 0x10148000

echo "Flashing software..."
cargo flash -p soccer-main --chip RP2040 --release
