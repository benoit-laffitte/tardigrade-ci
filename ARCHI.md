# Architecture du projet Tardigrade CI

Ce document donne une vue d ensemble de l architecture actuelle du projet.

## Vue globale (containers + flux)

```mermaid
flowchart LR
  subgraph Clients
    UI[Admin UI Dashboard\nReact/Vite]
    SCM[SCM Providers\nGitHub/GitLab]
    WK[Workers externes\ncrates/worker]
  end

  subgraph ControlPlane[Control Plane]
    SV[Server Axum\ncrates/server]
    API[GraphQL API\ncrates/api]
    CORE[Domain Model\ncrates/core]
    PLUG[Plugin Registry\ncrates/plugins]
    AUTH[Auth Primitives\ncrates/auth]
  end

  subgraph DataPlane[Etat et ordonnancement]
    ST[(Storage\ncrates/storage\nInMemory/Postgres)]
    Q[(Scheduler\ncrates/scheduler\nInMemory/File/Redis/Postgres)]
  end

  UI -->|GET static assets| SV
  UI -->|POST/GET /graphql| SV
  SCM -->|POST /webhooks/scm| SV
  WK -->|GraphQL claim/complete| SV

  SV --> API
  API --> CORE
  API --> AUTH
  API --> PLUG
  API --> ST
  API --> Q

  API -. SCM polling loop .-> ST
  API -. SCM polling loop .-> Q
```

## Flux operationnels

### 1. Build standard

```mermaid
sequenceDiagram
  participant U as User/Admin UI
  participant S as Server (Axum)
  participant A as API Service
  participant Q as Scheduler
  participant D as Storage
  participant W as Worker Externe

  U->>S: Mutation GraphQL run_job
  S->>A: run_job(job_id)
  A->>D: save BuildRecord(Pending)
  A->>Q: enqueue(build_id)

  loop Worker poll
    W->>S: worker_claim_build
    S->>A: claim_build_for_worker
    A->>Q: claim_next
    A->>D: mark Running + save
  end

  W->>S: worker_complete_build(status, logs)
  S->>A: complete_build_for_worker
  A->>D: mark Success/Failed + save
  A->>Q: ack or requeue
```

### 2. Trigger SCM

```mermaid
sequenceDiagram
  participant SCM as SCM Provider
  participant S as Server
  participant A as API Service
  participant D as Storage
  participant Q as Scheduler

  SCM->>S: POST /webhooks/scm
  S->>A: ingest webhook
  A->>D: read webhook security + dedup state
  A->>D: read polling config / jobs
  A->>D: save BuildRecord(Pending)
  A->>Q: enqueue(build_id)
```

## Cartographie des crates

- `crates/server`: bootstrap runtime Axum, montage routes GraphQL et webhook SCM, assets dashboard.
- `crates/api`: schema GraphQL, etat partage et mapping adaptateur entrant (HTTP/GraphQL).
- `crates/application`: use-cases CI et orchestration metier transport-neutre.
- `crates/core`: modele metier (jobs, builds, pipeline DSL, SCM config, technology profiles).
- `crates/storage`: persistence abstraite + implementations InMemory et Postgres.
- `crates/scheduler`: file de builds + backends InMemory, File, Redis, Postgres.
- `crates/worker`: worker externe qui claim/complete les builds via GraphQL.
- `crates/plugins`: contrat et registre plugins (lifecycle + permissions).
- `crates/auth`: primitives d authentification.

## Principes d architecture

- Control-plane GraphQL-only pour les operations CI.
- Entree webhook SCM native separee (`/webhooks/scm`).
- Separation nette entre orchestration API, persistence (storage), et ordonnancement (scheduler).
- Execution de build externalisee via workers dedies (pas de mode embedded).
- Backends remplacables via traits et selection runtime par configuration.

## Convergence hexagonale pragmatique (HEXA-05)

Cette section fixe le graphe cible de dependances pour la phase pragmatique.
Objectif: clarifier les frontieres d architecture sans casser la surface runtime actuelle.

### Graphe cible de dependances (phase pragmatique)

```mermaid
flowchart TD
  subgraph Adapters[Adaptateurs]
    SERVER[crates/server\nHTTP bootstrap + composition root]
    GQL[crates/api/graphql + handlers + state\nGraphQL/HTTP mapping]
    WORKER[crates/worker\nclient GraphQL claim/complete]
  end

  subgraph App[Application]
    USECASE[crates/application/application\nCiUseCases]
    SERVICE[crates/application/service\nCiService + orchestration]
  end

  subgraph Domain[Domain]
    CORE[crates/core\nentities + invariants + shared contracts]
  end

  subgraph PortsAdapters[Ports et adaptateurs sortants]
    STORAGE_PORT[Storage trait]
    SCHED_PORT[Scheduler trait]
    STORAGE_ADP[crates/storage\nInMemory/Postgres]
    SCHED_ADP[crates/scheduler\nInMemory/File/Redis/Postgres]
    PLUG[crates/plugins]
    AUTH[crates/auth]
  end

  SERVER --> GQL
  SERVER --> USECASE
  GQL --> USECASE
  USECASE --> SERVICE
  SERVICE --> CORE
  SERVICE --> STORAGE_PORT
  SERVICE --> SCHED_PORT
  STORAGE_ADP -.implements.-> STORAGE_PORT
  SCHED_ADP -.implements.-> SCHED_PORT
  USECASE --> PLUG
  USECASE --> AUTH
  WORKER --> CORE
```

### Regles de dependance (phase pragmatique)

- Regle 1: les adaptateurs entrants (`server`, `graphql`, `handlers`, `state`) appellent la couche use-case (`application`) et ne contiennent pas d orchestration metier longue.
- Regle 2: la couche application (`crates/application`) depend du domaine (`core`) et des ports (`Storage`, `Scheduler`), jamais d un backend concret.
- Regle 3: les backends concrets (`storage`, `scheduler`) sont choisis au bootstrap (`server`) et passes sous forme de trait objects.
- Regle 4: `worker` consomme des contrats neutres depuis `core` pour les DTO partages; tout couplage restant a `api` doit etre explicite et confine.

### Regles interdites (phase pragmatique)

- Interdit: `core` depend de `api`, `server`, `storage`, `scheduler`, `worker`.
- Interdit: les modules adaptateurs GraphQL/HTTP importent directement des types de backend Postgres/Redis/File.
- Interdit: la logique metier de workflow CI est dupliquee dans `server`, `graphql`, `handlers` ou `worker`.

### Ecarts restants assumes (avant phase stricte)

- Les operations plugin/policy sont partiellement portees par `ApiState` et seront alignees progressivement via la couche use-case.
- Le binaire de benchmark worker peut encore activer un couplage API derriere feature gate (`transport-bench`).
