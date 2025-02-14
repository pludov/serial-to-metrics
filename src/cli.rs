use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(
        short,
        long,
        default_value = "/dev/ttyACM0",
        help = "Path to serial port",
        env = "SERIAL_PORT"
    )]
    pub port: String,

    #[arg(short, long, default_value = "115200", help = "Baud rate")]
    pub rate: u32,

    #[arg(
        short,
        long,
        default_value = "http://localhost:8428/api/v1/import/prometheus",
        env = "METRIC_SERVER",
        help = "Target URL for metric server"
    )]
    pub url: String,

    #[arg(
        short,
        long,
        default_value = "10000",
        help = "Delay between updates in ms"
    )]
    pub delay: u64,

    #[arg(
        short,
        long,
        default_value = "5000",
        help = "Timeout for http requests in ms"
    )]
    pub timeout: u64,

    #[arg(
        long = "data-min-interval",
        default_value = "250",
        help = "Minimum idle interval expected between data - in ms"
    )]
    pub data_min_interval: u64,

    #[arg(
        long,
        default_value = "false",
        help = "Activate dump of raw data to stdout"
    )]
    pub verbose: bool,

    #[arg(long = "label")]
    pub labels: Vec<String>,
}
