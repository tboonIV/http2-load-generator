mod config;
mod function;
mod http_api;
mod runner;
mod scenario;
mod scripting;
mod stats;
mod variable;

use crate::config::read_yaml_file;
use crate::runner::AggregatedReport;
use crate::runner::Runner;
use crate::scenario::Global;
use chrono::Local;
use clap::Parser;
use std::error::Error;
use std::io::Write;
use std::thread;
use tokio::sync::mpsc;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./config.yaml")]
    config: String,

    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    // Read config
    let config = read_yaml_file(&args.config)?;

    // Configure Logging
    env_logger::Builder::new()
        .filter_module("jsonpath_lib", log::LevelFilter::Error)
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

    log::debug!("New Config: {:?}", config);

    // Runner in parallel
    let (tx, mut rx) = mpsc::channel(8);
    for _ in 0..config.parallel {
        let tx = tx.clone();
        let config = config.clone();
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                let global = Global::new(config.runner.global.clone());
                let mut runner = Runner::new(config.runner, &global).unwrap();
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
