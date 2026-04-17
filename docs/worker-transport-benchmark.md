# Worker Transport Benchmark

Date: 2026-04-16

## Objective

Measure the current agent d execution `claim -> complete` transport path and compare HTTP/1 vs HTTP/2 using the same GraphQL agent d execution contract.

This benchmark exists to support `REL-03` and to validate whether the newly added HTTP/2-capable agent d execution client produces a measurable gain in the current implementation.

## Harness

Implementation entrypoint:

- `crates/agent d execution/src/bin/transport_bench.rs`

Run command:

```bash
make agent d execution-transport-bench
```

Equivalent direct command:

```bash
env -u https_proxy -u http_proxy -u PXY_FAB_FONC \
cargo run -p tardigrade-worker --features transport-bench --bin transport_bench -- --iterations 200
```

## Method

- The benchmark starts two local servers on `127.0.0.1`:
  - one synthetic Axum mock GraphQL server
  - one real API router (`build_router(ApiState)`) backed by in-memory storage/scheduler
- The mock server exposes the minimal GraphQL contract needed by the agent d execution:
  - `worker_claim_build`
  - `worker_complete_build`
- Four benchmark scopes are measured with the same agent d execution client code path:
  - mock sequential
  - mock concurrent
  - real server sequential
  - real server concurrent
- Each scope compares protocol variants:
  - HTTP/1
  - HTTP/2 h2c (`http2_enabled=true`, `http2_prior_knowledge=true`)
- Real-server scopes pre-seed the queue with enough builds so each `claim -> complete` cycle has work.
- Output includes total runtime, average latency, p50, p95, and throughput.

## Local Result

Reference run:

```text
label,total_ms,avg_ms,p50_ms,p95_ms,ops_per_sec,cycles
mock-http1-seq,50.642,0.422,0.378,0.627,2369.58,120
mock-http2-h2c-seq,63.498,0.529,0.516,0.650,1889.83,120
mock-http1-conc,9.829,9.829,9.829,9.829,12208.36,120
mock-http2-h2c-conc,15.154,15.154,15.154,15.154,7918.90,120
real-http1-seq,80.566,0.671,0.673,0.697,1489.47,120
real-http2-h2c-seq,102.446,0.854,0.852,0.891,1171.35,120
real-http1-conc,19.819,19.819,19.819,19.819,6054.76,120
real-http2-h2c-conc,23.309,23.309,23.309,23.309,5148.26,120
relative_delta_avg_ms_percent[mock-seq],-25.39
relative_delta_avg_ms_percent[mock-conc],-54.17
relative_delta_avg_ms_percent[real-seq],-27.16
relative_delta_avg_ms_percent[real-conc],-17.61
```

## Reading Of The Result

- In all measured local scopes, `http1` is faster than `http2-h2c`.
- The measured average-latency penalty for `http2-h2c` ranges from about `17%` (real concurrent) to `54%` (mock concurrent).
- Throughput is also lower for `http2-h2c` in every measured scope.

## Important Limits

This benchmark does **not** prove that HTTP/2 is a bad choice in production.

It only proves that, for the current synthetic setup:

- loopback network
- tiny JSON GraphQL payloads
- no TLS
- mostly local request patterns (sequential and bounded local concurrency)
- local in-process mock server

`h2c` does not outperform HTTP/1 in these local tests.

That is a plausible result. HTTP/2 usually becomes more interesting when at least one of these conditions changes:

- higher concurrency
- real network latency
- TLS in front of the connection
- more request multiplexing pressure
- many agents d execution sharing fewer long-lived connections

## Practical Conclusion

What is validated now:

- the agent d execution transport supports HTTP/2 tuning and h2c prior knowledge
- the benchmark harness is reproducible and useful for regressions
- the current local micro-benchmark does not justify forcing HTTP/2 as universally faster

What is **not** validated yet:

- production-like gains under concurrent agent d execution load
- gains against the real server instead of a synthetic mock
- gains with TLS termination / HTTP/2 over TLS

## Recommendation

- Keep the HTTP/2-capable transport implementation.
- Keep HTTP/2 configurable instead of assuming it is always better.
- Do not close `REL-03` yet based only on local h2c runs.

## Suite Benchmark Steps

1. Add a TLS-enabled benchmark to compare HTTP/1.1 vs HTTP/2 in a production-like transport mode.
2. Add higher agent d execution fanout and larger payload variants to stress multiplexing effects.
3. Re-run the same matrix on a remote network path (non-loopback) to include RTT effects.