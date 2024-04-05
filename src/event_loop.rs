#![allow(dead_code)] // TODO remove

use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::{Receiver, Sender};

struct Event {
    scenario: String,
}

struct Response {
    text: String,
}

enum Message {
    NewEvent(Event),
    SendMessage(Event, Sender<Response>),
    Terminate,
}

async fn event_loop(mut rx: Receiver<Message>) {
    while let Some(msg) = rx.recv().await {
        match msg {
            Message::NewEvent(event) => {
                println!("Received new event: {}", event.scenario);
            }
            Message::SendMessage(event, tx) => {
                println!("Sending message: {}", event.scenario);

                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    let response = Response {
                        text: "Message received".to_string(),
                    };
                    println!("Received response: {}", response.text);
                    if tx.send(response).await.is_err() {
                        eprintln!("Failed to send response");
                    }
                });
            }
            Message::Terminate => {
                println!("Terminating event loop");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_loop() {
        let (tx, rx) = channel(32);
        tokio::spawn(async move {
            event_loop(rx).await;
        });

        let (resp_tx, mut resp_rx) = channel(32);
        tx.send(Message::SendMessage(
            Event {
                scenario: "Message1".into(),
            },
            resp_tx.clone(),
        ))
        .await
        .unwrap();

        tx.send(Message::SendMessage(
            Event {
                scenario: "Message2".into(),
            },
            resp_tx,
        ))
        .await
        .unwrap();

        println!("Waiting for response");
        for i in 0..2 {
            if let Some(response) = resp_rx.recv().await {
                println!("Response {} : {}", i, response.text);
            }
        }

        // Wait for the event loop to Terminate
        tx.send(Message::Terminate).await.unwrap();
    }
}
