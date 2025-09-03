use clap::Parser;
use tokio::net::{TcpListener, TcpStream};

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

    start_server(args).await;
}

async fn start_server(args: Args) {
    let listener = TcpListener::bind(format!("{}:{}", args.host, args.port))
        .await
        .expect("Failed to bind to address");

    loop {
        let (socket, _) = listener
            .accept()
            .await
            .expect("Failed to accept connection");
        tokio::spawn(handle_connection(socket));
    }
}

async fn handle_connection(socket: TcpStream) {
    // TODO: implement connection handling
}
