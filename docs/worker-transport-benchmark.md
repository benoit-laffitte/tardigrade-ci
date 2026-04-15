# Worker Transport Benchmark

Date: 2026-04-16

## Objective

Measure the current worker `claim -> complete` transport path and compare HTTP/1 vs HTTP/2 using the same GraphQL worker contract.

This benchmark exists to support `REL-03` and to validate whether the newly added HTTP/2-capable worker client produces a measurable gain in the current implementation.

## Harness

Implementation entrypoint:

- `crates/worker/src/bin/transport_bench.rs`

Run command:

```bash
make worker-transport-bench
```

Equivalent direct command:

```bash
env -u https_proxy -u http_proxy -u PXY_FAB_FONC \
cargo run -p tardigrade-worker --bin transport_bench -- --iterations 200
```

## Method

- The benchmark starts a local Axum mock server on `127.0.0.1`.
- The mock server exposes the minimal GraphQL contract needed by the worker:
  - `worker_claim_build`
  - `worker_complete_build`
- Two scenarios are measured with the same client code path:
  - `http1`
  - `http2-h2c` (`http2_enabled=true`, `http2_prior_knowledge=true`)
- Each scenario runs a warmup phase followed by measured sequential `claim -> complete` cycles.
- Output includes total runtime, average latency, p50, p95, and throughput.

## Local Result

Reference run:

```text
label,total_ms,avg_ms,p50_ms,p95_ms,ops_per_sec,cycles
http1,54.901,0.366,0.358,0.429,2732.19,150
http2-h2c,78.184,0.521,0.525,0.550,1918.56,150
relative_delta_avg_ms_percent,-42.41
```

## Reading Of The Result

- In this local benchmark, `http1` is faster than `http2-h2c`.
- The measured average latency penalty for `http2-h2c` is about `42%` relative to `http1`.
- Throughput is also lower in this specific setup.

## Important Limits

This benchmark does **not** prove that HTTP/2 is a bad choice in production.

It only proves that, for the current synthetic setup:

- loopback network
- tiny JSON GraphQL payloads
- no TLS
- sequential request pattern
- local in-process mock server

`h2c` does not outperform HTTP/1.

That is a plausible result. HTTP/2 usually becomes more interesting when at least one of these conditions changes:

- higher concurrency
- real network latency
- TLS in front of the connection
- more request multiplexing pressure
- many workers sharing fewer long-lived connections

## Practical Conclusion

What is validated now:

- the worker transport supports HTTP/2 tuning and h2c prior knowledge
- the benchmark harness is reproducible and useful for regressions
- the current local micro-benchmark does not justify forcing HTTP/2 as universally faster

What is **not** validated yet:

- production-like gains under concurrent worker load
- gains against the real server instead of a synthetic mock
- gains with TLS termination / HTTP/2 over TLS

## Recommendation

- Keep the HTTP/2-capable transport implementation.
- Keep HTTP/2 configurable instead of assuming it is always better.
- Do not close `REL-03` yet based only on this local run.

## Next Benchmark Steps

1. Add a concurrent multi-worker benchmark.
2. Add a benchmark against the real Rust server path.
3. Add a TLS-enabled benchmark to compare HTTP/1.1 vs HTTP/2 under a more realistic deployment profile.