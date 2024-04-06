#!/bin/bash

set -e

if [ ! -d "scripts/venv" ]; then
    cd scripts
    echo "Installing dependencies"
    virtualenv venv
    source venv/bin/activate
    pip3 install -r requirements.txt
    cd ../
else
    source venv/bin/activate
fi

ip_address=$(./scripts/api.sh ip)

curl -s -d "name=print_imu" -d "args=enable=true" "http://$ip_address/api/execute" > /dev/null

echo "Collecting data for 30 seconds..."

websocat -t --no-line "autoreconnect:ws://$ip_address/logs" "writefile:/tmp/calibrate.txt" &
sleep 30 && kill $! > /dev/null && wait $! 2>/dev/null || true

output=$(cat "/tmp/calibrate.txt" | defmt-print -e "target/thumbv6m-none-eabi/release/soccer-main")

echo "$output" | grep 'Imu mag' | sed -e '$d' -e 's/.*: //' -e 's/, /,/g' > /tmp/calibrate.txt

echo "Done collecting data"

curl -s -d "name=print_imu" -d "args=enable=false" "http://$ip_address/api/execute" > /dev/null

cd scripts
source venv/bin/activate
python3 calibrate.py /tmp/calibrate.txt 1

rm /tmp/calibrate.txt
