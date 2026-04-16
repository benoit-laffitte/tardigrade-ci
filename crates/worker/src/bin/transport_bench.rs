use anyhow::{Result, anyhow};
use axum::{Json, Router, extract::State, routing::post};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};
use tardigrade_api::{ApiState, build_router};
use tardigrade_scheduler::InMemoryScheduler;
use tardigrade_storage::InMemoryStorage;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use uuid::Uuid;

#[path = "../worker_api.rs"]
mod worker_api;

use worker_api::{HttpWorkerApi, WorkerApi};

/// Re-declares the worker transport config shape used by the shared HTTP client builder.
mod worker_config {
    /// Holds the transport-related worker settings consumed by the benchmark harness.
    #[derive(Debug, Clone)]
    pub(crate) struct WorkerConfig {
        pub(crate) server_url: String,
        pub(crate) worker_id: String,
        pub(crate) http2_enabled: bool,
        pub(crate) http2_prior_knowledge: bool,
        pub(crate) request_timeout_secs: u64,
        pub(crate) pool_idle_timeout_secs: u64,
        pub(crate) pool_max_idle_per_host: usize,
        pub(crate) http2_keep_alive_secs: u64,
    }
}

use worker_config::WorkerConfig;

/// Default number of measured claim/complete cycles per scenario.
const DEFAULT_ITERATIONS: usize = 200;

/// Default number of warmup cycles per scenario.
const DEFAULT_WARMUP: usize = 25;

/// Default worker fanout used by concurrent benchmark scenarios.
const DEFAULT_WORKERS: usize = 8;

/// Holds immutable state shared by the local benchmark GraphQL server.
#[derive(Debug)]
struct BenchServerState {
    claim_counter: AtomicUsize,
    fixed_job_id: Uuid,
    fixed_timestamp: String,
}

/// Owns the benchmark server address and its background task.
struct BenchServer {
    server_url: String,
    task: JoinHandle<()>,
}

/// Captures measured latency and throughput for one transport scenario.
struct ScenarioResult {
    label: &'static str,
    cycles: usize,
    total: Duration,
    samples: Vec<Duration>,
}

/// Runs the local worker transport benchmark and prints a CSV-friendly summary.
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let iterations = parse_usize_flag(&args, "--iterations", DEFAULT_ITERATIONS);
    let warmup = parse_usize_flag(&args, "--warmup", DEFAULT_WARMUP);
    let workers = parse_usize_flag(&args, "--workers", DEFAULT_WORKERS).max(1);

    let mock_server = spawn_mock_bench_server().await?;

    let mock_http1_seq = run_scenario(
        "mock-http1-seq",
        benchmark_config(&mock_server.server_url, false, false),
        iterations,
        warmup,
    )
    .await?;
    let mock_http2_seq = run_scenario(
        "mock-http2-h2c-seq",
        benchmark_config(&mock_server.server_url, true, true),
        iterations,
        warmup,
    )
    .await?;

    let mock_http1_conc = run_scenario_concurrent(
        "mock-http1-conc",
        benchmark_config(&mock_server.server_url, false, false),
        iterations,
        warmup,
        workers,
    )
    .await?;
    let mock_http2_conc = run_scenario_concurrent(
        "mock-http2-h2c-conc",
        benchmark_config(&mock_server.server_url, true, true),
        iterations,
        warmup,
        workers,
    )
    .await?;

    mock_server.task.abort();

    let real_server = spawn_real_bench_server().await?;
    seed_real_server_builds(&real_server.server_url, iterations + warmup).await?;
    let real_http1_seq = run_scenario(
        "real-http1-seq",
        benchmark_config(&real_server.server_url, false, false),
        iterations,
        warmup,
    )
    .await?;

    seed_real_server_builds(&real_server.server_url, iterations + warmup).await?;
    let real_http2_seq = run_scenario(
        "real-http2-h2c-seq",
        benchmark_config(&real_server.server_url, true, true),
        iterations,
        warmup,
    )
    .await?;

    seed_real_server_builds(&real_server.server_url, iterations + warmup).await?;
    let real_http1_conc = run_scenario_concurrent(
        "real-http1-conc",
        benchmark_config(&real_server.server_url, false, false),
        iterations,
        warmup,
        workers,
    )
    .await?;

    seed_real_server_builds(&real_server.server_url, iterations + warmup).await?;
    let real_http2_conc = run_scenario_concurrent(
        "real-http2-h2c-conc",
        benchmark_config(&real_server.server_url, true, true),
        iterations,
        warmup,
        workers,
    )
    .await?;

    print_table(&[
        &mock_http1_seq,
        &mock_http2_seq,
        &mock_http1_conc,
        &mock_http2_conc,
        &real_http1_seq,
        &real_http2_seq,
        &real_http1_conc,
        &real_http2_conc,
    ]);

    print_pair_delta("mock-seq", &mock_http1_seq, &mock_http2_seq);
    print_pair_delta("mock-conc", &mock_http1_conc, &mock_http2_conc);
    print_pair_delta("real-seq", &real_http1_seq, &real_http2_seq);
    print_pair_delta("real-conc", &real_http1_conc, &real_http2_conc);

    real_server.task.abort();
    Ok(())
}

/// Parses one optional usize CLI flag while keeping a safe fallback default.
fn parse_usize_flag(args: &[String], flag: &str, default: usize) -> usize {
    args.windows(2)
        .find(|window| window[0] == flag)
        .and_then(|window| window[1].parse::<usize>().ok())
        .unwrap_or(default)
}

/// Builds one worker config tuned for one benchmark transport scenario.
fn benchmark_config(
    server_url: &str,
    http2_enabled: bool,
    http2_prior_knowledge: bool,
) -> WorkerConfig {
    WorkerConfig {
        server_url: server_url.to_string(),
        worker_id: "bench-worker".to_string(),
        http2_enabled,
        http2_prior_knowledge,
        request_timeout_secs: 5,
        pool_idle_timeout_secs: 30,
        pool_max_idle_per_host: 8,
        http2_keep_alive_secs: 10,
    }
}

/// Spawns one local Axum server implementing the minimal worker GraphQL contract.
async fn spawn_mock_bench_server() -> Result<BenchServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let state = Arc::new(BenchServerState {
        claim_counter: AtomicUsize::new(0),
        fixed_job_id: Uuid::new_v4(),
        fixed_timestamp: "2026-04-16T00:00:00Z".to_string(),
    });

    let app = Router::new()
        .route("/graphql", post(graphql_handler))
        .with_state(state);

    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    Ok(BenchServer {
        server_url: format!("http://{addr}"),
        task,
    })
}

/// Spawns one local real API server using the production router and in-memory state.
async fn spawn_real_bench_server() -> Result<BenchServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let state = ApiState::with_components(
        "transport-bench",
        Arc::new(InMemoryStorage::default()),
        Arc::new(InMemoryScheduler::default()),
    );
    let app = build_router(state);

    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    Ok(BenchServer {
        server_url: format!("http://{addr}"),
        task,
    })
}

/// Serves synthetic GraphQL claim/complete payloads for worker transport measurements.
async fn graphql_handler(
    State(state): State<Arc<BenchServerState>>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let query = payload
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if query.contains("worker_claim_build") {
        let build_id = Uuid::new_v4();
        let sequence = state.claim_counter.fetch_add(1, Ordering::Relaxed) + 1;

        return Json(json!({
            "data": {
                "worker_claim_build": {
                    "id": build_id.to_string(),
                    "job_id": state.fixed_job_id.to_string(),
                    "status": "RUNNING",
                    "queued_at": state.fixed_timestamp,
                    "started_at": state.fixed_timestamp,
                    "finished_at": Value::Null,
                    "logs": [format!("claim-{sequence}")]
                }
            }
        }));
    }

    if query.contains("worker_complete_build") {
        let build_id = payload
            .get("variables")
            .and_then(|value| value.get("buildId"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        return Json(json!({
            "data": {
                "worker_complete_build": {
                    "id": build_id
                }
            }
        }));
    }

    Json(json!({
        "errors": [
            {
                "message": "unsupported benchmark GraphQL operation"
            }
        ]
    }))
}

/// Runs one warmup phase then measures claim/complete cycles for a given transport mode.
async fn run_scenario(
    label: &'static str,
    config: WorkerConfig,
    iterations: usize,
    warmup: usize,
) -> Result<ScenarioResult> {
    let api = HttpWorkerApi::from_config(&config)?;
    let graphql_url = format!("{}/graphql", config.server_url);

    for _ in 0..warmup {
        run_cycle(&api, &graphql_url, &config.worker_id).await?;
    }

    let started = Instant::now();
    let mut samples = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let cycle_started = Instant::now();
        run_cycle(&api, &graphql_url, &config.worker_id).await?;
        samples.push(cycle_started.elapsed());
    }

    Ok(ScenarioResult {
        label,
        cycles: iterations,
        total: started.elapsed(),
        samples,
    })
}

/// Runs one warmup+measurement scenario with concurrent workers.
async fn run_scenario_concurrent(
    label: &'static str,
    config: WorkerConfig,
    iterations: usize,
    warmup: usize,
    workers: usize,
) -> Result<ScenarioResult> {
    let graphql_url = format!("{}/graphql", config.server_url);

    let warmup_started = Instant::now();
    run_concurrent_cycles(&config, &graphql_url, warmup, workers).await?;
    let _warmup_total = warmup_started.elapsed();

    let started = Instant::now();
    run_concurrent_cycles(&config, &graphql_url, iterations, workers).await?;
    let measured_total = started.elapsed();

    // Concurrent scenario keeps aggregate timing as one sample per scenario run.
    Ok(ScenarioResult {
        label,
        cycles: iterations,
        total: measured_total,
        samples: vec![measured_total],
    })
}

/// Distributes cycles across worker tasks and executes claim/complete in parallel.
async fn run_concurrent_cycles(
    config: &WorkerConfig,
    graphql_url: &str,
    cycles: usize,
    workers: usize,
) -> Result<()> {
    let mut handles = Vec::with_capacity(workers);
    let base = cycles / workers;
    let extra = cycles % workers;

    for index in 0..workers {
        let per_worker_cycles = base + usize::from(index < extra);
        if per_worker_cycles == 0 {
            continue;
        }

        let worker_id = format!("{}-{index}", config.worker_id);
        let cfg = config.clone();
        let url = graphql_url.to_string();

        handles.push(tokio::spawn(async move {
            let api = HttpWorkerApi::from_config(&cfg)?;
            for _ in 0..per_worker_cycles {
                run_cycle(&api, &url, &worker_id).await?;
            }
            Result::<(), anyhow::Error>::Ok(())
        }));
    }

    for handle in handles {
        handle.await??;
    }

    Ok(())
}

/// Executes one full worker claim then complete roundtrip against the benchmark server.
async fn run_cycle(api: &HttpWorkerApi, graphql_url: &str, worker_id: &str) -> Result<()> {
    let build = api
        .claim(graphql_url, worker_id)
        .await?
        .ok_or_else(|| anyhow!("benchmark server returned no build"))?;

    api.complete(
        graphql_url,
        worker_id,
        build.id,
        &tardigrade_api::CompleteBuildRequest {
            status: tardigrade_api::WorkerBuildStatus::Success,
            log_line: Some("benchmark completion".to_string()),
        },
    )
    .await
}

/// Prints aggregate statistics and a relative average-latency delta between protocols.
fn print_table(results: &[&ScenarioResult]) {
    println!("label,total_ms,avg_ms,p50_ms,p95_ms,ops_per_sec,cycles");
    for result in results {
        println!(
            "{},{:.3},{:.3},{:.3},{:.3},{:.2},{}",
            result.label,
            duration_ms(result.total),
            duration_ms(average_duration(&result.samples)),
            duration_ms(percentile_duration(&result.samples, 0.50)),
            duration_ms(percentile_duration(&result.samples, 0.95)),
            operations_per_sec(result),
            result.cycles,
        );
    }
}

/// Prints relative average-latency delta between two protocol variants.
fn print_pair_delta(scope: &str, http1: &ScenarioResult, http2: &ScenarioResult) {
    println!(
        "relative_delta_avg_ms_percent[{scope}],{:.2}",
        relative_delta_percent(http1, http2)
    );
}

/// Returns relative average-latency delta between the two scenarios.
fn relative_delta_percent(http1: &ScenarioResult, http2: &ScenarioResult) -> f64 {
    let http1_avg = duration_ms(average_duration(&http1.samples));
    let http2_avg = duration_ms(average_duration(&http2.samples));
    if http1_avg > 0.0 {
        ((http1_avg - http2_avg) / http1_avg) * 100.0
    } else {
        0.0
    }
}

/// Returns the mean duration across all recorded samples.
fn average_duration(samples: &[Duration]) -> Duration {
    let total_nanos: u128 = samples.iter().map(Duration::as_nanos).sum();
    let average_nanos = total_nanos / u128::from(samples.len() as u64);
    nanos_to_duration(average_nanos)
}

/// Returns one percentile duration from the measured samples.
fn percentile_duration(samples: &[Duration], percentile: f64) -> Duration {
    let mut sorted: Vec<u128> = samples.iter().map(Duration::as_nanos).collect();
    sorted.sort_unstable();
    let index = ((sorted.len().saturating_sub(1)) as f64 * percentile).round() as usize;
    nanos_to_duration(sorted[index])
}

/// Converts one scenario runtime into operations per second.
fn operations_per_sec(result: &ScenarioResult) -> f64 {
    let seconds = result.total.as_secs_f64();
    if seconds > 0.0 {
        result.cycles as f64 / seconds
    } else {
        0.0
    }
}

/// Converts one duration to milliseconds represented as f64.
fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// Converts nanoseconds into a standard duration with safe clamping.
fn nanos_to_duration(nanos: u128) -> Duration {
    let clamped = nanos.min(u128::from(u64::MAX));
    Duration::from_nanos(clamped as u64)
}

/// Seeds one real API server with enough queued builds for the benchmark scenario.
async fn seed_real_server_builds(server_url: &str, count: usize) -> Result<()> {
    let graphql_url = format!("{server_url}/graphql");
    let bootstrap_client = reqwest::Client::new();

    let create_payload = bootstrap_client
        .post(&graphql_url)
        .json(&json!({
            "query": "mutation CreateJob($input: GqlCreateJobInput!) { create_job(input: $input) { id } }",
            "variables": {
                "input": {
                    "name": "bench-job",
                    "repository_url": "https://example.invalid/bench.git",
                    "pipeline_path": "pipelines/bench.yml"
                }
            }
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;

    let job_id = create_payload
        .get("data")
        .and_then(|value| value.get("create_job"))
        .and_then(|value| value.get("id"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("failed to bootstrap benchmark job id"))?;

    for _ in 0..count {
        let payload = bootstrap_client
            .post(&graphql_url)
            .json(&json!({
                "query": "mutation RunJob($jobId: ID!) { run_job(jobId: $jobId) { id } }",
                "variables": {
                    "jobId": job_id
                }
            }))
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        if payload.get("errors").and_then(Value::as_array).is_some() {
            return Err(anyhow!("failed to enqueue benchmark build"));
        }
    }

    Ok(())
}
