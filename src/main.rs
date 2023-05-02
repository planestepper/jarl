use clap::Parser;
use jarl::{Cli, Keeper};

use tokio::io::*;
use tokio::net::{ TcpListener, TcpStream };
use std::sync::{Arc, Mutex};


type TimeKeeper = Arc<Mutex<Keeper>>;


#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Cli::parse();
    
    let address = format!("{}:{}", args.ip, args.port);
    let listener = TcpListener::bind(address).await.unwrap();

    let keeper = Arc::new(Mutex::new(
        Keeper::new(args.requests, args.period)
    ));

    while let Ok((stream, _address)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, keeper.clone()));
    }
}

async fn handle_connection(mut stream: TcpStream, keeper: TimeKeeper) {
    let response = keeper.lock().unwrap().get_delay();
    stream.write_all((format!("{:.3}", response)).as_bytes()).await.unwrap();
}
