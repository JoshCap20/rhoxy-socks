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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.verbose {
        // TODO: init tracing subscriber with debug level
    } else {
        // TODO: init tracing subscriber with info level
    }
}
