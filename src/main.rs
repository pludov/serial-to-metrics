use std::{
    collections::HashMap,
    io::{ErrorKind, Read},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use cli::Args;

mod cli;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum Measurement {
    P,
    I,
    U,
}

#[derive(PartialEq, Clone, Debug)]
struct Metric {
    measurement: Measurement,
    timestamp: u64,
    value: f64,
}

struct SerialReceiver {
    channel: std::sync::mpsc::Sender<Metric>,
}

impl SerialReceiver {
    fn parse_line(&self, line: &[u8]) -> Result<(Measurement, f64), &str> {
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

        Ok((measurement, value))
        //
    }

    fn handle_line(&self, line: &[u8]) {
        let parsed = self.parse_line(line);
        if parsed.is_err() {
            println!(
                "Could not parse line: {} !!!!!!!!!!!!!!!!!",
                parsed.err().unwrap()
            );
            return;
        }
        let (measurement, value) = parsed.unwrap();

        self.channel
            .send(Metric {
                measurement,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Epoch overflow ?")
                    .as_secs(),
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
            } else {
                i += 1;
            }
        }
    }

    fn monitor(&self, args: &Args) {
        let mut timed_out = false;
        // open the serial device
        loop {
            let port_open = serialport::new(&args.port, args.rate)
                .timeout(Duration::from_millis(500))
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
                        if timed_out {
                            timed_out = false;
                            self.consume_lines(&mut line_buf, &mut line_pos, bytes);
                            if line_pos == 256 {
                                println!("Buffer full, giving up...");
                                break;
                            }
                        } else {
                            println!("Discarding data");
                        }
                    }
                    Err(err) => match err.kind() {
                        ErrorKind::TimedOut => {
                            timed_out = true;
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
    channel: std::sync::mpsc::Receiver<Metric>,
}

impl SerialSender {
    fn send(&self) {
        let mut latest = HashMap::new();
        let mut last_sent = Instant::now();
        let mut interval = Duration::from_secs(2);
        loop {
            let metric = self.channel.recv_timeout(Duration::from_millis(1000));
            match metric {
                Ok(metric) => {
                    latest.insert(metric.measurement.clone(), metric);
                }
                Err(_) => {}
            }

            let now = Instant::now();

            if now.duration_since(last_sent) > interval {
                println!("Sending metrics {:?}", latest);
                for (measurement, metric) in &latest {
                    println!("{:?}: {:?}", measurement, metric.value);
                }
                last_sent = now;
                latest.clear();
                interval = Duration::from_secs(5);
            }
        }
    }
}

fn main() {
    let args = Args::parse();
    let (sender, receiver) = std::sync::mpsc::channel::<Metric>();
    let serial_receiver = SerialReceiver { channel: sender };
    let serial_sender = SerialSender { channel: receiver };
    std::thread::spawn(move || {
        serial_sender.send();
    });
    serial_receiver.monitor(&args);
}
