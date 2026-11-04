use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

async fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0u8; 512];

    loop {
        let bytes_read = match stream.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        };

        if let Err(e) = stream.write_all(&buffer[..bytes_read]).await {
            eprintln!("Write error: {}", e);
            break;
        }
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:9001").await.unwrap();

    println!("Server listening on port 9001");

    loop {
        match listener.accept().await {
            Ok((stream, address)) => {
                println!("New connection: {}", address);

                tokio::spawn(async move {
                    handle_connection(stream).await;
                });
            }
            Err(e) => {
                println!("Connection failed: {}", e);
            }
        }
    }
}
