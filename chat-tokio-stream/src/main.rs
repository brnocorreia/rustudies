use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{broadcast, Mutex},
};

#[tokio::main]
async fn main() {
    // We create a TCP listener on port 8080
    let listener = TcpListener::bind("localhost:8080").await.unwrap();

    // We create a broadcast channel to send messages to all clients connected to the server
    let (tx, _rx) = broadcast::channel(10);

    // We create a mutex to store the users connected to the server
    let users = Arc::new(Mutex::new(HashMap::<SocketAddr, String>::new()));

    let reserved_nicknames = ["admin", "server", "system"];

    loop {
        // We accept a connection from a client
        let (mut socket, addr) = listener.accept().await.unwrap();
        let tx = tx.clone();
        let mut rx = tx.subscribe();
        let users = Arc::clone(&users);

        tokio::spawn(async move {
            // We split the socket into a reader and writer, its useful to use the buffer reader
            let (reader, mut writer) = socket.split();

            // We create a buffer reader and a line string to read the client's message
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            let mut nickname = String::new();
            let mut is_valid_nickname = false;

            while !is_valid_nickname {
                writer
                    .write_all(b"Please enter your nickname: ")
                    .await
                    .unwrap();

                if reader.read_line(&mut line).await.unwrap() == 0 {
                    return;
                }

                nickname = line.trim().to_string();
                line.clear();

                let nickname_lower = nickname.to_lowercase();

                if reserved_nicknames
                    .iter()
                    .any(|&n| n.to_lowercase() == nickname_lower)
                {
                    writer.write_all(b"This nickname is reserved for the server! Please choose another one.\n").await.unwrap();
                    continue;
                }

                // Check if nickname is already in use
                let nickname_in_use = {
                    let users_lock = users.lock().await;
                    users_lock
                        .values()
                        .any(|n| n.to_lowercase() == nickname_lower)
                };

                if nickname_in_use {
                    writer
                        .write_all(b"This nickname is already in use! Please choose another one.\n")
                        .await
                        .unwrap();
                    continue;
                }

                // Nickname is valid
                is_valid_nickname = true;
            }

            {
                let mut users_lock = users.lock().await;
                users_lock.insert(addr, nickname.clone());
            }

            let connect_msg = format!(
                "[SERVER]: A new user [{}] connected! Be welcome!\n",
                nickname
            );
            tx.send((connect_msg, addr)).unwrap();

            // We loop until the client disconnects
            loop {
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        // If the client disconnects (0 bytes read), we break the loop
                        if result.unwrap() == 0 {
                            let nickname = {
                                let users_lock = users.lock().await;
                                users_lock.get(&addr).cloned().unwrap_or_else(|| addr.to_string())
                            };

                            let disconnect_msg = format!("[SERVER]: The user [{}] disconnected!\n", nickname);
                            tx.send((disconnect_msg, addr)).unwrap();

                            let mut users_lock = users.lock().await;
                            users_lock.remove(&addr);
                            break;
                        }

                        let nickname = {
                            let users_lock = users.lock().await;
                            users_lock.get(&addr).cloned().unwrap_or_else(|| addr.to_string())
                        };

                        let message = format!("[{}]: {}", nickname, line);
                        // We send the client's message to all clients connected to the server
                        tx.send((message, addr)).unwrap();
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
