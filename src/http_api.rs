use bytes::Bytes;
use h2::client::ResponseFuture;
use h2::client::SendRequest;
use h2::SendStream;
use http::Method;
use http::Request;
use http::StatusCode;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::time::Instant;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio::time::Duration;

pub struct HttpRequest {
    pub uri: String,
    pub method: Method,
    pub headers: Option<Vec<HashMap<String, String>>>,
    pub body: Option<serde_json::Value>,
    pub timeout: Duration,
}

pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: http::HeaderMap,
    pub body: Option<serde_json::Value>,
    pub request_start: Instant,
    pub retry_count: u8,
}

#[derive(Debug)]
pub struct HttpError(String);

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "HttpError: {}", self.0)
    }
}

impl Error for HttpError {}

impl From<HttpError> for Box<dyn Error + Send> {
    fn from(err: HttpError) -> Self {
        Box::new(err)
    }
}

pub async fn send_request(
    client: &mut SendRequest<Bytes>,
    http_request: HttpRequest,
) -> Result<JoinHandle<Result<HttpResponse, HttpError>>, Box<dyn Error>> {
    log::debug!(
        "Sending request {} {}",
        http_request.method,
        http_request.uri
    );

    let mut request_builder = Request::builder()
        .uri(http_request.uri)
        .method(http_request.method);
    if let Some(headers) = http_request.headers {
        for header in headers {
            for (k, v) in header {
                request_builder = request_builder.header(k, v);
            }
        }
    }
    let request = request_builder.body(())?;

    let (response, mut stream, retry_count, request_start) =
        send_request_with_retries(client, &request).await?;

    let request_body = serde_json::to_string(&http_request.body)?;
    log::debug!("Request body: {}", request_body);

    stream.send_data(request_body.into(), true)?;
    // log::debug!("Request sent");

    let result: tokio::task::JoinHandle<Result<HttpResponse, HttpError>> =
        tokio::task::spawn(async move {
            let result: Result<HttpResponse, Box<dyn std::error::Error>> = (async {
                // let response = response.await?;
                let response = timeout(http_request.timeout, response).await??;
                log::trace!("Response: {:?}", response);

                // Headers
                let headers = response.headers().clone();

                // Status
                let status = response.status();

                // Body
                let mut body = response.into_body();
                let mut response_body = String::new();

                loop {
                    let chunk = timeout(http_request.timeout, body.data()).await;
                    if let Some(chunk) = chunk? {
                        response_body.push_str(&String::from_utf8(chunk?.clone().to_vec())?);
                    } else {
                        // no more data chunk, exit loop
                        break;
                    }
                }

                let body = parse_json_body(&response_body, &headers);

                Ok(HttpResponse {
                    status,
                    headers,
                    body,
                    request_start,
                    retry_count,
                })
            })
            .await;

            result.map_err(|e| HttpError(format!("Error processing response: {}", e)))
        });

    Ok(result)
}

fn parse_json_body(response_body: &str, headers: &http::HeaderMap) -> Option<serde_json::Value> {
    if response_body.is_empty() {
        return None;
    }

    let content_type = match get_content_type(headers) {
        Some(content_type) => content_type,
        None => {
            return None;
        }
    };

    if !content_type.contains("application/json") {
        return None;
    }

    match serde_json::from_str(response_body) {
        Ok(body) => Some(body),
        Err(e) => {
            log::error!("Error parsing response body: {}", e);
            None
        }
    }
}

fn get_content_type(headers: &http::HeaderMap) -> Option<String> {
    match headers.get("content-type") {
        Some(content_type) => match content_type.to_str() {
            Ok(content_type_str) => Some(content_type_str.to_string()),
            Err(_e) => None,
        },
        None => None,
    }
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
