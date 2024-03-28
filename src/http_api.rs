use bytes::Bytes;
use h2::client::ResponseFuture;
use h2::client::SendRequest;
use h2::SendStream;
use http::Method;
use http::Request;
use http::StatusCode;
use serde_json::json;
use std::error::Error;
use std::time::Instant;
use tokio::task::JoinHandle;
use tokio::time::Duration;

pub struct HttpRequest {
    pub uri: String,
    pub method: Method,
    pub body: serde_json::Value,
}

pub struct HttpResponse {
    pub status: StatusCode,
    pub body: serde_json::Value,
    pub request_start: Instant,
    pub retry_count: u8,
}

pub async fn send_request(
    client: &mut SendRequest<Bytes>,
    http_request: HttpRequest,
) -> Result<JoinHandle<HttpResponse>, Box<dyn Error>> {
    let request = Request::builder()
        .uri(http_request.uri)
        .method(http_request.method)
        .body(())?;

    let (response, mut stream, retry_count, request_start) =
        send_request_with_retries(client, &request).await?;

    let request_body = serde_json::to_string(&http_request.body)?;

    stream.send_data(request_body.into(), true)?;
    log::debug!("Request sent");

    let result = tokio::task::spawn(async move {
        let result: Result<HttpResponse, Box<dyn std::error::Error>> = (async {
            let response = response.await?;
            log::trace!("Response: {:?}", response);

            let status = response.status();
            let mut body = response.into_body();
            let mut response_body = String::new();
            while let Some(chunk) = body.data().await {
                response_body.push_str(&String::from_utf8(chunk?.clone().to_vec())?);
            }

            Ok(HttpResponse {
                status,
                body: serde_json::from_str(&response_body)?,
                request_start,
                retry_count,
            })
        })
        .await;

        match result {
            Ok(ok) => ok,
            Err(_e) => {
                // log::error!("Error processing response: {}", e);
                // TODO need better error handling
                HttpResponse {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    body: json!({}),
                    request_start,
                    retry_count,
                }
            }
        }
    });

    Ok(result)
}

async fn send_request_with_retries(
    client: &mut SendRequest<Bytes>,
    request: &Request<()>,
) -> Result<(ResponseFuture, SendStream<Bytes>, u8, Instant), Box<dyn Error>> {
    let retry_delay = Duration::from_millis(1);
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
                if retry_count >= 1 {
                    // log::error!("Maximum retries reached. Aborting.");
                    return Err(Box::new(e));
                }
                tokio::time::sleep(retry_delay).await;
            }
        }
    }
}
