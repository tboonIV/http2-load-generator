use chrono::Local;
use h2::client;
use http::Request;
use std::error::Error;
use std::io::Write;
use std::thread;
use tokio::net::TcpStream;

use serde_json::json;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::new()
        .format(|buf, record| {
            let now = Local::now();
            let thread = thread::current();
            let thread_name = thread.name().unwrap_or("unnamed");
            let thread_id = thread.id();

            writeln!(
                buf,
                "{} [{}] {:?} - ({}): {}",
                now.format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                thread_id,
                thread_name,
                record.args()
            )
        })
        .filter(None, log::LevelFilter::Info)
        .init();

    fetch_url().await?;

    // TODO properly wait for the response
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    Ok(())
}

pub async fn fetch_url() -> Result<(), Box<dyn Error>> {
    let tcp = TcpStream::connect("127.0.0.1:8080").await?;
    let (mut client, h2) = client::handshake(tcp).await?;

    tokio::task::spawn(async move {
        if let Err(e) = h2.await {
            log::error!("Unexpected error: {:?}", e);
        }
    });

    for _ in 0..3 {
        let request = Request::builder()
            .uri("http://127.0.0.1:8080/rsgateway/data/json/subscriber")
            .method("POST")
            .body(())?;

        let (response, mut stream) = client.send_request(request, false)?;

        let payload = json!({
            "$": "MtxRequestSubscriberCreate",
            "Name": "James Bond",
            "FirstName": "James",
            "LastName": "Bond",
            "ContactEmail": "james.bond@email.com"
        });
        let request_body = serde_json::to_string(&payload)?;

        stream.send_data(request_body.into(), true)?;
        log::info!("Request sent");

        tokio::task::spawn(async move {
            let result: Result<(), Box<dyn std::error::Error>> = (async {
                let response = response.await?;
                log::debug!("Response: {:?}", response);

                let mut body = response.into_body();
                while let Some(chunk) = body.data().await {
                    log::info!("Response Body: {:?}", chunk?);
                }
                Ok(())
            })
            .await;

            if let Err(e) = result {
                log::error!("Error processing response: {}", e);
            }
        });
    }
    Ok(())
}
