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
use tokio::time;
use tokio::time::Duration;

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

    let parallel = 16;
    for _ in 0..parallel {
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                let _report = runner().await.unwrap();
                // tx.send(report).await.unwrap();
            });
        });
    }

    Ok(())
}

pub async fn runner() -> Result<(), Box<dyn Error>> {
    let tcp = TcpStream::connect("127.0.0.1:8080").await?;
    let (mut client, h2) = client::handshake(tcp).await?;

    tokio::task::spawn(async move {
        if let Err(e) = h2.await {
            log::error!("Unexpected error: {:?}", e);
        }
    });

    let target_tps = 1000;
    let duration_s = 10;
    let param = RunParameter::new(target_tps, duration_s);
    let total_iterations = param.total_requests as f64 / param.batch_size as f64;
    let total_iterations = total_iterations.ceil() as u32;
    let total_requests = total_iterations * param.batch_size;

    log::info!(
        "Sending total request {}, total_iteration {}, with {} TPS, batch size {}, interval {}",
        total_requests,
        total_iterations,
        param.target_tps,
        param.batch_size,
        param.interval.as_secs_f64()
    );

    let counter = Arc::new(Mutex::new(0u32));
    let start = Instant::now();

    let mut interval = time::interval(param.interval);
    for _ in 0..total_iterations {
        interval.tick().await;

        for _ in 0..param.batch_size {
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
    }

    while *counter.lock().unwrap() < total_requests {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    let total_count = *counter.lock().unwrap();
    let elapsed = start.elapsed();
    let elapsed_s = elapsed.as_secs() as f64 + elapsed.subsec_millis() as f64 / 1000.0;
    let tps = total_count as f64 / (elapsed.as_micros() as f64 / 1_000_000.0);
    log::info!("Elapsed: {:.3}s , {} requests per second", elapsed_s, tps,);

    Ok(())
}

#[derive(Clone)]
pub struct RunParameter {
    pub target_tps: u32,
    pub batch_size: u32,
    pub interval: Duration,
    pub total_requests: u32,
}

impl RunParameter {
    pub fn new(call_rate: u32, duration_s: u32) -> RunParameter {
        let rps = call_rate;
        let batch_size = (rps / 200) as u32;
        let batch_size = if batch_size == 0 { 1 } else { batch_size };
        let batches_per_second = rps as f64 / batch_size as f64;
        let interval = Duration::from_secs_f64(1.0 / batches_per_second);
        let total_requests = rps * duration_s;

        RunParameter {
            target_tps: rps,
            batch_size,
            interval,
            total_requests,
        }
    }
}
