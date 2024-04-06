#!/bin/bash

set -e

trap 'exit 0' INT

source .env

ip_address=""

init() {
    if [ "$1" = "help" ] || [ -n "$1" ]; then
        return 0
    fi

    mac_address=$(echo " $MAC_ADDRESS " | sed 's/:0*/:/g')
    ip_address=$(arp -an | grep "$mac_address" | awk '{print $2}' | tr -d '()')

    if [ -z "$ip_address" ]; then
        mac_address="$MAC_ADDRESS"
        ip_address=$(sudo arp-scan --localnet | grep "$mac_address" | awk '{print $1}')
    fi

    if [ -z "$ip_address" ]; then
        echo "Could not discover ip address of the device"
        exit 1
    fi
}

help() {
    echo "Soccer"
    echo ""
    echo "USAGE:"
    echo "    ./scripts/api.sh [COMMAND]"
    echo ""
    echo "COMMANDS:"
    echo "    help       Show usage information."
    echo "    dashboard  Open dashboard in browser."
    echo "    ip         Print ip address of the device."
    echo "    logs       Stream logs to console."
    echo "    update     Upload software and restart."
    echo "    run        Build, update, and logs."
}

init

case "$1" in
    help|"")
        help
        ;;
    dashboard)
        open "http://$ip_address"
        ;;
    ip)
        echo "$ip_address"
        ;;
    logs)
        while true; do
            until timeout 0.2 bash -c "ping -c1 $ip_address &> /dev/null"; do :; done
            websocat -tE --ping-interval=1 --ping-timeout=3 --no-line "ws://$ip_address/logs" | defmt-print -e "target/thumbv6m-none-eabi/release/soccer-main"
            echo "Reconnecting..."
        done
        ;;
    update)
        rust-objcopy -O binary "target/thumbv6m-none-eabi/release/soccer-main" - | websocat -bEB 128 "ws://$ip_address/update"
        ;;
    run)
        echo "Building..." && cargo build -p soccer-main --release
        echo "Updating..." && ./scripts/api.sh update && sleep 5
        echo "Waiting..." && ./scripts/api.sh logs
        ;;
    *)
        echo "The command $1 was not found."
        echo ""
        echo "Use ./scripts/api.sh help for more information."
        ;;
esac
