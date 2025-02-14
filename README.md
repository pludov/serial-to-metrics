A simple utility to read metrics from a serial link (ttyS...) in prometheus exposition
format, and forward them to a victoria-metrics endpoint

# Installation

Run `./install.sh`

# Configuration

To change the serial port (default to ttyS4), create a configuration file `/etc/systemd/system/serial-to-metrics.service.d/serial.conf` with the following content:

```
[Service]
Environment="SERIAL_PORT=/dev/ttyS1"
```

The following environment variables can be set:
  * `SERIAL_PORT`: the serial port to read from
  * `METRIC_URL`: the URL of the victoria-metrics endpoint (default to `http://localhost:8428/api/v1/import/prometheus`)
  * `DELAY`: Delay between updates in ms
  * `VERBOSE`: Print the metrics read from the serial port
