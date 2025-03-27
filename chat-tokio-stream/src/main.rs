use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::broadcast,
};

#[tokio::main]
async fn main() {
    // We create a TCP listener on port 8080
    let listener = TcpListener::bind("localhost:8080").await.unwrap();

    // We create a broadcast channel to send messages to all clients connected to the server
    let (tx, _rx) = broadcast::channel(10);

    loop {
        // We accept a connection from a client
        let (mut socket, addr) = listener.accept().await.unwrap();

        let tx = tx.clone();
        let mut rx = tx.subscribe();

        // We are spawning a new task to handle the client's connection
        // In this way, we can handle multiple clients concurrently without I/O blocking
        tokio::spawn(async move {
            // We split the socket into a reader and writer, its useful to use the buffer reader
            let (reader, mut writer) = socket.split();

            // We create a buffer reader and a line string to read the client's message
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            // We loop until the client disconnects
            loop {
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        // If the client disconnects (0 bytes read), we break the loop
                        if result.unwrap() == 0 {
                            break;
                        }

                        // We send the client's message to all clients connected to the server
                        tx.send((line.clone(), addr)).unwrap();
                        // Then we clear the line string to read the next message
                        line.clear();
                    }
                    result = rx.recv() => {
                        // We receive the message from the broadcast channel
                        let (msg, _other_addr) = result.unwrap();

                        // We check if the message is not from the same client
                        // If it is, we don't send it to the client
                        if addr != _other_addr {
                            // We write the client's message to the socket
                            writer.write_all(&msg.as_bytes()).await.unwrap();
                        }
                    }
                }
            }
        });
    }
}
