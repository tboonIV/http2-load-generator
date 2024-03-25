use chrono::Local;
use h2::client;
use http::Request;
use std::error::Error;
use std::io::Write;
use std::thread;
use tokio::net::TcpStream;

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
    let _ = env_logger::try_init();

    let tcp = TcpStream::connect("127.0.0.1:8080").await?;
    let (mut client, h2) = client::handshake(tcp).await?;

    tokio::task::spawn(async move {
        if let Err(e) = h2.await {
            log::error!("Unexpected error: {:?}", e);
        }
    });

    for _ in 0..3 {
        let request = Request::builder()
            .uri("https://127.0.0.1:8080/test")
            .method("POST")
            .body(())
            .unwrap();

        let (response, mut stream) = client.send_request(request, false).unwrap();

        let request_body = b"{\"foo\": \"value1\"}".to_vec();
        stream.send_data(request_body.into(), true).unwrap();
        log::info!("Request sent");

        tokio::task::spawn(async move {
            let response = response.await.unwrap();
            log::debug!("Response: {:?}", response);

            let mut body = response.into_body();
            while let Some(chunk) = body.data().await {
                log::info!("Response Body: {:?}", chunk.unwrap());
            }
        });
    }

    // TODO properly wait for the response
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    Ok(())
}
