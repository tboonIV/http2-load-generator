#![allow(dead_code)] // TODO remove

use crate::http_api::{send_request, HttpRequest};
use bytes::Bytes;
use h2::client::SendRequest;
use http::Method;
use serde_json::json;
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

async fn event_loop(
    mut client: SendRequest<Bytes>,
    mut rx: Receiver<Message>,
) -> Result<(), Box<dyn std::error::Error>> {
    while let Some(msg) = rx.recv().await {
        match msg {
            Message::NewEvent(event) => {
                println!("Received new event: {}", event.scenario);
            }
            Message::SendMessage(event, tx) => {
                println!("Sending message: {}", event.scenario);
                let http_request = HttpRequest {
                    uri: "http://127.0.0.1:8080/rsgateway/data/json/subscriber".to_string(),
                    method: Method::POST,
                    body: Some(json!({
                        "$": "MtxRequestSubscriberCreate",
                        "Name": "James Bond",
                        "FirstName": event.scenario.clone(),
                        "LastName": "Bond",
                        "ContactEmail": "james.bond@email.com"
                    })),
                };
                let future = send_request(&mut client, http_request).await.unwrap();
                println!("First Request sent");

                tokio::spawn(async move {
                    // tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    let response = future.await.expect("Failed to receive response");
                    println!("Response Status: {:?}", response.status);
                    println!("Response Body: {:?}", response.body);
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
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use h2::client;
//     use tokio::net::TcpStream;
//     use tokio::sync::mpsc::channel;
//
//     #[tokio::test]
//     async fn test_event_loop() {
//         let tcp = TcpStream::connect("localhost:8080").await.unwrap();
//         let (client, h2) = client::handshake(tcp).await.unwrap();
//
//         tokio::task::spawn(async move {
//             if let Err(e) = h2.await {
//                 log::error!("Unexpected error: {:?}", e);
//             }
//         });
//
//         let (tx, rx) = channel(32);
//         tokio::spawn(async move {
//             let _ = event_loop(client, rx).await.unwrap();
//         });
//
//         let (resp_tx, mut resp_rx) = channel(32);
//         tx.send(Message::SendMessage(
//             Event {
//                 scenario: "Message1".into(),
//             },
//             resp_tx.clone(),
//         ))
//         .await
//         .unwrap();
//
//         tx.send(Message::SendMessage(
//             Event {
//                 scenario: "Message2".into(),
//             },
//             resp_tx.clone(),
//         ))
//         .await
//         .unwrap();
//
//         // let (resp_tx_2, mut resp_rx_2) = channel(32);
//         println!("Waiting for response");
//         for i in 0..4 {
//             if let Some(response) = resp_rx.recv().await {
//                 println!("Response {} : {}", i, response.text);
//                 tx.send(Message::SendMessage(
//                     Event {
//                         scenario: "Message3".into(),
//                     },
//                     resp_tx.clone(),
//                 ))
//                 .await
//                 .unwrap();
//             }
//         }
//
//         // for i in 0..2 {
//         //     if let Some(response) = resp_rx_2.recv().await {
//         //         println!("Response2 {} : {}", i, response.text);
//         //     }
//         // }
//
//         // Wait for the event loop to Terminate
//         tx.send(Message::Terminate).await.unwrap();
//     }
// }
