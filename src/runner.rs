use crate::config;
use crate::config::RunnerConfig;
use crate::http_api::{send_request, HttpRequest, HttpResponse};
use crate::stats::ApiStats;
use bytes::Bytes;
use futures::future::{BoxFuture, FutureExt};
use h2::client;
use h2::client::SendRequest;
use http::Method;
use http::StatusCode;
use std::error::Error;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::time;
use tokio::time::Duration;

pub struct Runner {
    param: RunParameter,
    target_address: String,
    first_scenario: ScenarioParameter,
    subsequent_scenarios: Vec<ScenarioParameter>,
}

impl Runner {
    pub fn new(config: RunnerConfig) -> Result<Runner, Box<dyn Error>> {
        // batch size
        let batch_size = match config.batch_size {
            config::BatchSize::Auto(_) => None,
            config::BatchSize::Fixed(size) => Some(size),
        };

        // duration
        if config.duration.as_secs() <= 0 {
            return Err("Duration must be at least 1s".into());
        }
        let duration_s = config.duration.as_secs() as u32;

        // target address
        let url = config.base_url.clone();
        let address = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or(&url);
        let address = address.trim_end_matches('/');
        log::debug!("Target Address: {}", address);

        // scenarios
        let mut subsequent_scenarios = vec![];
        let first_scenario = config.scenarios.get(0).ok_or("No scenario defined")?;

        for scenario_config in config.scenarios.iter().skip(1) {
            subsequent_scenarios.push(scenario_config.into());
        }
        log::debug!("Scenarios: {:?}", subsequent_scenarios);

        Ok(Runner {
            param: RunParameter::new(config.target_tps, duration_s, batch_size),
            target_address: address.into(),
            first_scenario: first_scenario.into(),
            subsequent_scenarios,
        })
    }

    pub async fn run(&self) -> Result<RunReport, Box<dyn Error>> {
        let tcp = TcpStream::connect(&self.target_address).await?;
        let (mut client, h2) = client::handshake(tcp).await?;

        tokio::task::spawn(async move {
            if let Err(e) = h2.await {
                log::error!("Unexpected error: {:?}", e);
            }
        });

        let param = &self.param;
        let total_iterations = param.total_requests as f64 / param.batch_size as f64;
        let total_iterations = total_iterations.ceil() as u32;
        let total_requests = total_iterations * param.batch_size;

        log::info!(
            "Sending Total Req: {}, Iteration: {}, Target TPS: {}, Batch Size: {}, Interval: {}",
            total_requests,
            total_iterations,
            param.target_tps,
            param.batch_size,
            param.interval.as_secs_f64()
        );

        let start = Instant::now();
        let api_stats = Arc::new(ApiStats::new());

        let mut interval = time::interval(param.interval);
        for _ in 0..total_iterations {
            interval.tick().await;

            let mut response_futures = vec![];
            for _ in 0..param.batch_size {
                let scenario = self.first_scenario.clone();

                let http_request = HttpRequest {
                    uri: scenario.uri.clone(),
                    method: scenario.method.clone(),
                    body: scenario.body.clone(),
                };
                let future = send_request(&mut client, http_request).await;
                log::debug!("First Request sent");
                // let future = Self::run_scenario(&mut client, scenario).await;
                match future {
                    Ok(future) => response_futures.push(future),
                    Err(_e) => {
                        // log::error!("Error sending request: {}", e);
                        api_stats.inc_error();
                    }
                }
            }

            // This will make the load-generator fully asynchronous and non-blocking
            // Otherwise, it will be partially blocking.
            //
            // let api_stats = Arc::clone(&api_stats);
            // tokio::task::spawn(async move {
            for future in response_futures {
                let response = future.await.unwrap();
                log::debug!("Response Status: {:?}", response.status);
                log::debug!("Response Body: {:?}", response.body);

                api_stats.inc_retry(response.retry_count.into());

                if response.status != StatusCode::OK {
                    api_stats.inc_error();
                } else {
                    let round_trip_time = response.request_start.elapsed().as_micros() as u64;
                    api_stats.inc_rtt(round_trip_time);
                    api_stats.inc_success();

                    Self::run_scenario_sub(&mut client, self.subsequent_scenarios.clone(), 0).await;
                }
            }
            // });
        }

        while api_stats.get_success() + api_stats.get_error() < total_requests {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let success_count = api_stats.get_success();
        let error_count = api_stats.get_error();
        let total_count = success_count + error_count;
        let total_rtt = Duration::from_micros(api_stats.get_rtt());
        let total_retry = api_stats.get_retry();

        let elapsed = start.elapsed();
        let elapsed_s = elapsed.as_secs() as f64 + elapsed.subsec_millis() as f64 / 1000.0;
        let tps = success_count as f64 / (elapsed.as_micros() as f64 / 1_000_000.0);
        let avg_rtt = total_rtt.as_millis() as f64 / success_count as f64;

        log::info!(
            "Elapsed: {:.3}s, TPS: {:.3}, RTT: {:.3}ms, Error: ({}/{}), Retry: {}",
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
            total_rtt,
        };
        Ok(report)
    }

    // async fn run_scenario(
    //     client: &mut SendRequest<Bytes>,
    //     scenario: ScenarioParameter,
    // ) -> Result<JoinHandle<HttpResponse>, Box<dyn Error>> {
    //     let http_request = HttpRequest {
    //         uri: scenario.uri.clone(),
    //         method: scenario.method.clone(),
    //         body: scenario.body.clone(),
    //     };
    //     let future = send_request(client, http_request).await?;
    //     log::debug!("First Request sent");
    //     // match future {
    //     //     Ok(future) => response_futures.push(future),
    //     //     Err(_e) => {
    //     //         // log::error!("Error sending request: {}", e);
    //     //         api_stats.inc_error();
    //     //     }
    //     // }
    //     Ok(tokio::task::spawn(async move {
    //         let response = future.await.unwrap();
    //         log::debug!("Response Status: {:?}", response.status);
    //         log::debug!("Response Body: {:?}", response.body);
    //
    //         // api_stats.inc_retry(response.retry_count.into());
    //
    //         if response.status != StatusCode::OK {
    //             // api_stats.inc_error();
    //         } else {
    //             // let round_trip_time = response.request_start.elapsed().as_micros() as u64;
    //             // api_stats.inc_rtt(round_trip_time);
    //             // api_stats.inc_success();
    //         }
    //         // let response = Self::run_scenario_sub(client, scenarios, 0).await;
    //
    //         response
    //     }))
    // }

    fn run_scenario_sub<'a>(
        client: &'a mut SendRequest<Bytes>,
        scenarios: Vec<ScenarioParameter>,
        idx: usize,
    ) -> BoxFuture<'a, HttpResponse> {
        // ) -> BoxFuture<'statc, Result<HttpResponse, Box<dyn Error>>> {

        async move {
            let scenario = scenarios.get(idx).unwrap();
            let http_request = HttpRequest {
                uri: scenario.uri.clone(),
                method: scenario.method.clone(),
                body: scenario.body.clone(),
            };
            let future = send_request(client, http_request).await.unwrap();
            log::debug!("{} Request sent", idx);
            // match future {
            //     Ok(future) => response_futures.push(future),
            //     Err(_e) => {
            //         // log::error!("Error sending request: {}", e);
            //         api_stats.inc_error();
            //     }
            // }
            let response = future.await.unwrap();
            log::debug!("Response Status: {:?}", response.status);
            log::debug!("Response Body: {:?}", response.body);

            // api_stats.inc_retry(response.retry_count.into());

            if response.status != StatusCode::OK {
                // api_stats.inc_error();
            } else {
                // let round_trip_time = response.request_start.elapsed().as_micros() as u64;
                // api_stats.inc_rtt(round_trip_time);
                // api_stats.inc_success();
            }

            let response = if idx + 1 < scenarios.len() {
                let next_future = Self::run_scenario_sub(client, scenarios, idx + 1);
                next_future.await
            } else {
                response
            };
            response
        }
        .boxed()
    }
}

#[derive(Debug, Clone)]
pub struct ScenarioParameter {
    pub name: String,
    pub uri: String,
    pub method: Method,
    pub body: Option<serde_json::Value>,
}

impl From<&config::Scenario> for ScenarioParameter {
    fn from(config: &config::Scenario) -> Self {
        ScenarioParameter {
            name: config.name.clone(),
            uri: config.path.clone(),
            method: config.method.parse().unwrap(),
            body: match &config.body {
                Some(body) => Some(serde_json::from_str(body).unwrap()),
                None => None,
            },
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
    pub total_rtt: Duration,
}

pub struct AggregatedReport {
    pub total_tps: f64,
    pub elapsed: Duration,
    pub total_success: u32,
    pub total_error: u32,
    pub total_rtt: Duration,
}

impl AggregatedReport {
    pub fn new() -> AggregatedReport {
        AggregatedReport {
            total_tps: 0.0,
            elapsed: Duration::from_secs(0),
            total_success: 0,
            total_error: 0,
            total_rtt: Duration::from_secs(0),
        }
    }

    pub fn add(&mut self, report: RunReport) {
        self.total_tps += report.tps;
        self.elapsed = self.elapsed.max(report.elapsed);
        self.total_success += report.success_count;
        self.total_error += report.error_count;
        self.total_rtt += report.total_rtt;
    }

    pub fn report(&self) {
        let elapsed_s =
            self.elapsed.as_secs() as f64 + self.elapsed.subsec_millis() as f64 / 1000.0;

        let avg_rtt = self.total_rtt.as_millis() as f64 / self.total_success as f64;

        log::info!("Total TPS: {:.3}", self.total_tps);
        log::info!("Average Round Trip: {:.4}ms", avg_rtt);
        log::info!("Elapsed: {:.3}s", elapsed_s);
        log::info!(
            "Success Rate: {:.2}%",
            self.total_success as f64 / (self.total_success + self.total_error) as f64 * 100.0
        );
    }
}
