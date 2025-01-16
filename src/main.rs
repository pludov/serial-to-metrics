use std::{
    collections::HashMap,
    io::{ErrorKind, Read},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use cli::Args;
use ureq::Error;
mod cli;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum Measurement {
    P,
    I,
    U,
}

impl Measurement {
    fn metric_name(&self) -> &'static str {
        match self {
            Measurement::P => "supply_consumed_power",
            Measurement::I => "supply_current",
            Measurement::U => "supply_voltage",
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
struct Metric {
    measurement: Measurement,
    timestamp: f64,
    value: f64,
}

struct SerialReceiver {
    verbose: bool,
    channel: std::sync::mpsc::Sender<Metric>,
}

impl SerialReceiver {
    fn parse_line(&self, line: &[u8]) -> Result<Option<(Measurement, f64)>, &str> {
        if line.len() == 0 {
            return Ok(None);
        }
        if line.len() == 1 && line[0] == b'\r' {
            return Ok(None);
        }
        for i in 0..line.len() {
            if line[i] > 127 {
                return Err("Non ascii char");
            }
        }
        if line.len() < 4 {
            return Err("Line too short");
        }

        let measurement = match line[0] {
            b'P' => Measurement::P,
            b'I' => Measurement::I,
            b'U' => Measurement::U,
            _ => return Err("Wrong first char"),
        };

        if line[1] != b' ' {
            return Err("No space");
        }

        if line[line.len() - 1] != b'\r' {
            return Err("No CR");
        }

        // Convert to a string
        let line_str = std::str::from_utf8(&line[2..line.len() - 1]).unwrap();

        let value: f64 = line_str.parse().map_err(|_e| "Not a number")?;

        Ok(Some((measurement, value)))
        //
    }

    fn handle_line(&self, line: &[u8]) {
        if self.verbose {
            println!("Received: {:?}", line);
        }
        let parsed = self.parse_line(line);
        if parsed.is_err() {
            println!("Could not parse line: {}", parsed.err().unwrap());
            return;
        }
        let parsed = parsed.unwrap();
        if parsed.is_none() {
            return;
        }
        let (measurement, value) = parsed.unwrap();

        self.channel
            .send(Metric {
                measurement,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Epoch overflow ?")
                    .as_secs_f64(),
                value,
            })
            .expect("Failed to send metric");
    }

    fn consume_lines(&self, line_buf: &mut [u8; 256], line_pos: &mut usize, bytes: usize) {
        let mut i = *line_pos;
        *line_pos += bytes;
        while i < *line_pos {
            if line_buf[i] == b'\n' {
                // Create an ascii string from the buffer
                self.handle_line(&line_buf[0..i]);

                // move the data to the begining of the buffer
                for j in i + 1..256 {
                    line_buf[j - i - 1] = line_buf[j];
                }

                *line_pos -= i + 1;
                i = 0;
                //println!("Buffer left is {:?}", &line_buf[0..*line_pos]);
            } else {
                i += 1;
            }
        }
    }

    fn monitor(&self, args: &Args) {
        let mut data_frame_start: Option<Instant> = None;

        // open the serial device
        loop {
            let port_open = serialport::new(&args.port, args.rate)
                .timeout(Duration::from_millis(args.data_min_interval))
                .open_native();

            let mut port = match port_open {
                Ok(port) => port,
                Err(err) => {
                    println!("Failed to open port {} : {:?}", args.port.as_str(), err);
                    std::thread::sleep(Duration::from_secs(5));
                    continue;
                }
            };

            let mut line_buf = [0u8; 256];
            let mut line_pos = 0;

            loop {
                match port.read(&mut line_buf[line_pos..256]) {
                    Ok(bytes) => {
                        if data_frame_start.is_none() {
                            data_frame_start = Some(Instant::now());
                        } else if data_frame_start.unwrap().elapsed()
                            > Duration::from_millis(args.data_min_interval)
                        {
                            println!(
                                "Data overflow: data reception lasted more than {} ms",
                                args.data_min_interval
                            );
                            break;
                        }
                        self.consume_lines(&mut line_buf, &mut line_pos, bytes);
                        if line_pos == 256 {
                            println!("Buffer full, giving up...");
                            break;
                        }
                    }
                    Err(err) => match err.kind() {
                        ErrorKind::TimedOut => {
                            if line_pos > 0 {
                                println!("Discarding data");
                            }
                            data_frame_start = None;
                            line_pos = 0;
                            line_buf = [0u8; 256];
                            continue;
                        }
                        _ => {
                            println!("Error reading from port: {}", err);
                            break;
                        }
                    },
                }
            }
            drop(port);
            std::thread::sleep(Duration::from_secs(5));
        }
    }
}

struct SerialSender {
    url: String,
    labels: String,
    channel: std::sync::mpsc::Receiver<Metric>,
    agent: ureq::Agent,
}

impl SerialSender {
    fn new(args: &Args, channel: std::sync::mpsc::Receiver<Metric>) -> Self {
        let mut labels;
        if args.labels.is_empty() {
            labels = "".to_string();
        } else {
            labels = "{".to_string();
            for label in &args.labels {
                // Split label arround the = sign
                let key_value = label.split_once('=');
                if key_value.is_none() {
                    panic!("Invalid label format: {}", label);
                }
                let key_value = key_value.unwrap();
                if labels.len() > 1 {
                    labels.push_str(",");
                }
                labels.push_str(format!("{}=\"{}\"", key_value.0, key_value.1).as_str());
            }
            labels.push_str("}");
        }
        let agent = ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(5))
            .timeout_write(Duration::from_secs(5))
            .timeout_connect(Duration::from_secs(5))
            .build();

        Self {
            labels,
            channel,
            agent,
            url: args.url.clone(),
        }
    }

    // Perform a HTTP POST request to the server
    fn send_payload(&self, payload: &str) {
        let res = self
            .agent
            .post(&self.url)
            .set(
                "Content-Type",
                "application/openmetrics-text; version=1.0.0; charset=utf-8",
            )
            .send_string(&payload);

        match res {
            Ok(res) => {
                if res.status() < 200 || res.status() >= 300 {
                    println!("Failed to send metrics: {:?}", res.status());
                }
            }
            Err(Error::Status(status, _)) => {
                println!("Failed to send metrics: {:?}", status);
            }
            Err(Error::Transport(err)) => {
                println!("Failed to send metrics: {:?}", err.kind());
            }
        }
    }

    fn send(&self) {
        let mut latest = HashMap::new();
        let mut last_sent = Instant::now();
        let mut interval = Duration::from_secs(2);
        loop {
            let metric = self.channel.recv_timeout(Duration::from_millis(1000));
            match metric {
                Ok(metric) => {
                    let dic = latest
                        .entry(metric.measurement.clone())
                        .or_insert_with(Vec::new);
                    dic.push(metric);
                }
                Err(_) => {}
            }

            let now = Instant::now();

            if now.duration_since(last_sent) > interval {
                let mut payload: String = String::new();
                let labels = &self.labels;
                for (measurement, metrics) in &latest {
                    for metric in metrics {
                        let metric_name = measurement.metric_name();
                        let metric_value = metric.value;
                        let metric_timestamp = metric.timestamp;
                        payload.push_str(
                            format!(
                                "{metric_name}{labels} {metric_value:.8} {metric_timestamp:.3}\n"
                            )
                            .as_str(),
                        );
                    }
                }
                last_sent = now;
                latest.clear();

                self.send_payload(&payload);
                interval = Duration::from_secs(5);
            }
        }
    }
}

fn main() {
    let args = Args::parse();
    let (sender, receiver) = std::sync::mpsc::channel::<Metric>();
    let serial_receiver = SerialReceiver {
        verbose: args.verbose,
        channel: sender,
    };
    let serial_sender = SerialSender::new(&args, receiver);
    std::thread::spawn(move || {
        serial_sender.send();
    });
    serial_receiver.monitor(&args);
}
