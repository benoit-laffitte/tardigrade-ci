# Technology Profile Onboarding

This guide explains how to add a new first-class stack profile in Tardigrade CI.

The onboarding flow keeps code, docs, and validation behavior aligned.

## Scope

A technology profile is the model consumed by the platform to describe:

- language family
- runtime container image
- build/test/package command strategy

Current built-in catalog lives in [crates/core/src/lib.rs](../crates/core/src/lib.rs).

## 1. Add profile metadata in core model

Update `built_in_technology_profiles()` in [crates/core/src/lib.rs](../crates/core/src/lib.rs):

- choose a stable `id` (lowercase, short, unique)
- define a human-readable `display_name`
- map to a `TechnologyLanguage` enum value
- define runtime image and optional shell
- define strategy commands (`install`, `build`, `test`, `package`)

Use explicit image tags for reproducibility.

## 2. Extend language enum when required

If the new stack introduces a language family not represented yet:

- add a new variant to `TechnologyLanguage`
- keep serde naming compatible (`snake_case`)
- update tests that assert expected language coverage

## 3. Add non-blocking validation hints (optional but recommended)

If the stack has common CI pitfalls, extend `PipelineDefinition::validation_hints()` in [crates/core/src/lib.rs](../crates/core/src/lib.rs).

Hints should be:

- non-blocking recommendations
- deterministic
- focused on high-signal mistakes

Examples:

- missing lockfile behavior
- interactive CLI flags in CI
- overly narrow test target scope

## 4. Add/update recipes in docs

Update [docs/pipeline-recipes.md](pipeline-recipes.md) with a copy/paste-ready recipe for the new stack.

Recipe expectations:

- valid schema `version: 1`
- explicit image
- clear ordered stages
- explicit command arrays

## 5. Extend smoke matrix coverage

Add or update API integration smoke tests in [crates/api/tests/jobs.rs](../crates/api/tests/jobs.rs) so the new profile has an end-to-end create/run success path.

The smoke matrix should keep at least Rust, Python, and Java coverage and include the new stack when it becomes first-class.

## 6. Update backlog status

Update [BACKLOG.md](../BACKLOG.md):

- mark the relevant `TECH-*` item in progress/done
- keep next `TECH-*` item ready for continuation

## 7. Validation checklist

Run the following before opening a PR:

```bash
env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo test -p tardigrade-core
env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo test -p tardigrade-api --test jobs
make ci
```

## Done criteria for a new profile

A new profile onboarding is complete when:

- model metadata is present and validated
- docs recipe is available and copy/paste-ready
- smoke matrix includes the stack (or explicitly documents why deferred)
- full CI is green
