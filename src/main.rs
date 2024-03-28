use bytes::Bytes;
use chrono::Local;
use h2::client;
use h2::client::ResponseFuture;
use h2::client::SendRequest;
use h2::SendStream;
use http::Method;
use http::Request;
use std::error::Error;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
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
        .filter(None, log::LevelFilter::Debug)
        .init();

    let (tx, mut rx) = mpsc::channel(8);

    let target_tps = 1;
    let duration_s = 5;
    let parallel = 1;
    let batch_size = Some(2);
    // let batch_size = None; // If None, it will be calculated based on target_tps automatically

    for _ in 0..parallel {
        let tx = tx.clone();
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                let report = runner(target_tps, duration_s, batch_size).await.unwrap();
                tx.send(report).await.unwrap();
            });
        });
    }

    drop(tx);

    let mut total_tps = 0f64;
    let mut elapsed = tokio::time::Duration::from_secs(0);
    let mut total_success = 0;
    let mut total_error = 0;
    while let Some(report) = rx.recv().await {
        total_tps += report.tps;
        elapsed = elapsed.max(report.elapsed);
        total_success += report.success_count;
        total_error += report.error_count;
    }
    let elapsed_s = elapsed.as_secs() as f64 + elapsed.subsec_millis() as f64 / 1000.0;

    log::info!("Total TPS: {:.3}", total_tps);
    log::info!("Elapsed: {:.3}", elapsed_s);
    log::info!(
        "Success Rate: {:.3}%",
        total_success as f64 / (total_success + total_error) as f64 * 100.0
    );

    Ok(())
}

pub async fn runner(
    target_tps: u32,
    duration_s: u32,
    batch_size: Option<u32>,
) -> Result<RunReport, Box<dyn Error>> {
    let tcp = TcpStream::connect("127.0.0.1:8080").await?;
    let (mut client, h2) = client::handshake(tcp).await?;

    tokio::task::spawn(async move {
        if let Err(e) = h2.await {
            log::error!("Unexpected error: {:?}", e);
        }
    });

    let param = RunParameter::new(target_tps, duration_s, batch_size);
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

    let success_counter = Arc::new(Mutex::new(0u32));
    let error_counter = Arc::new(Mutex::new(0u32));
    let start = Instant::now();
    let total_rtt = Arc::new(Mutex::new(Duration::from_secs(0)));
    let mut total_retry = 0;

    let mut interval = time::interval(param.interval);
    for _ in 0..total_iterations {
        interval.tick().await;

        for _ in 0..param.batch_size {
            let http_request = HttpRequest {
                uri: "http://127.0.0.1:8080/rsgateway/data/json/subscriber".to_string(),
                method: Method::POST,
                body: json!({
                    "$": "MtxRequestSubscriberCreate",
                    "Name": "James Bond",
                    "FirstName": "James",
                    "LastName": "Bond",
                    "ContactEmail": "james.bond@email.com"
                }),
            };

            send_request(&mut client, http_request).await?;
            let mut success_counter = success_counter.lock().unwrap();
            *success_counter += 1;

            //     let request = Request::builder()
            //         .uri("http://127.0.0.1:8080/rsgateway/data/json/subscriber")
            //         .method("POST")
            //         .body(())?;
            //
            //     let (response, mut stream, _retry_count, request_start) =
            //         match send_request_with_retries(&mut client, &request).await {
            //             Ok(ok) => ok,
            //             Err(_e) => {
            //                 // Back pressure
            //                 tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            //                 let mut error_counter = error_counter.lock().unwrap();
            //                 *error_counter += 1;
            //                 continue;
            //             }
            //         };
            //     total_retry += _retry_count as u32;
            //
            //     let payload = json!({
            //         "$": "MtxRequestSubscriberCreate",
            //         "Name": "James Bond",
            //         "FirstName": "James",
            //         "LastName": "Bond",
            //         "ContactEmail": "james.bond@email.com"
            //     });
            //     let request_body = serde_json::to_string(&payload)?;
            //
            //     stream.send_data(request_body.into(), true)?;
            //     log::debug!("Request sent");
            //
            //     let success_counter = Arc::clone(&success_counter);
            //     let error_counter = Arc::clone(&error_counter);
            //     let total_rtt = Arc::clone(&total_rtt);
            //     tokio::task::spawn(async move {
            //         let result: Result<(), Box<dyn std::error::Error>> = (async {
            //             let response = response.await?;
            //             log::trace!("Response: {:?}", response);
            //
            //             let mut body = response.into_body();
            //             while let Some(chunk) = body.data().await {
            //                 log::debug!("Response Body: {:?}", chunk?);
            //             }
            //
            //             Ok(())
            //         })
            //         .await;
            //
            //         if let Err(e) = result {
            //             log::error!("Error processing response: {}", e);
            //             let mut error_counter = error_counter.lock().unwrap();
            //             *error_counter += 1;
            //         } else {
            //             let round_trip_time = request_start.elapsed();
            //             let mut total_rtt = total_rtt.lock().unwrap();
            //             *total_rtt += round_trip_time;
            //             let mut success_counter = success_counter.lock().unwrap();
            //             *success_counter += 1;
            //         }
            //     });
        }
    }

    while *success_counter.lock().unwrap() + *error_counter.lock().unwrap() < total_requests {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    let success_count = *success_counter.lock().unwrap();
    let error_count = *error_counter.lock().unwrap();
    let total_count = success_count + error_count;
    let total_rtt = *total_rtt.lock().unwrap();

    let elapsed = start.elapsed();
    let elapsed_s = elapsed.as_secs() as f64 + elapsed.subsec_millis() as f64 / 1000.0;
    let tps = success_count as f64 / (elapsed.as_micros() as f64 / 1_000_000.0);
    let avg_rtt = total_rtt.as_millis() as f64 / success_count as f64;

    log::info!(
        "Elapsed: {:.3}s, TPS: {:.3}, AVG: {:.3}ms, ERR: ({}/{}), RETRY: {}",
        elapsed_s,
        tps,
        avg_rtt,
        error_count,
        total_count,
        total_retry
    );

    let report = RunReport {
        tps,
        elapsed,
        success_count,
        error_count,
    };
    Ok(report)
}

pub struct HttpRequest {
    pub uri: String,
    pub method: Method,
    pub body: serde_json::Value,
}

async fn send_request(
    client: &mut SendRequest<Bytes>,
    http_request: HttpRequest,
) -> Result<(), Box<dyn Error>> {
    let request = Request::builder()
        .uri(http_request.uri)
        .method(http_request.method)
        .body(())?;

    let (response, mut stream, _retry_count, _request_start) =
        send_request_with_retries(client, &request).await?;

    let request_body = serde_json::to_string(&http_request.body)?;

    stream.send_data(request_body.into(), true)?;
    log::debug!("Request sent");

    tokio::task::spawn(async move {
        let result: Result<(), Box<dyn std::error::Error>> = (async {
            let response = response.await?;
            log::trace!("Response: {:?}", response);

            let mut body = response.into_body();
            while let Some(chunk) = body.data().await {
                log::debug!("Response Body: {:?}", chunk?);
            }

            Ok(())
        })
        .await;

        if let Err(e) = result {
            log::error!("Error processing response: {}", e);
        }
    });

    Ok(())
}

async fn send_request_with_retries(
    client: &mut SendRequest<Bytes>,
    request: &Request<()>,
) -> Result<(ResponseFuture, SendStream<Bytes>, u8, Instant), Box<dyn Error>> {
    let retry_delay = Duration::from_millis(50);
    let mut retry_count = 0;

    loop {
        let start_time = Instant::now();
        match client.send_request(request.clone(), false) {
            Ok((response, stream)) => {
                return Ok((response, stream, retry_count, start_time));
            }
            Err(e) => {
                // log::warn!("Error sending request: {}", e);
                retry_count += 1;
                if retry_count >= 3 {
                    // log::error!("Maximum retries reached. Aborting.");
                    return Err(Box::new(e));
                }
                tokio::time::sleep(retry_delay).await;
            }
        }
    }
}

#[derive(Clone)]
pub struct RunParameter {
    pub target_tps: u32,
    pub batch_size: u32,
    pub interval: Duration,
    pub total_requests: u32,
}

impl RunParameter {
    pub fn new(call_rate: u32, duration_s: u32, batch_size: Option<u32>) -> RunParameter {
        let rps = call_rate;
        let batch_size = if let Some(batch_size) = batch_size {
            batch_size
        } else {
            let batch_size = (rps / 200) as u32;
            let batch_size = if batch_size == 0 { 1 } else { batch_size };
            batch_size
        };
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

pub struct RunReport {
    pub tps: f64,
    pub elapsed: Duration,
    pub success_count: u32,
    pub error_count: u32,
}
