use std::time::Duration;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

async fn client() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to the server...");
    let addr = "127.0.0.1:9001";

    let mut stream = timeout(Duration::from_secs(5), TcpStream::connect(addr)).await??;

    let msg = b"Hello, Echo Server!";
    stream.write_all(msg).await?;

    println!("Sent: {}", String::from_utf8_lossy(msg));

    let mut buffer = [0u8; 1024];
    let bytes_read = stream.read(&mut buffer).await?;

    let response = &buffer[..bytes_read];

    println!("Received: {}", String::from_utf8_lossy(response));

    assert_eq!(response, msg, "Response does not match the sent message");

    println!("Verification passed!");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    client().await
}
