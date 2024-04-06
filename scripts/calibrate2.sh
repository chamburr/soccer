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

collect() {
    read -p "Hold robot still and press enter... ($1/$2) "
    echo "Collecting data... ($1/$2)"

    websocat -t --no-line "autoreconnect:ws://$ip_address/logs" "writefile:/tmp/calibrate2-$1.txt" &
    sleep 4 && kill $! > /dev/null && wait $! 2>/dev/null || true

    output=$(cat "/tmp/calibrate2-$1.txt" | defmt-print -e "target/thumbv6m-none-eabi/release/soccer-main")

    echo "$output" | grep 'Imu acc' | sed -e '$d' -e 's/.*: //' -e 's/, /,/g' | tail -200 >> /tmp/calibrate2.txt

    rm "/tmp/calibrate2-$1.txt"

    echo "Done collecting data ($1/$2)"
}

curl -s -d "name=print_imu" -d "args=enable=true" "http://$ip_address/api/execute" > /dev/null

echo -n "" > /tmp/calibrate2.txt

count=8
for i in $(seq $count); do
    collect $i $count
done

curl -s -d "name=print_imu" -d "args=enable=false" "http://$ip_address/api/execute" > /dev/null

cd scripts
source venv/bin/activate
python3 calibrate.py /tmp/calibrate2.txt 2

rm /tmp/calibrate2.txt
