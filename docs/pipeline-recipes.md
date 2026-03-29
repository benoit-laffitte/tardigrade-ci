# Pipeline Recipes (Multi-Technology)

This guide provides copy/paste-ready examples for common stacks.

All recipes use the same DSL contract:

- `version: 1`
- ordered `stages`
- ordered `steps`
- each step provides `image` + `command`

## Rust workspace

```yaml
version: 1
stages:
  - name: compile
    steps:
      - name: cargo-build
        image: "rust:1.94"
        command: ["cargo", "build", "--workspace", "--locked"]
  - name: verify
    steps:
      - name: cargo-test
        image: "rust:1.94"
        command: ["cargo", "test", "--workspace"]
```

## Python project

```yaml
version: 1
stages:
  - name: deps
    steps:
      - name: install
        image: "python:3.12"
        command: ["python", "-m", "pip", "install", "-r", "requirements.txt"]
  - name: verify
    steps:
      - name: pytest
        image: "python:3.12"
        command: ["pytest", "-q"]
```

## Java project (Maven)

```yaml
version: 1
stages:
  - name: build
    steps:
      - name: mvn-package
        image: "maven:3.9-eclipse-temurin-21"
        command: ["mvn", "-B", "-DskipTests", "package"]
  - name: verify
    steps:
      - name: mvn-test
        image: "maven:3.9-eclipse-temurin-21"
        command: ["mvn", "-B", "test"]
```

## Node.js project

```yaml
version: 1
stages:
  - name: deps
    steps:
      - name: npm-ci
        image: "node:20-bookworm"
        command: ["npm", "ci"]
  - name: build
    steps:
      - name: npm-build
        image: "node:20-bookworm"
        command: ["npm", "run", "build"]
  - name: verify
    steps:
      - name: npm-test
        image: "node:20-bookworm"
        command: ["npm", "test"]
```

## Go project

```yaml
version: 1
stages:
  - name: deps
    steps:
      - name: go-mod-download
        image: "golang:1.24-bookworm"
        command: ["go", "mod", "download"]
  - name: build
    steps:
      - name: go-build
        image: "golang:1.24-bookworm"
        command: ["go", "build", "./..."]
  - name: verify
    steps:
      - name: go-test
        image: "golang:1.24-bookworm"
        command: ["go", "test", "./..."]
```

## Mixed stack (Rust backend + Python checks + Java contract tests)

```yaml
version: 1
stages:
  - name: rust-build
    steps:
      - name: compile
        image: "rust:1.94"
        command: ["cargo", "build", "--workspace"]

  - name: python-quality
    steps:
      - name: lint
        image: "python:3.12"
        command: ["python", "-m", "ruff", "check", "."]

  - name: java-contract-tests
    steps:
      - name: test
        image: "maven:3.9-eclipse-temurin-21"
        command: ["mvn", "-B", "-pl", "contracts", "test"]
```

## Notes

- Keep images pinned to explicit tags for reproducibility.
- Use one stage per logical quality gate to keep execution flow readable.
- Prefer small, explicit command arrays over shell wrappers.
