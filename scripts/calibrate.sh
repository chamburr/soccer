#!/bin/bash

set -e

if [ ! -d "scripts/venv" ]; then
    cd scripts
    echo "Installing dependencies"
    virtualenv venv
    source venv/bin/activate
    pip3 install -r requirements.txt
    cd ../
fi

cargo build -p soccer-vision --features calibration --release

read -p "Unpower, put into bootloader mode and press enter..."

echo "Collecting data for 30 seconds..."

cargo run -p soccer-vision --features calibration --release > /tmp/calibrate.txt &
sleep 30 && kill -SIGINT $! > /dev/null || true

sleep 2

output=$(cat /tmp/calibrate.txt)
echo "$output" | grep --text 'Imu mag' | sed -e '$d' -e 's/.*: //' -e 's/, /,/g' > /tmp/calibrate.txt

echo "Done collecting data"

cd scripts
source venv/bin/activate
python3 calibrate.py /tmp/calibrate.txt 1

rm /tmp/calibrate.txt
