use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};

#[tokio::main]
async fn main() {
    // We create a TCP listener on port 8080
    let listener = TcpListener::bind("localhost:8080").await.unwrap();

    loop {
        // We accept a connection from a client
        let (mut socket, _addr) = listener.accept().await.unwrap();

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
                // We read the client's message
                let bytes_read = reader.read_line(&mut line).await.unwrap();

                // If the client disconnects (0 bytes read), we break the loop
                if bytes_read == 0 {
                    break;
                }

                // We write the client's message to the socket
                writer.write_all(&line.as_bytes()).await.unwrap();

                // We clear the line string to read the next message
                line.clear();
            }
        });
    }
}
