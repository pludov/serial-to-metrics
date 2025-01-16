use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(
        short,
        long,
        default_value = "/dev/ttyACM0",
        help = "Path to serial port"
    )]
    pub port: String,

    #[arg(short, long, default_value = "115200", help = "Baud rate")]
    pub rate: u32,

    #[arg(
        short,
        long,
        default_value = "http://localhost:8428/api/v1/import/prometheus",
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

    #[arg(long = "label")]
    pub labels: Vec<String>,
}
