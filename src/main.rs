use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:9001").unwrap();
    println!("Server listening on port 9001");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap())
            }
            Err(e) => {
                println!("Connection failed: {}", e)
            }
        }
    }
}
