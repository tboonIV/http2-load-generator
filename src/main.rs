mod config;
mod http_api;
mod runner;
mod stats;

use crate::config::read_yaml_file;
use crate::runner::AggregatedReport;
use crate::runner::Runner;
use chrono::Local;
use std::error::Error;
use std::io::Write;
use std::thread;
use tokio::sync::mpsc;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    // Read config
    let config = read_yaml_file("config.yaml")?;

    // Configure Logging
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
        .filter(None, config.log_level.into())
        .init();

    log::debug!("Config: {:?}", config);

    // Other parameters
    let batch_size = match config.batch_size {
        config::BatchSize::Auto(_) => None,
        config::BatchSize::Fixed(size) => Some(size),
    };

    if config.duration.as_secs() <= 0 {
        return Err("Duration must be at least 1s".into());
    }
    let duration_s = config.duration.as_secs() as u32;

    // Runner in parallel
    let (tx, mut rx) = mpsc::channel(8);
    for _ in 0..config.parallel {
        let tx = tx.clone();
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                let runner = Runner::new(config.target_tps, duration_s, batch_size);
                let report = runner.run().await.unwrap();
                tx.send(report).await.unwrap();
            });
        });
    }

    drop(tx);

    // Aggregate report
    let mut aggregate_report = AggregatedReport::new();
    while let Some(report) = rx.recv().await {
        aggregate_report.add(report);
    }
    aggregate_report.report();

    Ok(())
}
