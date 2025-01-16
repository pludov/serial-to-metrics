#!/bin/bash


cargo build --release

sudo cp target/release/serial-to-metrics /usr/bin/
sudo cp serial-to-metrics.service /etc/systemd/system/
# daemon-reload is needed to make systemd aware of the new service
sudo systemctl daemon-reload
sudo systemctl enable serial-to-metrics
sudo systemctl start serial-to-metrics
