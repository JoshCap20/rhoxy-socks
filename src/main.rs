use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "localhost")]
    host: String,

    #[arg(short, long, default_value = "8080", help = "Port to listen on")]
    port: u16,

    #[arg(long, help = "Enable debug logging")]
    verbose: bool,
}

fn main() {
    println!("Hello, world!");
}
