# Contrat API canonique (GraphQL-first)

Ce document formalise la surface HTTP runtime exposee par Tardigrade CI.

## Perimetre

- Contrat public control-plane: GraphQL-first.
- Exception explicite: endpoint webhook SCM natif.
- Le dashboard web est servi comme console statique (hors control-plane API).

## Matrice des routes runtime (CORECI-01a)

| Methode | Route | Montee par | Statut | Notes |
| --- | --- | --- | --- | --- |
| GET | /graphql | `crates/api/src/routing/mod.rs` | Exposee | GraphQL Playground |
| POST | /graphql | `crates/api/src/routing/mod.rs` | Exposee | Endpoint GraphQL principal |
| POST | /webhooks/scm | `crates/server/src/webhook_adapter.rs` | Exposee | Adaptateur webhook SCM natif |
| GET | / | `crates/server/src/dashboard/service.rs` (fallback static) | Exposee | Console dashboard statique |
| GET | /<asset> | `crates/server/src/dashboard/service.rs` (fallback static) | Exposee | Assets frontend (`app.js`, `styles.css`, etc.) |

## Inventaire handlers orphelins (CORECI-01a)

- `crates/api/src/handlers`: uniquement handlers GraphQL montes.
- Aucun handler REST legacy detecte dans `crates/api/src`.
- Aucun montage runtime de routes REST (`/health`, `/jobs`, `/metrics`, etc.) cote API/server.

Conclusion: pas de surface orpheline a quarantaine immediate sur ce scope; `CORECI-01c` peut etre cloture par constat si aucun nouvel artefact REST n est reintroduit.

## Contrat canonique (CORECI-01b)

### Routes control-plane supportees

- `GET /graphql`
- `POST /graphql`
- `POST /webhooks/scm`

### Hors contrat control-plane

- Toute route REST historique (`/health`, `/jobs`, `/builds`, `/workers`, `/metrics`, etc.).

### Politique de regression

- Toute exposition accidentelle d une route hors contrat doit casser les tests de surface (voir `CORECI-01d`).
- Test de regression de surface runtime: `crates/server/tests/webhook_adapter.rs::server_route_surface_matches_canonical_contract`.

## Matrice de policy auth GraphQL (CORECI-02a)

| Surface | Operations | Politique cible |
| --- | --- | --- |
| GraphQL Query | `health`, `live`, `ready`, `jobs`, `builds`, `workers`, `plugins`, `plugin_policy`, `plugin_authorization_check`, `metrics`, `scm_webhook_rejections`, `dead_letter_builds`, `dashboard_snapshot` | Lecture: pas d API key requise |
| GraphQL Mutation | `create_job`, `run_job`, `cancel_build`, `load_plugin`, `init_plugin`, `execute_plugin`, `unload_plugin`, `upsert_plugin_policy`, `upsert_webhook_security_config`, `upsert_scm_polling_config`, `run_scm_polling_tick`, `ingest_scm_webhook`, `worker_claim_build`, `worker_complete_build` | Ecriture: API key requise |
| Webhook natif | `POST /webhooks/scm` | Hors policy API key (auth provider par signature/token SCM) |

Note de phase:

- `CORECI-02b` ajoute extraction/verification API key au niveau middleware server et publie un contexte de requete.
- `CORECI-02c` applique l enforcement sur les operations d ecriture GraphQL: `missing` -> `unauthorized`, `invalid` -> `forbidden`.
