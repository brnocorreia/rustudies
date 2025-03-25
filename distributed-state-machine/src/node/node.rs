use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum State {
    Init,
    Running,
    Stopped,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageType {
    Proposal,
    Acknowledgment,
    Commit,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub sender_id: u64,
    pub message_type: MessageType,
    pub proposed_state: State,
    pub proposal_id: String,
}

pub struct Node {
    pub id: u64,
    pub state: Arc<Mutex<State>>,
    pub peers: HashMap<u64, String>,
    pub address: String,
    pub tx: mpsc::Sender<Message>,
    pub proposal_acknowledgments: Arc<Mutex<HashMap<String, HashSet<u64>>>>,
}

impl Node {
    pub async fn send_message(&self, message: &Message, receiver_address: &str) -> io::Result<()> {
        let mut stream = TcpStream::connect(receiver_address).await?;
        let serialized_message = serde_json::to_vec(message)?;
        stream.write_all(&serialized_message).await?;
        Ok(())
    }

    pub async fn broadcast_proposal(&self, proposed_state: State) {
        let proposal_id = Uuid::new_v4().to_string();
        let message = Message {
            sender_id: self.id,
            message_type: MessageType::Proposal,
            proposed_state,
            proposal_id: proposal_id.clone(),
        };

        let mut proposal_acknowledgments = self.proposal_acknowledgments.lock().await;
        proposal_acknowledgments.insert(proposal_id.clone(), HashSet::new());

        for address in self.peers.values() {
            if let Err(e) = self.send_message(&message, address).await {
                eprintln!("Failed to send message to {}: {:?}", address, e);
            }
        }
        self.wait_for_acknowledgments(proposal_id).await;
    }

    pub async fn listen(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.address).await?;
        println!("Node {} listening on {}", self.id, self.address);

        loop {
            let (mut socket, _) = listener.accept().await?;
            let tx = self.tx.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    match socket.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if let Ok(message) = serde_json::from_slice::<Message>(&buf[..n]) {
                                tx.send(message)
                                    .await
                                    .expect("Failed to send message to channel");
                            }
                        }
                        Err(e) => {
                            eprintln!("Error reading message: {:?}", e);
                            break;
                        }
                    }
                }
            });
        }
    }

    pub async fn handle_incoming_messages(&self, mut rx: mpsc::Receiver<Message>) {
        while let Some(message) = rx.recv().await {
            match message.message_type {
                MessageType::Proposal => {
                    // Handle proposal: Send acknowledgment back
                }
                MessageType::Acknowledgment => {
                    // Track acknowledgment and check for consensus
                }
                MessageType::Commit => {
                    // Commit the proposed state change
                }
            }
        }
    }

    async fn wait_for_acknowledgments(&self, proposal_id: String) {
        let majority = (self.peers.len() / 2) + 1;

        loop {
            let ack_count = {
                let acks = self.proposal_acknowledgments.lock().await;
                acks.get(&proposal_id).map(|acks| acks.len()).unwrap_or(0)
            };
            if ack_count >= majority {
                // Commit the proposal
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
