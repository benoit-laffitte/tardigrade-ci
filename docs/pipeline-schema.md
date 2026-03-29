# Pipeline Schema v1 (Draft)

This document defines the first version of the CI pipeline schema.

## Goals

- Provide a versioned, explicit pipeline contract.
- Support ordered stages and steps.
- Provide retry policy hooks at step level.

## Structure

Top-level object:

- `version` (`u32`): schema version. Current value is `1`.
- `stages` (`PipelineStage[]`): ordered stage list.

Stage object:

- `name` (`string`): stage name.
- `steps` (`PipelineStep[]`): ordered step list.

Step object:

- `name` (`string`): step name.
- `image` (`string`): container image used by executor.
- `command` (`string[]`): command and args.
- `env` (`map<string,string>`): optional environment map (empty by default).
- `retry` (`PipelineRetryPolicy | null`): optional per-step retry override.

Retry policy object:

- `max_attempts` (`u32`): maximum execution attempts.
- `backoff_ms` (`u64`): delay before retry in milliseconds.

## Example (YAML)

```yaml
version: 1
stages:
  - name: compile
    steps:
      - name: build
        image: rust:1.94
        command: ["cargo", "build", "--workspace"]
        env: {}
  - name: verify
    steps:
      - name: unit-tests
        image: rust:1.94
        command: ["cargo", "test", "--workspace"]
        env: {}
        retry:
          max_attempts: 3
          backoff_ms: 1500
```

## Notes

- This schema definition is the contract layer only.
- YAML parsing and structural validation are implemented in `tardigrade-core`.
- API-level validation surface and error mapping are covered by follow-up backlog items (`DSL-03+`).
