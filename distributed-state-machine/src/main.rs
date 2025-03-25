use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::node::node::{Message, MessageType, Node, State};
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    sync::{mpsc, Mutex},
};
use uuid::Uuid;

pub mod node;

async fn simulate_client_interaction() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    let proposal_message = Message {
        sender_id: 999,
        message_type: MessageType::Proposal,
        proposed_state: State::Running,
        proposal_id: Uuid::new_v4().to_string(),
    };

    let serialized_message = serde_json::to_vec(&proposal_message)?;
    stream.write_all(&serialized_message).await?;
    println!("Simulated client sent proposal to Node 1");
    Ok(())
}

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(State::Init));
    let proposal_acknowledgments = Arc::new(Mutex::new(HashMap::new()));

    let (tx1, rx1) = mpsc::channel(32);
    let node1 = Arc::new(Node {
        id: 1,
        state: state.clone(),
        peers: HashMap::from([(2, "0.0.0.0:8081".to_string())]),
        address: "0.0.0.0:8080".to_string(),
        tx: tx1,
        proposal_acknowledgments: proposal_acknowledgments.clone(),
    });

    let (tx2, rx2) = mpsc::channel(32);
    let node2 = Arc::new(Node {
        id: 2,
        state: state.clone(),
        peers: HashMap::from([(1, "0.0.0.0:8080".to_string())]),
        address: "0.0.0.0:8081".to_string(),
        tx: tx2,
        proposal_acknowledgments,
    });

    let node1_clone_for_messages = Arc::clone(&node1);
    tokio::spawn(async move {
        node1_clone_for_messages.handle_incoming_messages(rx1).await;
    });

    let node2_clone_for_messages = Arc::clone(&node2);
    tokio::spawn(async move {
        node2_clone_for_messages.handle_incoming_messages(rx2).await;
    });

    // Listen for incoming connections
    let node1_clone_for_listen = Arc::clone(&node1);
    tokio::spawn(async move {
        node1_clone_for_listen
            .listen()
            .await
            .expect("Node 1 failed to listen");
    });

    let node2_clone_for_listen = Arc::clone(&node2);
    tokio::spawn(async move {
        node2_clone_for_listen
            .listen()
            .await
            .expect("Node 2 failed to listen");
    });

    // Ensure the servers have time to start up
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Use the original `node1` Arc to broadcast a proposal
    node1.broadcast_proposal(State::Running).await;

    // Start the simulation after a short delay to ensure nodes are listening
    tokio::time::sleep(Duration::from_secs(2)).await;
    if let Err(e) = simulate_client_interaction().await {
        eprintln!("Failed to simulate client: {:?}", e);
    }
}
