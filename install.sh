#!/bin/bash

# This script is used to install the serial-to-metrics service on a Raspberry Pi.
# It will build & install the binary and setup the systemd service

set -euo pipefail

cargo build --release

sudo cp target/release/serial-to-metrics /usr/bin/
sudo cp systemd/serial-to-metrics.service /etc/systemd/system/

sudo systemctl daemon-reload
sudo systemctl enable serial-to-metrics
sudo systemctl start serial-to-metrics
