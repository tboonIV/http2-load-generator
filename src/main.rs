use chrono::Local;
use h2::client;
use http::Request;
use std::error::Error;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Instant;
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

    // let mut counter: u32 = 0;
    let counter = Arc::new(Mutex::new(0u32));
    let total_iteration = 50;
    let start = Instant::now();

    for _ in 0..total_iteration {
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
        log::debug!("Request sent");

        let counter = Arc::clone(&counter);
        tokio::task::spawn(async move {
            let result: Result<(), Box<dyn std::error::Error>> = (async {
                let response = response.await?;
                log::trace!("Response: {:?}", response);

                let mut body = response.into_body();
                while let Some(chunk) = body.data().await {
                    log::debug!("Response Body: {:?}", chunk?);
                }

                let mut counter = counter.lock().unwrap();
                *counter += 1;
                Ok(())
            })
            .await;

            if let Err(e) = result {
                log::error!("Error processing response: {}", e);
            }
        });
    }

    while *counter.lock().unwrap() < total_iteration {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    let elapsed = start.elapsed();
    let elapsed_s = elapsed.as_secs() as f64 + elapsed.subsec_millis() as f64 / 1000.0;
    let tps = total_iteration as f64 / (elapsed.as_micros() as f64 / 1_000_000.0);
    log::info!("Elapsed: {:.3}s , {} requests per second", elapsed_s, tps,);

    Ok(())
}
