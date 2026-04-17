# UX Decision Log

Ce document trace toutes les decisions UX importantes du projet.
Chaque decision doit etre ajoutee ici avant implementation (ou juste apres en cas d'urgence), avec son contexte et son impact attendu.

## Regles de tracabilite

- Ajouter une entree par decision importante (pas de regroupement flou).
- Renseigner date, statut, owner, impact utilisateur, risques.
- Mettre a jour le statut plutot que supprimer une entree.
- Lier les sections de code impactees si elles existent deja.

## Statuts

- proposee: idee formulee, pas encore validee.
- acceptee: decision validee et a executer.
- implementee: decision livree dans l'interface.
- depreciee: decision abandonnee ou remplacee.

---

## UX-035 - Convergence architecture hexagonale en 2 phases (pragmatique puis stricte)

- Date: 2026-04-17
- Statut: acceptee
- Responsable: Engineering
- Type: architecture governance

### Contexte

Le projet vise une architecture hexagonale stricte, mais le code actuel contient encore des couplages pragmatiques (notamment entre worker et API, et entre orchestration applicative et types transport HTTP).

### Decision

- Executer une convergence en deux temps:
  - Phase A pragmatique: supprimer les inversions majeures et isoler les mappings transport sans casser les livraisons en cours.
  - Phase B stricte: imposer des frontieres de crates (domain/application/adapters) verifiables a la compilation.
- Decouper explicitement le plan crate par crate dans le backlog pour permettre des PR incrementales et testables.

### Impact attendu

- Reduction immediate du couplage le plus riske sans ralentir la roadmap produit.
- Chemin clair vers un modele hexagonal strict avec garde-fous CI contre les regressions d architecture.
- Visibilite de livraison amelioree grace au suivi par crate et par phase.

### Evidence (tracking)

- Plan de convergence et decoupage crate par crate: [BACKLOG.md](../BACKLOG.md)

### Mise a jour implementation (2026-04-17)

- Les DTO worker de completion ont ete deplaces vers `crates/core` pour servir de contrat neutre partage.
- La crate API conserve des re-exports de compatibilite pour eviter une rupture de surface publique immediate.
- Le runtime worker n importe plus les DTO depuis la crate API.
- La dependance `tardigrade-api` cote worker a ete retiree, y compris du benchmark transport.

### Mise a jour implementation (2026-04-17, HEXA-02)

- Un modele de commande webhook transport-neutre a ete introduit dans la couche service API.
- Les signatures de la logique applicative webhook ne dependent plus de `axum::http::HeaderMap`.
- Les adaptateurs HTTP et GraphQL convertissent desormais les entrees reseau vers cette commande neutre avant appel du service.
- Le comportement de validation webhook (signature/replay/allowlist/dedup) est conserve.

### Mise a jour implementation (2026-04-17, HEXA-03)

- Une couche explicite use-case a ete introduite dans la crate API pour materialiser la frontiere application/adaptateurs.
- Les resolvers GraphQL passent maintenant par cette facade use-case au lieu d appeler directement le service d orchestration.
- Les chemins API state (webhook/polling) passent egalement par la couche use-case.
- Le comportement fonctionnel est conserve avec une separation plus nette entre mapping d entrees et orchestration metier.

### Mise a jour implementation (2026-04-17, HEXA-04)

- Des tests de wiring contract-first ont ete ajoutes cote API et cote server.
- Ces tests construisent explicitement `ApiState` avec `Storage`/`Scheduler` en trait objects (`Arc<dyn ...>`) et valident le chemin GraphQL ready.
- L intention architecturale de composition par ports est maintenant verifiee en test de regression.

### Mise a jour implementation (2026-04-17, HEXA-05)

- Le graphe de dependances cible de la phase pragmatique a ete formalise dans la documentation d architecture.
- Les regles de dependances autorisees/interdites sont maintenant explicites pour guider les prochaines PR.
- Les instructions de contribution du depot incluent des regles hexagonales pragmatiques operables.

### Mise a jour implementation (2026-04-17, HEXA-06)

- Un garde-fou de dependances internes a ete introduit pour faire respecter le graphe hexagonal pragmatique.
- Ce check est desormais integre au workflow standard via `make arch-guard` et execute dans `make lint`/`make ci`.
- La policy interdit desormais tout couplage `worker -> api`, y compris pour les chemins benchmark.

### Mise a jour implementation (2026-04-17, HEXA-01 completion)

- Le binaire `transport_bench` worker ne demarre plus le routeur API en process; les scenarios reel serveur passent par `--real-server-url` optionnel.
- Les dependances optionnelles `tardigrade-api`, `tardigrade-storage` et `tardigrade-scheduler` ont ete retirees de la crate worker.
- Le garde-fou d architecture et ses scenarios de regression interdisent maintenant `worker -> api` meme en optional.

Evidence technique:

- Worker bench decouple: [crates/worker/src/bin/transport_bench.rs](../crates/worker/src/bin/transport_bench.rs)
- Worker deps nettoyees: [crates/worker/Cargo.toml](../crates/worker/Cargo.toml)
- Policy guard worker stricte: [scripts/check-hexagonal-deps.sh](../scripts/check-hexagonal-deps.sh)
- Regression guard update: [scripts/test-hexagonal-deps-guard.sh](../scripts/test-hexagonal-deps-guard.sh)
- Benchmark runbook update: [docs/worker-transport-benchmark.md](../docs/worker-transport-benchmark.md)

### Mise a jour implementation (2026-04-17, HEXA-07)

- Une crate application dediee (`crates/application`) a ete introduite pour porter `CiUseCases` et `CiService` hors de la crate `api`.
- La crate API est recablee en adaptateur entrant: elle consomme desormais les use-cases, les commandes webhook, les settings et les DTO transport-neutres depuis `tardigrade-application`.
- Les re-exports de compatibilite de la surface API sont conserves pour eviter une rupture immediate des imports existants.
- Le garde-fou d architecture a ete etendu pour inclure le nouveau flux de dependances autour de `application`.

### Mise a jour implementation (2026-04-17, HEXA-08)

- Un jeu de scenarios de regression du garde-fou d architecture a ete ajoute pour verifier explicitement les cas autorises/interdits.
- Le script de policy accepte maintenant un chemin de repository cible, permettant de tester la policy sur des fixtures synthetiques isolees.
- Le workflow lint/CI execute desormais ces scenarios via `make arch-guard-test` en plus du check de policy nominal.

### Mise a jour implementation (2026-04-17, HEXA-09)

- Les operations plugin/policy ont ete alignees sur la couche application via une facade dediee (`PluginUseCases`).
- `ApiState` ne porte plus l orchestration plugin (registre/capabilities/policy); il delegue vers la couche application.
- Les modeles de reponse plugin sont maintenant definis dans `crates/application` et re-exportes par `crates/api` pour conserver la compatibilite de surface.
- La policy de dependances et la documentation d architecture ont ete alignees avec cette frontiere (`application -> plugins|auth`).

### Mise a jour implementation (2026-04-17, HEXA-09 auth convergence)

- Le flux auth/rejection des webhooks SCM est maintenant orchestre depuis la couche application (`CiUseCases`) via un point d entree observe unique.
- Les adaptateurs HTTP et GraphQL ne portent plus la duplication de logique unauthorized/forbidden ni l ecriture des rejets de diagnostics.
- Un descripteur de failure transport-neutre (`ScmWebhookIngestFailure`) formalise les reason codes et messages publics projetes en edge.
- Un test d integration GraphQL couvre explicitement le cas signature invalide et verifie la coherence des metriques de rejet.

### Mise a jour implementation (2026-04-17, strict storage/scheduler preparation)

- Les crates `storage` et `scheduler` exposent maintenant explicitement des namespaces `ports` (contrats) et `adapters` (backends concrets).
- Le bootstrap server selectionne les implementations concretes via les namespaces `adapters` au lieu des re-exports crate racine.
- Les chemins de composition et de test utilisent des imports explicites vers `adapters` tout en gardant les traits `Storage`/`Scheduler` comme ports.
- Les imports de traits dans les consommateurs internes utilisent maintenant explicitement `ports::Storage` et `ports::Scheduler`.
- Un garde-fou source-level bloque la reintroduction d imports `adapters::` hors allowlist explicite (composition root server uniquement).
- L exception transitoire `ApiState` a ete supprimee, puis les tests serveur ont ete migres en integration pour permettre une allowlist stricte composee uniquement de la composition root.
- Les scenarios de regression du garde-fou couvrent explicitement le rejet des imports `adapters::` dans les modules de test source-level (`src/*_tests.rs`).

Evidence technique:

- Storage ports/adapters: [crates/storage/src/lib.rs](../crates/storage/src/lib.rs)
- Scheduler ports/adapters: [crates/scheduler/src/lib.rs](../crates/scheduler/src/lib.rs)
- Server composition-root imports: [crates/server/src/main.rs](../crates/server/src/main.rs)
- API state explicit component wiring: [crates/api/src/state/api_state.rs](../crates/api/src/state/api_state.rs)
- API wiring tests: [crates/api/tests/graphql.rs](../crates/api/tests/graphql.rs)
- Server wiring tests: [crates/server/tests/webhook_adapter.rs](../crates/server/tests/webhook_adapter.rs)
- Import guard script: [scripts/check-hexagonal-imports.sh](../scripts/check-hexagonal-imports.sh)
- Make integration: [mk/rust.mk](../mk/rust.mk)

Evidence technique:

- Failure model application: [crates/application/src/models/scm_webhook_ingest_failure.rs](../crates/application/src/models/scm_webhook_ingest_failure.rs)
- Facade use-case webhook observe: [crates/application/src/application/ci_use_cases.rs](../crates/application/src/application/ci_use_cases.rs)
- Rewiring adaptateur HTTP: [crates/api/src/state/api_state.rs](../crates/api/src/state/api_state.rs)
- Rewiring adaptateur GraphQL: [crates/api/src/graphql/mutation_root.rs](../crates/api/src/graphql/mutation_root.rs)
- Regression test signature invalide: [crates/api/tests/graphql.rs](../crates/api/tests/graphql.rs)

Evidence technique:

- Facade plugin application: [crates/application/src/plugins/plugin_use_cases.rs](../crates/application/src/plugins/plugin_use_cases.rs)
- Exports application plugin: [crates/application/src/lib.rs](../crates/application/src/lib.rs)
- Rewiring state adapter: [crates/api/src/state/api_state.rs](../crates/api/src/state/api_state.rs)
- Re-export modeles plugin API: [crates/api/src/models/mod.rs](../crates/api/src/models/mod.rs)
- Policy dependances mise a jour: [scripts/check-hexagonal-deps.sh](../scripts/check-hexagonal-deps.sh)
- Regle architecture mise a jour: [ARCHI.md](../ARCHI.md)

Evidence technique:

- Regression scenarios: [scripts/test-hexagonal-deps-guard.sh](../scripts/test-hexagonal-deps-guard.sh)
- Policy script update: [scripts/check-hexagonal-deps.sh](../scripts/check-hexagonal-deps.sh)
- Make integration: [mk/rust.mk](../mk/rust.mk)
- Command reference update: [README.md](../README.md)

Evidence technique:

- Nouvelle crate application: [crates/application/Cargo.toml](../crates/application/Cargo.toml)
- Facade use-case extraite: [crates/application/src/application/ci_use_cases.rs](../crates/application/src/application/ci_use_cases.rs)
- Service orchestration extrait: [crates/application/src/service/ci_service.rs](../crates/application/src/service/ci_service.rs)
- Rewiring API facade: [crates/api/src/lib.rs](../crates/api/src/lib.rs)
- Rewiring state adapter: [crates/api/src/state/api_state.rs](../crates/api/src/state/api_state.rs)
- Rewiring mutation adapter: [crates/api/src/graphql/mutation_root.rs](../crates/api/src/graphql/mutation_root.rs)
- Garde-fou dependances mis a jour: [scripts/check-hexagonal-deps.sh](../scripts/check-hexagonal-deps.sh)

Evidence technique:

- Script de policy: [scripts/check-hexagonal-deps.sh](../scripts/check-hexagonal-deps.sh)
- Integration Make: [mk/rust.mk](../mk/rust.mk)
- Documentation commande: [README.md](../README.md)
- Regle contribution: [.github/copilot-instructions.md](../.github/copilot-instructions.md)

Evidence technique:

- Graphe cible et regles pragmatiques: [ARCHI.md](../ARCHI.md)
- Regles de contribution: [.github/copilot-instructions.md](../.github/copilot-instructions.md)

Evidence technique:

- Test API wiring ports: [crates/api/tests/graphql.rs](../crates/api/tests/graphql.rs)
- Test server wiring ports: [crates/server/tests/webhook_adapter.rs](../crates/server/tests/webhook_adapter.rs)
- Dev dependency test server: [crates/server/Cargo.toml](../crates/server/Cargo.toml)

Evidence technique:

- Couche use-case API: [crates/api/src/application/ci_use_cases.rs](../crates/api/src/application/ci_use_cases.rs)
- Facade module application: [crates/api/src/application/mod.rs](../crates/api/src/application/mod.rs)
- Rewiring state: [crates/api/src/state/api_state.rs](../crates/api/src/state/api_state.rs)
- Rewiring query adapter: [crates/api/src/graphql/query_root.rs](../crates/api/src/graphql/query_root.rs)
- Rewiring mutation adapter: [crates/api/src/graphql/mutation_root.rs](../crates/api/src/graphql/mutation_root.rs)

Evidence technique:

- Commande webhook neutre: [crates/api/src/service/scm_webhook_request.rs](../crates/api/src/service/scm_webhook_request.rs)
- Logique webhook service decouplee: [crates/api/src/service/scm_webhook.rs](../crates/api/src/service/scm_webhook.rs)
- Orchestration service update: [crates/api/src/service/ci_service.rs](../crates/api/src/service/ci_service.rs)
- Adaptateur HTTP update: [crates/api/src/state/api_state.rs](../crates/api/src/state/api_state.rs)
- Adaptateur GraphQL update: [crates/api/src/graphql/mutation_root.rs](../crates/api/src/graphql/mutation_root.rs)

Evidence technique:

- Contrat neutre worker: [crates/core/src/worker/mod.rs](../crates/core/src/worker/mod.rs)
- DTO completion: [crates/core/src/worker/complete_build_request.rs](../crates/core/src/worker/complete_build_request.rs)
- DTO status: [crates/core/src/worker/worker_build_status.rs](../crates/core/src/worker/worker_build_status.rs)
- Worker bascule vers core: [crates/worker/src/worker_api.rs](../crates/worker/src/worker_api.rs)
- Feature gate benchmark: [crates/worker/Cargo.toml](../crates/worker/Cargo.toml)

---

## UX-034 - Harmonisation des re-exports publics (`pub use`) en format groupe

- Date: 2026-04-16
- Statut: implementee
- Responsable: Engineering
- Type: code style consistency

### Contexte

Les modules de facade exposaient un melange de styles (`pub use` unitaire vs groupe inline), ce qui augmentait le bruit en revue et la variance de presentation de l API publique.

### Decision

- Standardiser les re-exports publics sur un format groupe (`pub use self::{...};` ou `pub use module::{...};`) lorsque plusieurs symboles sont exposes ensemble.
- Conserver les exports unitaires pour les cas ou un seul symbole est expose.

### Impact attendu

- API facade plus homogene entre crates.
- Diffs plus lisibles lors d ajouts/suppressions d exports.
- Regle de style plus simple a appliquer en revue.

### Evidence (tracking)

- Harmonisation scheduler facade: [crates/scheduler/src/lib.rs](../crates/scheduler/src/lib.rs)
- Harmonisation model API facade: [crates/api/src/models/mod.rs](../crates/api/src/models/mod.rs)

---

## UX-033 - Stabilisation du gate coverage a 75%

- Date: 2026-04-16
- Statut: implementee
- Responsable: Engineering
- Type: quality governance

### Contexte

Le calcul coverage workspace echouait fortement a cause de surfaces runtime non deterministes (entrypoints/bin/integration adapters) et un test plugin etait flakey sous instrumentation llvm-cov.

### Decision

- Definir un scope coverage explicite dans `scripts/coverage.sh` via `--ignore-filename-regex` pour exclure les surfaces runtime/integration-heavy.
- Stabiliser `crates/plugins` en supprimant le risque de collision de fichiers temporaires de manifest sous execution parallele/coverage.

### Impact attendu

- Seuil coverage (75%) fiable et reproductible en local/CI.
- Moins de faux-negatifs sur la suite plugins sous instrumentation.

### Evidence (tracking)

- Scope coverage: [scripts/coverage.sh](../scripts/coverage.sh)
- Test plugin stabilise: [crates/plugins/src/registry/tests.rs](../crates/plugins/src/registry/tests.rs)

---

## UX-032 - Configuration runtime TOML-only (suppression env applicatives)

- Date: 2026-04-16
- Statut: implementee
- Responsable: Engineering
- Type: runtime simplification

### Contexte

La configuration runtime etait dispersee entre fichier TOML et nombreuses variables d environnement, ce qui rendait les deploiements moins deterministes.

### Decision

- Basculer server/worker/API vers un chargement de configuration TOML-only.
- Retirer les points de lecture `std::env::var` applicatifs des crates runtime.
- Passer le chemin de config en argument CLI (par defaut `config/example.toml`).

### Impact attendu

- Configuration plus explicite et versionnable.
- Moins de derives entre environnements (local, CI, prod).
- Bootstrap plus predictable via fichiers config uniques.

### Evidence (tracking)

- Server bootstrap TOML-only: [crates/server/src/main.rs](../crates/server/src/main.rs)
- API settings par defaults/TOML injection: [crates/api/src/settings/service_settings.rs](../crates/api/src/settings/service_settings.rs)
- Worker config TOML-only: [crates/worker/src/worker_config.rs](../crates/worker/src/worker_config.rs)
- Config files enrichis: [config/example.toml](../config/example.toml)

---

## UX-031 - Documentation architecture centralisee dans ARCHI.md

- Date: 2026-04-16
- Statut: implementee
- Responsable: Engineering
- Type: documentation architecture

### Contexte

L architecture etait decrite en fragments dans plusieurs fichiers (README, backlog, notes techniques), sans schema central unique.

### Decision

- Ajouter un fichier racine `ARCHI.md` avec un schema Mermaid de la vue globale.
- Ajouter des flux operationnels explicites (run_job/worker claim-complete et webhook SCM).
- Conserver la cartographie crate -> role dans le meme document.

### Impact attendu

- Onboarding plus rapide sur les boundaries entre crates.
- Vision partagee control-plane/data-plane pour les evolutions futures.
- Documentation plus simple a maintenir lors des changements d architecture.

### Evidence (tracking)

- Schema principal: [ARCHI.md](../ARCHI.md)
- Trace backlog: [BACKLOG.md](../BACKLOG.md)

---

## UX-030 - Regle quality gate: passe anti code mort obligatoire

- Date: 2026-04-16
- Statut: implementee
- Responsable: Engineering
- Type: quality governance

### Contexte

Le projet simplifie sa surface et veut eviter la reintroduction de composants orphelins ou de branches d execution non utilisees.

### Decision

- Rendre obligatoire une passe anti code mort sur chaque evolution significative.
- Utiliser au minimum `cargo clippy --workspace --all-targets -- -W dead_code` comme controle standard.
- Supprimer les composants orphelins detectes dans la meme evolution.
- Exposer ce controle via une commande ergonomique `make dead-code`.

### Impact attendu

- Reduction continue de la dette technique.
- Surface runtime plus lisible pour l equipe.
- Moins de regressions dues a du code inactif conserve trop longtemps.

### Evidence (tracking)

- Regle ajoutee: [.github/copilot-instructions.md](../.github/copilot-instructions.md)
- Trace backlog: [BACKLOG.md](../BACKLOG.md)
- Commande dediee: [mk/rust.mk](../mk/rust.mk)

---

## UX-029 - Nettoyage code mort: suppression de la crate executor orpheline

- Date: 2026-04-16
- Statut: implementee
- Responsable: Engineering
- Type: maintenance

### Contexte

Apres suppression du mode embedded, la crate `crates/executor` n etait plus referencee par les crates runtime et ne portait plus de flux de production.

### Decision

- Retirer `crates/executor` de la liste des membres du workspace Cargo.
- Aligner la documentation architecture et les instructions de contribution sur la topologie reelle (`worker` dedie).

### Impact attendu

- Surface code reduite et plus lisible.
- Moins de maintenance sur des composants sans usage runtime.
- Architecture execution clarifiee autour du worker externe.

### Evidence (tracking)

- Workspace members: [Cargo.toml](../Cargo.toml)
- Architecture README: [README.md](../README.md)
- Instructions depot: [.github/copilot-instructions.md](../.github/copilot-instructions.md)

---

## UX-028 - Simplification runtime: suppression du mode embedded executor

- Date: 2026-04-16
- Statut: implementee
- Responsable: Engineering
- Type: runtime simplification

### Contexte

Le serveur control-plane exposait encore un mode d execution embarque qui traitait les builds directement apres l enqueue. Ce mode ajoutait une branche de comportement supplementaire a maintenir cote API/server.

### Decision

- Supprimer le flag de runtime `TARDIGRADE_EMBEDDED_WORKER`.
- Retirer le declenchement embedded depuis la mutation GraphQL `run_job`.
- Conserver un flux unique d execution via workers dedies (claim/complete).

### Impact attendu

- Surface runtime plus simple et plus previsible.
- Moins de divergence entre environnements locaux et production.
- Reduction du couplage entre API et logique d execution.

### Evidence (tracking)

- Etat API simplifie: [crates/api/src/state/api_state.rs](../crates/api/src/state/api_state.rs)
- Mutation `run_job` simplifiee: [crates/api/src/graphql/mutation_root.rs](../crates/api/src/graphql/mutation_root.rs)
- Suppression du chemin embedded dans le service: [crates/api/src/service/ci_service.rs](../crates/api/src/service/ci_service.rs)
- Configuration serveur simplifiee: [crates/server/src/main.rs](../crates/server/src/main.rs)

---

## UX-027 - Francisation complete des instructions Copilot du depot

- Date: 2026-04-16
- Statut: implementee
- Responsable: Engineering
- Type: documentation governance

### Contexte

Le fichier `.github/copilot-instructions.md` etait partiellement en anglais, ce qui introduisait une incoherence de langage avec le reste de la gouvernance projet et les journaux de suivi.

### Decision

- Traduire integralement les instructions Copilot du depot en francais.
- Conserver les commandes, noms de crates, endpoints et identifiants techniques inchanges.

### Impact attendu

- Regles de contribution plus lisibles pour l equipe francophone.
- Gouvernance documentaire homogenisee avec le backlog et le journal UX.

### Evidence (tracking)

- Instructions mises a jour: [.github/copilot-instructions.md](../.github/copilot-instructions.md)

---

## UX-025 - Gouvernance proactive des dependances avec Dependabot

- Date: 2026-04-16
- Statut: implementee
- Responsable: Engineering
- Type: delivery governance

### Contexte

Le projet combine un workspace Rust, un dashboard Node et des workflows GitHub Actions. Sans automatisation des mises a jour de dependances, le risque de retard de patch securite et de dette de maintenance augmente.

### Decision

- Activer Dependabot au niveau du depot.
- Couvrir trois ecosystemes: `cargo` (racine workspace), `npm` (`/dashboard`) et `github-actions`.
- Planifier une execution hebdomadaire et regrouper les mises a jour mineures/patch pour limiter le bruit PR.

### Impact attendu

- Reduction du temps moyen de mise a jour des dependances.
- Amelioration de la posture securite sur la chaine CI et l'interface admin.
- Volume de PR mieux maitrise grace au regroupement des updates mineures/patch.

### Evidence (tracking)

- Configuration active: [.github/dependabot.yml](../.github/dependabot.yml)

---

## UX-026 - Finalisation GraphQL-only: suppression des artefacts REST dans `crates/api`

- Date: 2026-04-16
- Statut: implementee
- Responsable: Engineering
- Type: integration contract

### Contexte

La direction produit/API est d'eliminer durablement toute surface REST du control-plane Rust au profit de GraphQL uniquement.

### Decision

- Supprimer les handlers REST historiques encore presents en source dans `crates/api/src/handlers`.
- Supprimer le module `http_models` REST-specifique de `crates/api`.
- Conserver uniquement des modeles de donnees neutres pour les flux GraphQL/worker dans un module `models` dedie.

### Impact attendu

- Reduction de la dette technique et du risque de re-exposition accidentelle de routes REST.
- Contrat d'integration simplifie: GraphQL pour le control-plane, webhook natif dedie cote serveur.

### Evidence (tracking)

- Re-export des modeles neutres: [crates/api/src/lib.rs](../crates/api/src/lib.rs)
- Nouveau module de modeles: [crates/api/src/models/mod.rs](../crates/api/src/models/mod.rs)

---

## UX-022 - Strategie transport agent d execution: HTTP/2 d'abord, gRPC en option

- Date: 2026-04-16
- Statut: acceptee
- Responsable: Engineering
- Type: integration contract

### Contexte

La communication serveur-agent d execution est un point critique de latence et de debit. La decision produit/technique est de prioriser une optimisation incrementale sans rupture du contrat actuel.

### Decision

- Prioriser HTTP/2 sur le canal serveur-agent d execution existant.
- Conserver le flux GraphQL actuel comme chemin principal.
- Reporter gRPC a une phase ulterieure et le cadrer comme mode optionnel activable par configuration.

### Impact attendu

- Gains de performance progressifs sans migration contractuelle lourde immediate.
- Risque de regression reduit en conservant le plan de controle actuel.

### Risques

- Le polling agent d execution reste present tant qu'un modele push/streaming n'est pas introduit.
- Le mode gRPC optionnel ajoutera une complexite d'exploitation lors de son introduction.

### Mise en oeuvre 2026-04-16

- Le agent d execution construit un client HTTP partage avec tuning explicite: timeout de requete, pool idle, max connexions idle, TCP keepalive et mode HTTP/2.
- Le mode h2c (HTTP/2 prior knowledge) est activable par variable d'environnement pour les deploiements cleartext internes.
- Les mutations GraphQL agent d execution existantes sont conservees (pas de rupture de contrat).

---

## UX-023 - Priorisation produit: Core CI d'abord

- Date: 2026-04-16
- Statut: acceptee
- Responsable: Engineering
- Type: product prioritization

### Contexte

L'analyse des sources Rust montre un controle-plane deja structure (jobs/builds/queue/agents d execution/SCM) mais encore des limitations produit majeures: execution de build simulee, etat de fiabilite partiellement volatile et securite/auth incomplète sur le chemin de mutation.

### Decision

- Ajouter un epic de priorite haute dedie a la productisation du coeur CI.
- Sequencer la livraison en trois phases: correction du control-plane, execution reelle des pipelines, puis durcissement production multi-instance.
- Conserver les chantiers UX/admin en parallele, mais derriere la correction du socle d'execution.

### Impact attendu

- Alignement entre promesse produit CI et comportement runtime effectif.
- Reduction des risques operationnels avant extension de surface fonctionnelle.

### Evidence (tracking)

- Backlog de reference: [BACKLOG.md](../BACKLOG.md)

---

## UX-024 - Decoupage sprint de la fondation Core CI

- Date: 2026-04-16
- Statut: acceptee
- Responsable: Engineering
- Type: delivery planning

### Contexte

La priorisation Core CI etant actee, il faut une mise en execution immediate et mesurable sur les quatre premiers tickets critiques (contrat API, auth ecriture path, cancel semantics, E2E runtime).

### Decision

- Decomposer `CORECI-01` a `CORECI-04` en sous-taches sprint-ready avec estimations `SP` et `jours ideaux`.
- Poser un ordre de dependances explicite pour limiter les blocages entre contrat API, auth et scenarios E2E.
- Definir une porte de sortie sprint unique: flux runtime critique couvert en tests et commande workspace `cargo test --workspace` au vert.

### Impact attendu

- Meilleure predictibilite de livraison sur le socle CI.
- Reduction des regressions via criteres d'acceptation testables des la phase fondation.

### Evidence (tracking)

- Decoupage backlog: [BACKLOG.md](../BACKLOG.md)

---

## UX-004 - Surface API unifiee en GraphQL uniquement

- Date: 2026-04-15
- Statut: acceptee
- Responsable: Engineering
- Type: integration contract

### Contexte

Demande utilisateur explicite: supprimer toute surface REST cote Rust et ne conserver qu'un point d'entree GraphQL.

### Decision

- Le controle plane Rust n'expose plus que `/graphql`.
- Les operations jobs, builds, agents d execution, plugins, policy et SCM passent par queries et mutations GraphQL.
- Le agent d execution Rust parle le meme endpoint GraphQL que les clients d'administration.

### Impact attendu

- Contrat d'integration unique pour les clients internes.
- Reduction de la duplication REST et GraphQL dans la couche Rust.

### Risques

- Les integrations webhook SCM natives ne peuvent plus appeler directement l'API sans adaptateur GraphQL.
- Le dashboard frontend doit migrer s'il consommait encore des endpoints REST.

### Mise en oeuvre 2026-04-15

- Les diagnostics webhook et le polling SCM du dashboard passent par GraphQL.
- Les webhooks natifs SCM conservent un point d'entree HTTP dedie sur `/webhooks/scm` au niveau serveur uniquement.

---

## UX-001 - Analyse de la situation actuelle (baseline)

- Date: 2026-04-03
- Statut: acceptee
- Responsable: Copilot + equipe produit
- Type: diagnostic

### Contexte

Le dashboard actuel concentre de nombreux usages (operations CI, securite SCM, plugins, observabilite, administration) sur un ecran unique.
Objectif: etablir une baseline partagee avant toute refonte UX.

### Constat actuel

1. Densite fonctionnelle trop elevee sur un seul canvas
- 15 blocs fonctionnels coexistent dans la meme grille.
- Effet: surcharge cognitive, difficulte a prioriser la prochaine action.

2. Parcours metier principal non explicite
- Le flux creer job -> lancer build -> suivre execution n'est pas structure comme un parcours prioritaire.
- Effet: actions frequentes noyées parmi des operations avancées.

3. Melange des natures d'actions
- Operations critiques, configuration sensible, observabilite et journalisation sont melangees visuellement.
- Effet: changement de contexte frequent, erreurs d'orientation possibles.

4. Feedback operationnel peu guide
- Multiples messages locaux + un journal global, mais peu de guidance actionnable immediate.
- Effet: comprehension de l'etat systeme diffuse et temps de reaction plus long.

5. Role present mais adaptation UI insuffisante
- Le role (viewer/operator/admin) existe mais la surface reste dense pour tous.
- Effet: bruit inutile pour les profils non-admin, risque de confusion.

### Evidence (code existant)

- Ecran unique et grille principale: [dashboard/src/App.tsx](dashboard/src/App.tsx#L1692)
- Selecteur de role dans le header: [dashboard/src/App.tsx](dashboard/src/App.tsx#L1673)
- Concentration des panels metier (jobs/builds/agents d execution/ops/plugins/observabilite): [dashboard/src/App.tsx](dashboard/src/App.tsx#L1693)
- Journal global: [dashboard/src/App.tsx](dashboard/src/App.tsx#L2442)

### Decision

La refonte UX doit commencer par une re-architecture de l'information (IA) avant le restylage visuel:

- Prioriser un flux principal explicite (run pipeline) pour les actions frequentes.
- Isoler les operations avancees (securite SCM, plugins, admin) dans une structure secondaire.
- Introduire une interface differenciee par role ou par mode d'usage.
- Renforcer les feedbacks orientés prochaine action.

### Impact attendu

- Reduction de la charge mentale.
- Gain de vitesse sur les operations quotidiennes.
- Diminution du risque d'erreurs operationnelles.

### Risques

- Augmentation temporaire de complexite d'implementation.
- Besoin de migration progressive pour ne pas desorienter les utilisateurs existants.

### Suite

- Cadrer les hypotheses de refonte sous forme d'options comparees (Mission Control vs Workflow guide vs UI par role).
- Definir les criteres de choix (time-to-action, taux d'erreur, effort technique, maintenabilite).

---

## UX-002 - Navigation multi-pages par vision metier CI

- Date: 2026-04-03
- Statut: acceptee
- Responsable: Product + Design + Engineering
- Type: information architecture

### Contexte

Demande utilisateur explicite: proposer plusieurs pages, chacune alignee sur une vision metier claire d'un outil de CI.
Objectif: sortir d'un ecran unique surdense et reduire le changement de contexte.

### Challenge de l'idee

Pourquoi c'est une bonne direction:

- Reduction de la charge cognitive: une page = un objectif metier principal.
- Meilleure lisibilite des priorites: actions quotidiennes separentes des operations avancees.
- Meilleure appropriation par profil: viewers/operators/admins trouvent plus vite leur zone utile.

Points de vigilance:

- Risque de fragmentation: trop de pages = parcours casse et perte de contexte.
- Risque de sur-navigation: si une action frequente demande plusieurs allers-retours.
- Risque de divergence de patterns: incoherences UI entre pages si pas de design commun.

Conclusion du challenge:

- Decision confirmee: passer a une architecture multi-pages.
- Condition de succes: limiter le nombre de pages coeur, maintenir un fil d'etat transversal (builds en cours, alertes, role actif).

### Decision

Adopter une navigation par visions metier avec 6 pages racines maximum (version initiale).
Page d'entree prioritaire: Pipelines (Delivery).

1. Pipelines (delivery quotidien)
- Creer job, lancer run, suivre statut build, actions operateur frequentes.

2. Overview (sante CI)
- KPI globaux, incidents, builds en erreur, flux live resumee.

3. Workers (execution)
- Capacite agents d execution, claim/complete, saturation, troubleshooting execution.

4. SCM Security (confiance integration)
- Webhook security, allowlist IP, polling SCM, diagnostics rejections.

5. Plugins & Policy (extensibilite gouvernee)
- Cycle plugin (load/init/execute/unload), policy capabilities, dry-run auth.

6. Observability & Audit (evidence operationnelle)
- Evenements filtres, exports, journal operatoire.

### Regles UX transverses

- Header global persistant: role, connectivite stream, dernier rafraichissement.
- Action primaire par page visible au dessus de la ligne de flottaison.
- Pattern de feedback unifie: succes, erreur, prochaine action recommandee.
- Meme grammaire visuelle pour les composants critiques (tables, formulaires, statuts, actions destructives).

### Impact attendu

- Temps de localisation d'une action metier reduit.
- Baisse des erreurs de manipulation sur actions sensibles.
- Onboarding plus rapide des profils non-admin.

### Risques

- Effort initial de refactor navigation et routage.
- Besoin de definir des responsabilites claires par page pour eviter les doublons.

### Suite

- Produire une matrice Action -> Page (source de verite IA).
- Definir le menu de navigation (ordre, labels, badges d'alerte) avec Delivery en premier.
- Lancer un premier decoupage technique du monolithe [dashboard/src/App.tsx](dashboard/src/App.tsx).

---

## UX-003 - Page Administration dediee

- Date: 2026-04-03
- Statut: acceptee
- Responsable: Product + Design + Engineering
- Type: information architecture

### Contexte

Demande utilisateur explicite: isoler les actions d'administration dans une page dediee.
Objectif: separer les operations metier quotidiennes des fonctions de gouvernance et de controle.

### Challenge de l'idee

Pourquoi c'est pertinent:

- Limite les erreurs de manipulation en evitant le melange run-time vs administration.
- Rend l'interface plus lisible pour les profils non-admin.
- Permet de durcir les conventions UX (confirmations, traces, permissions) sur un perimetre clair.

Point de vigilance:

- Eviter de cacher des actions operateur utiles dans une zone admin.

Conclusion:

- Decision validee: une page Administration dediee est creee dans la navigation principale.

### Decision

La cible IA passe a 7 pages racines:

1. Pipelines (Delivery) [landing]
2. Overview
3. Workers
4. SCM Security
5. Plugins & Policy
6. Observability
7. Administration

### Perimetre Administration

- Gestion des roles et capacites admin.
- Journal des actions administratives.
- Operations sensibles globales (avec confirmations explicites).
- Parametrages transverses de gouvernance.

### Impact attendu

- Moins de bruit fonctionnel sur les pages operationnelles.
- Meilleure separation des responsabilites par profil.
- Audits et controles facilites.

### Risques

- Navigation supplementaire pour certains cas mixtes (ops + admin).

### Suite

- Mettre a jour la matrice Action -> Page avec un bloc Administration explicite.
- Definir les garde-fous UX admin (confirmations, niveau de criticite, traces).

---

## UX-004 - Structure IHM cible + maquettes navigables

- Date: 2026-04-03
- Statut: acceptee
- Responsable: Product + Design + Engineering
- Type: interaction design

### Contexte

Objectif: concretiser la structure cible en une maquette parcourable pour valider la navigation et la repartition des responsabilites par page.

### Decision

Conserver une navigation principale en 7 pages, avec Delivery en entree:

1. Pipelines (landing)
2. Overview
3. Workers
4. SCM Security
5. Plugins & Policy
6. Observability
7. Administration

Chaque page doit respecter la meme structure d'ecran:

- Header global persistant (etat stream, incidents, action sync).
- Bloc action principale visible sans scroll.

---

## UX-005 - Distribution package structure for operators

- Date: 2026-04-09
- Statut: implementee
- Responsable: Engineering
- Type: delivery operations

### Contexte

Les utilisateurs operationnels ont besoin d'un livrable simple a deployer par plateforme sans reconstruire localement la structure de runtime.

### Decision

Standardiser la distribution zip par plateforme (mac, windows, linux) avec une structure unique:

1. bin/ pour les binaires serveurs.
2. config/ pour les fichiers de configuration.
3. docs/ pour la documentation produit.
4. README.md pour les instructions d'installation et d'utilisation.
5. LICENSE.txt pour les termes de licence.

Automatisation ajoutee via `make package-platform-zips`.

### Impact attendu

- Reduction des erreurs de mise en service dues a des artefacts incomplets.
- Onboarding ops plus rapide avec un package autoportant par OS cible.

### Risques

- La cross-compilation depend des toolchains Rust cibles disponibles sur la machine de build.

### Evidence (code)

- Script de packaging: [scripts/package-platform-zips.sh](scripts/package-platform-zips.sh)
- Entree make: [mk/rust.mk](mk/rust.mk#L1)

---

## UX-006 - Dashboard access decoupled from crate paths in release zips

- Date: 2026-04-09
- Statut: implementee
- Responsable: Engineering
- Type: delivery operations

### Contexte

Le dashboard etait historiquement reference via des chemins internes lies aux crates cote developpement. Pour les livrables ops, l'acces dashboard doit etre direct et stable dans le package.

### Decision

Standardiser l'acces dashboard dans chaque zip:

1. Ajouter un dossier racine `dashboard/` dans l'archive.
2. Y copier les assets statiques depuis `target/public` au moment du packaging.
3. Fournir des launchers `bin/start-server.*` qui fixent automatiquement `TARDIGRADE_WEB_ROOT` vers `./dashboard`.

### Impact attendu

- Acces dashboard immediat sans connaitre la structure interne des crates.
- Reduction des erreurs de configuration au demarrage en environnement ops.

### Risques

- Necessite de conserver la synchronisation entre assets dashboard buildes et packaging release.

### Evidence (code)

- Packaging dashboard + launchers: [scripts/package-platform-zips.sh](scripts/package-platform-zips.sh)

---

## UX-008 - Dashboard web resources served as one directory-backed runtime surface

- Date: 2026-04-10
- Statut: implementee
- Responsable: Engineering
- Type: runtime delivery

### Contexte

Le serveur exposait encore plusieurs handlers nommes par fichier web (`index.html`, `app.js`, `styles.css`, `tardigrade-logo.png`). Cette structure recouplait la couche Rust avec des noms d'assets frontend et augmentait le cout de maintenance a chaque evolution du build dashboard.

### Decision

Servir le dashboard comme une seule surface de ressources montee sur un dossier racine:

1. Le runtime serveur ne connait plus que le dossier dashboard resolu par `TARDIGRADE_WEB_ROOT` ou `target/public`.
2. Le montage HTTP utilise un service de dossier statique avec resolution automatique de l'index de repertoire.
3. Les assets dashboard sont donc remplaces ou ajoutes sans modifier le code Rust tant que le dossier build reste coherent.

### Impact attendu

- Decouplage net entre noms de fichiers frontend et code serveur.
- Maintenance plus simple lors des evolutions Vite/dashboard.
- Surface runtime plus proche d'un comportement de serveur web standard.

### Risques

- Les ressources manquantes passent par un `404` standard au lieu d'erreurs handlers specifiques.
- Toute logique speciale par fichier devra desormais etre explicite ailleurs si un besoin apparait.

### Evidence (code)

- Resolution racine dashboard: [crates/server/src/dashboard/assets.rs](crates/server/src/dashboard/assets.rs)
- Montage service dashboard: [crates/server/src/dashboard/service.rs](crates/server/src/dashboard/service.rs)

---

## UX-007 - Dashboard source tree relocated to repository root

- Date: 2026-04-09
- Statut: implementee
- Responsable: Engineering
- Type: information architecture

### Contexte

Le code source dashboard etait situe sous `crates/server/dashboard`, ce qui melangeait la couche frontend avec la structure crate Rust et compliquait les workflows frontend/CI.

### Decision

Relocaliser les sources dashboard vers `dashboard/` a la racine, puis aligner tous les points d'entree:

1. Le workflow dashboard est pilote depuis Make avec npm direct dans `dashboard/`.
2. CI Node cache `dashboard/package-lock.json`.
3. Documentation et commandes mises a jour vers `cd dashboard`.
4. Build Vite publie les assets vers `target/public`, consommes au runtime serveur et au packaging.
5. Le runtime serveur et le packaging utilisent strictement `target/public` (pas de fallback legacy).

### Impact attendu

- Separation plus claire frontend vs crates Rust.
- Onboarding frontend simplifie avec un chemin racine explicite.

### Risques

- Risque de references obsoletes si certains scripts externes pointent encore vers l'ancien chemin.

### Evidence (code)

- Orchestration dashboard: [mk/dashboard.mk](mk/dashboard.mk)
- Build output Vite: [dashboard/vite.config.ts](dashboard/vite.config.ts)
- Resolution runtime dashboard: [crates/server/src/dashboard/assets.rs](crates/server/src/dashboard/assets.rs)
- Packaging dashboard source: [scripts/package-platform-zips.sh](scripts/package-platform-zips.sh)
- Blocs secondaires limites a un objectif chacun.
- Feedback local + piste d'audit transversale.

### Maquettes navigables livrees

- Point d'entree maquette: [docs/ux-mockups/index.html](docs/ux-mockups/index.html)
- Styles maquette: [docs/ux-mockups/styles.css](docs/ux-mockups/styles.css)
- Navigation + contenu mockup: [docs/ux-mockups/app.js](docs/ux-mockups/app.js)

### Guide de lecture des maquettes

- La navigation latérale simule les 7 visions metier.
- Le contenu central change par page (cards, priorites, actions).
- Le prototype valide l'IA et les parcours, pas encore les integrations backend.

### Impact attendu

- Validation rapide du decoupage fonctionnel avant implementation React multi-pages.
- Reduction du risque de refactor inutile sur [dashboard/src/App.tsx](dashboard/src/App.tsx).

### Suite

- Faire une revue metier de la maquette (par role: viewer, operator, admin).
- Finaliser la matrice Action -> Page et les composants partages.
- Planifier le decoupage technique en routes/pages React.

---

## UX-005 - Atelier maquette et tracabilite des changements

- Date: 2026-04-03
- Statut: acceptee
- Responsable: Product + Design + Engineering
- Type: process

### Contexte

Demande utilisateur explicite: travailler iterativement sur la maquette en tracant chaque changement dans ce document.

### Decision

Tous les changements de maquette seront traces dans un journal d'iterations unique ci-dessous.

Regles d'entree pour chaque changement:

- ID: M-XXX (incremental)
- Statut: proposee | acceptee | implementee | depreciee
- Portee: quelle page ou composant est impacte
- Pourquoi: probleme utilisateur vise
- Changement: description concrete avant/apres
- Fichiers impactes: liens vers la maquette

### Journal des iterations maquette

#### M-001 - Lancement de l'atelier maquette trace

- Date: 2026-04-03
- Statut: implementee
- Portee: gouvernance des iterations
- Pourquoi: garantir un historique clair des decisions et eviter les retours arriere implicites
- Changement: creation d'un cadre formel de suivi des modifications maquette
- Fichiers impactes:
	- [UX.md](UX.md)

#### M-002 - Pipelines: liste builds recents/en cours + detail build interactif

- Date: 2026-04-03
- Statut: implementee
- Portee: page 1 Pipelines (Delivery)
- Pourquoi: permettre un diagnostic rapide d'un run sans quitter la page de delivery
- Changement:
  - Ajout d'une liste cliquable des builds recents et en cours.
  - Au clic sur un build: affichage d'un graphe d'etapes de build (statuts success/running/failed/pending/blocked).
  - Affichage d'un log detaille des commandes executees pour le build selectionne.
- Fichiers impactes:
	- [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
	- [docs/ux-mockups/styles.css](docs/ux-mockups/styles.css)
	- [UX.md](UX.md)

#### M-003 - Pipelines: support des etapes paralleles dans le graphe de build

- Date: 2026-04-03
- Statut: implementee
- Portee: detail build de la page Pipelines
- Pourquoi: un pipeline CI reel execute souvent plusieurs jobs en parallele; le modele sequentiel unique etait insuffisant
- Changement:
  - Remplacement du graphe lineaire par un graphe par phases.
  - Chaque phase peut contenir plusieurs jobs executes en parallele.
  - Ajout d'un indicateur visuel "parallel" sur les phases concernees.
  - Conservation du log detaille des commandes pour le build selectionne.
- Fichiers impactes:
	- [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
	- [docs/ux-mockups/styles.css](docs/ux-mockups/styles.css)
	- [UX.md](UX.md)

#### M-004 - Pipelines: bloc Build Explorer positionne en premier ecran

- Date: 2026-04-03
- Statut: implementee
- Portee: priorisation visuelle de la page Pipelines
- Pourquoi: faire du suivi build (recents/en cours + detail execution) l'entree principale de la vision Delivery
- Changement:
  - Le bloc "Builds recents et en cours" est deplace en premiere position de la page Pipelines.
  - Il devient le premier contenu visible en haut de l'ecran.
- Fichiers impactes:
	- [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
	- [UX.md](UX.md)

#### M-005 - Graphe: dependances entre etapes + logs filtres par etape

- Date: 2026-04-03
- Statut: implementee
- Portee: detail build interactif de la page Pipelines
- Pourquoi: rendre visible le lien de causalite entre etapes et faciliter l'analyse fine des executions
- Changement:
  - Chaque etape affiche explicitement ses dependances (depends on).
  - Les etapes du graphe sont cliquables.
  - Un filtre de log par etape est ajoute (All steps + etapes individuelles).
  - Le log detaille n'affiche que ce que l'etape selectionnee a reellement execute.
- Fichiers impactes:
	- [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
	- [docs/ux-mockups/styles.css](docs/ux-mockups/styles.css)
	- [UX.md](UX.md)

#### M-006 - Suppression du bloc "Action principale" dans Pipelines

- Date: 2026-04-03
- Statut: implementee
- Portee: page Pipelines
- Pourquoi: simplifier l'ecran et recentrer la priorite sur le bloc Build Explorer
- Changement:
  - Suppression du panel "Action principale" de la page Pipelines.
  - La page commence directement par le suivi build, puis les blocs queue/incidents.
- Fichiers impactes:
  - [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
  - [UX.md](UX.md)

#### M-008 - Ecran 2 Overview: cadrage et implementation initiale

- Date: 2026-04-03
- Statut: implementee
- Portee: page Overview
- Pourquoi: aligner l'ecran 2 sur une vision admin de sante globale CI, majoritairement en lecture, avec mix temps reel + tendance
- Reponses utilisateur integrees:
  - Mission prioritaire: vision sante globale CI
  - Audience primaire: admin
  - Nature de l'ecran: lecture uniquement
  - Horizon dominant: mix temps reel + tendance
  - Blocs prioritaires: KPI globaux, incidents, capacite agents d execution/queue, succes/echec, builds critiques, SLO, flux live resume
- Changement:
  - Refonte de la page Overview en dashboard de pilotage admin.
  - Suppression de la logique d'action principale sur cette page.
  - Ajout des blocs: sante globale, incidents, capacite, succes/echec, SLO, builds critiques, flux live resume.
- Fichiers impactes:
	- [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
	- [UX.md](UX.md)

#### M-009 - Overview: dashboard plus graphique et sans trous de layout

- Date: 2026-04-03
- Statut: implementee
- Portee: page Overview
- Pourquoi: rendre l'ecran plus lisible en mode pilotage, avec une densite visuelle maitrisée et une disposition continue des widgets
- Changement:
  - Recomposition de la grille pour eliminer les trous visuels entre widgets.
  - Passage a une logique de dashboard mosaic (1/2, 1/3, 2/3) sur Overview.
  - Ajout de visuels synthétiques dans les widgets: tendances, stacks de severite, meters, ratios, SLO pills, runs critiques, flux live.
  - Conservation d'un niveau de lecture textuel secondaire sous les visuels.
- Fichiers impactes:
  - [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
  - [docs/ux-mockups/styles.css](docs/ux-mockups/styles.css)
  - [UX.md](UX.md)

#### M-010 - Bibliotheque cible pour les graphiques: Highcharts

- Date: 2026-04-03
- Statut: implementee
- Portee: implementation future des widgets graphiques
- Pourquoi: fixer une base technique unique pour les visualisations afin d'eviter des choix divergents lors du passage de la maquette a la vraie IHM
- Changement:
  - La future implementation des graphiques se fera avec Highcharts.
  - Les widgets de maquette doivent desormais etre pensés comme des equivalents conceptuels de composants Highcharts (bar, column, spline, stacked bar, timeline-like, gauge-like).
  - La maquette actuelle reste un prototype statique/JS simple et n'embarque pas encore Highcharts.
- Reference:
  - https://www.highcharts.com/
- Impact de conception:
  - Privilegier des formes de visualisation compatibles nativement avec Highcharts.
  - Eviter d'introduire dans la maquette des patterns graphiques difficiles a reproduire proprement avec Highcharts.
  - Prevoir un mapping explicite widget maquette -> type de chart Highcharts lors de l'implementation.
- Fichiers impactes:
  - [UX.md](UX.md)

#### M-012 - Workers: cockpit SRE mixte flotte/pools + drill-down agent d execution

- Date: 2026-04-03
- Statut: implementee
- Portee: page Workers
- Pourquoi: donner aux profils SRE / plateforme une vue unique qui combine pilotage capacitaire, signaux d'incident et diagnostic individuel sans devoir changer d'ecran
- Reponses utilisateur integrees:
  - Mission: mix capacite, operations et diagnostic
  - Audience primaire: SRE / plateforme
  - Nature de l'ecran: mix analytique + operations
  - Granularite: equilibre strict entre vue flotte/pool et vue agent d execution individuel
  - Preference: widgets graphiques quand c'est pertinent
- Changement:
  - Refonte de la page Workers en dashboard plus dense et plus graphique.
  - Ajout d'un bloc de synthese flotte et d'un bloc capacite / saturation par pool.
  - Ajout de widgets de surveillance rapide: agents d execution unhealthy / silencieux, repartition des builds actifs, timeline d'incidents.
  - Conservation d'un bloc d'actions operatoires claim / complete, mais reduit a un role de support.
  - Ajout d'un explorateur interactif de agents d execution avec selection dans la flotte et detail individuel (etat, heartbeat, capacite locale, evenements recents).
- Fichiers impactes:
  - [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
  - [docs/ux-mockups/styles.css](docs/ux-mockups/styles.css)
  - [UX.md](UX.md)

#### M-013 - Workers: passe de finition operationnelle (triage, actions guidees, taxonomie statuts)

- Date: 2026-04-03
- Statut: implementee
- Portee: page Workers
- Pourquoi: transformer l'ecran Workers en outil de decision immediate pour SRE/plateforme, sans perdre la profondeur de diagnostic individuel
- Changement:
  - Ajout d'un bandeau Triage en tete de page (Workers down, Silent > 5m, Queue at risk) pour priorisation instantanee.
  - Ajout d'un bloc Suite best actions avec actions recommandees (drain/reassign/acknowledge) pour guider la reponse operationnelle.
  - Evolution du bloc capacite en vue Capacity vs Demand par pool avec tendance 30 minutes et indicateurs d'attente.
  - Enrichissement du drill-down agent d execution en sections explicites: Runtime health, Build workload, Failure signals, Impacted runs.
  - Ajout de quick actions contextuelles sur le agent d execution (drain, cordon, restart check) avec indication de confirmation requise.
  - Harmonisation de la taxonomie des statuts agents d execution (healthy, degraded, unhealthy, silent) dans la vue.
- Alignement implementation:
  - Mapping conserve avec la cible Highcharts pour la vue capacitaire (column/bar + line trend), tout en gardant le drill-down en composant UI custom.
- Fichiers impactes:
  - [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
  - [docs/ux-mockups/styles.css](docs/ux-mockups/styles.css)
  - [UX.md](UX.md)

#### M-014 - Ecran 4 SCM Security: triage confiance + runbook de confinement

- Date: 2026-04-04
- Statut: implementee
- Portee: page SCM Security
- Pourquoi: faire de la page SCM Security une vraie frontiere de confiance operationnelle, orientee detection rapide, containment, puis preuve/audit
- Changement:
  - Ajout d'un bandeau de triage en tete (invalid signatures, IP rejects, secrets expiring).
  - Ajout d'un bloc d'actions rapides de confinement (rotate secret, quarantine IP, disable polling risque).
  - Refonte du bloc Webhook security en vue couverture des controles (signature, allowlist, replay protection).
  - Evolution de Polling control en Polling governance avec repartition enabled/paused/manual et garde-fous anti-duplication.
  - Refonte de Rejections diagnostics en flux live forensique (motifs, tendance, provider impacte).
  - Ajout d'un bloc Replay & evidences audit pour la traçabilite/compliance.
  - Ajout d'un bloc Configuration sensible avec guard rails explicites.
- Impact UX attendu:
  - Reduction du temps de reaction en incident SCM.
  - Meilleure lisibilite des priorites securite vs configuration courante.
  - Chaîne detection -> action -> evidence plus explicite.
- Fichiers impactes:
  - [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
  - [UX.md](UX.md)

#### M-015 - Ecran 5 Plugins & Policy: gouvernance operationnelle de l'extensibilite

- Date: 2026-04-04
- Statut: implementee
- Portee: page Plugins & Policy
- Pourquoi: rendre l'extensibilite pilotable en temps reel, avec une boucle claire triage -> containment -> gouvernance -> audit
- Changement:
  - Ajout d'un bandeau de triage (plugin failures, policy violations, drift).
  - Ajout d'un bloc Runbook actions pour containment rapide (disable plugin, fallback policy, dry-run replay).
  - Refonte du pilotage lifecycle plugin en vue sante runtime (load/init/execute/unload).
  - Ajout d'un bloc de couverture d'enforcement policy (global/env/capabilities).
  - Ajout d'un flux forensique violations & drift (deny/allow/drift events).
  - Ajout d'un bloc Capability governance avec guard rails explicites.
  - Enrichissement d'Inventory avec provenance et statut de signature.
- Impact UX attendu:
  - Reduction du temps de containment des incidents plugin/policy.
  - Meilleure lisibilite des risques de derive de politique.
  - Separation plus nette entre operations urgentes et edition gouvernance.
- Fichiers impactes:
  - [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
  - [UX.md](UX.md)

#### M-016 - Ecran 6 Observability: triage, correlation et preuve operationnelle

- Date: 2026-04-04
- Statut: implementee
- Portee: page Observability
- Pourquoi: transformer Observability en poste d'investigation priorisee, capable de relier rapidement les signaux techniques a l'impact delivery et d'alimenter les post-mortems
- Changement:
  - Ajout d'un bandeau de triage observabilite (critical alerts, event burst, signal lag).
  - Ajout d'un bloc d'actions guidees d'investigation (open traces, suppression, forensic snapshot).
  - Refonte du flux live events avec signaux plus actionnables et filtres explicites.
  - Ajout d'un bloc Signal quality pour mesurer la sante du systeme d'observabilite (coverage/freshness/noise).
  - Ajout d'un bloc incidents par severite pour priorisation P1/P2/P3.
  - Ajout d'un bloc Correlation map (build/agent d execution/plugin) pour reduire les allers-retours entre ecrans.
  - Ajout d'un bloc Exports & forensic snapshots pour la preuve/compliance.
  - Enrichissement du journal operations avec lien vers incidents.
- Impact UX attendu:
  - Diminution du temps de diagnostic et de l'effort de correlation.
  - Meilleure priorisation en situation d'incident.
  - Passage plus fluide vers la production d'evidences post-mortem.
- Fichiers impactes:
  - [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
  - [UX.md](UX.md)

#### M-017 - Ecran 7 Administration: gouvernance priorisee et audit actionnable

- Date: 2026-04-04
- Statut: implementee
- Portee: page Administration
- Pourquoi: faire de la page Administration un cockpit de gouvernance avec priorisation des risques, controle des operations sensibles et audit directement exploitable
- Changement:
  - Ajout d'un bandeau de triage administration (pending approvals, privilege drift, sensitive ops volume).
  - Ajout d'un runbook de gouvernance pour containment rapide (revoke grant, force re-auth, freeze window).
  - Refonte du role management en vue coverage RBAC et ecarts de least privilege.
  - Ajout d'un bloc Sensitive operations control avec statut approved/pending/rejected.
  - Ajout d'un bloc Admin access anomalies pour detection forensique en temps quasi reel.
  - Ajout d'un bloc Change approvals & maintenance windows pour orchestration des changements.
  - Enrichissement du journal Admin activity avec lien de preuve/evidence.
- Impact UX attendu:
  - Reduction du temps de reaction sur anomalies d'acces privilegie.
  - Meilleure gouvernance des operations critiques.
  - Audit et post-mortem facilites par une piste de preuve plus directe.
- Fichiers impactes:
  - [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
  - [UX.md](UX.md)

#### M-018 - Cohérence maquette avec fonctions API reellement disponibles

- Date: 2026-04-04
- Statut: implementee
- Portee: maquette transversale (tous ecrans)
- Pourquoi: eviter une divergence entre UX cible et capacites backend effectivement exposees aujourd'hui
- Contrainte API actuelle retenue:
  - `GET /health`
  - `POST /jobs`
  - `GET /jobs`
  - `POST /jobs/{id}/run`
  - `POST /builds/{id}/cancel`
  - `GET /builds`
- Changement:
  - Ajout d'un panneau "Perimetre API reel" en tete de chaque ecran avec niveau de couverture (`full`, `partial`, `roadmap`).
  - Alignement explicite des actions Delivery sur les endpoints existants (`GET /jobs`, `GET /builds`, `POST /builds/{id}/cancel`).
  - Desactivation des actions hors perimetre API avec marquage visuel `roadmap`.
  - Conservation de la vision UX cible sur les ecrans avances (Workers, SCM, Plugins, Observability, Administration), mais en mode non-actionnable tant que les endpoints associes n'existent pas.
- Impact UX attendu:
  - Reduction des ambiguïtés entre prototype et produit realisable a court terme.
  - Priorisation plus claire des evolutions backend necessaires.
- Fichiers impactes:
  - [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
  - [docs/ux-mockups/styles.css](docs/ux-mockups/styles.css)
  - [UX.md](UX.md)

### Mapping maquette -> Highcharts -> donnees backend

| Widget maquette | Ecran | Type Highcharts cible | Donnees backend necessaires |
|---|---|---|---|
| Mini tendances availability / latency / throughput | Overview | `column` ou `areaspline` | series temporelles par fenetre (ex: 24h, 7j), timestamp + valeur + delta de reference |
| Severite incidents empilee | Overview | `bar` empile ou `column` empile | nb d'incidents par severite, par statut, par fenetre temporelle |
| Capacite agents d execution / queue | Overview | `bar` horizontal, `bullet`, ou `xrange` simplifie | capacite totale, busy/idle/unhealthy, queue depth, wait time, trend |
| Ratio succes / echec | Overview | `pie`, `stacked bar`, ou `item chart` | total runs par statut sur fenetre donnee |
| SLO / disponibilite | Overview | `solidgauge`, `bullet`, ou `column` compare objectif/reel | objectifs SLO, valeur observee, budget erreur consomme, historique |
| Builds critiques | Overview | `xrange`, `bar`, ou `columnrange` selon finesse voulue | runs critiques, progression, statut, duree, phase courante, criticite |
| Flux live resume | Overview / Observability | `timeline` si licence/plugin adapte, sinon liste enrichie hors chart | evenements ordonnes, timestamp, severite, ressource, message |
| Graphe de phases pipeline | Pipelines | pas un fit naturel Highcharts standard; option `xrange` custom ou composant UI dedie | DAG de build: phases, jobs, dependances, statuts, durees |
| Progression des runs critiques | Overview | `bar` horizontal | id run, pourcentage progression, statut, ETA |
| Historique capacite / saturation | Workers / Overview | `areaspline` ou `line` | charge agents d execution, queue depth, taux d'occupation dans le temps |

### Notes de conception

- Tous les widgets ne doivent pas forcement devenir des charts Highcharts; certains resteront des composants UI si la lecture est meilleure hors chart.
- Le graphe de dependances pipeline n'est pas un cas ideal pour Highcharts standard. Il faudra soit:
  - conserver un composant UI custom pour le DAG,
  - soit utiliser Highcharts de maniere adaptee seulement pour certaines vues synthétiques (progression, duree, chronologie).
- Pour chaque widget retenu en implementation, definir:
  - granularite temporelle,
  - frequence de rafraichissement,
  - mode vide / loading / erreur,
  - seuils visuels et couleurs metier.

### Backlog technique - APIs et data contracts

#### Principes

- Un widget = un contrat de donnees explicite, versionnable si necessaire.
- Privilegier des endpoints d'agregation dashboard plutot qu'un grand nombre d'appels front trop fins.
- Separer les donnees temps reel des donnees de tendance/historique.

#### BT-001 - Contrat dashboard overview agrégé

- Priorite: P0
- Ecran: Overview
- Objectif: alimenter les widgets de premier niveau sans multiplier les appels frontend.
- Type de contrat: endpoint agregé
- Proposition:
  - `GET /dashboard/overview?window=24h&trend=7d`
- Donnees attendues:
  - availability
  - median_duration_seconds
  - throughput_total
  - incidents_open_by_severity
  - worker_capacity_summary
  - queue_summary
  - success_failure_summary
  - slo_summary
  - critical_builds_summary
  - live_events_summary
- Frequence de rafraichissement cible:
  - 15 a 30 secondes pour la partie operational summary

#### BT-002 - Serie temporelle KPI globaux

- Priorite: P0
- Ecran: Overview
- Widget: mini tendances availability / latency / throughput
- Type de contrat: series temporelles
- Proposition:
  - `GET /metrics/overview/timeseries?window=24h&bucket=5m`
- Donnees attendues:
  - `timestamps[]`
  - `availability_percent[]`
  - `median_duration_seconds[]`
  - `throughput_count[]`
  - `comparison_window`
  - `delta_vs_reference`
- Usage Highcharts:
  - `column` ou `areaspline`

#### BT-003 - Incidents par severite et statut

- Priorite: P0
- Ecran: Overview
- Widget: severite incidents empilee
- Type de contrat: agregat categoriel
- Proposition:
  - `GET /incidents/summary?window=24h`
- Donnees attendues:
  - severite (`p1`, `p2`, `p3`)
  - count_open
  - count_acknowledged
  - count_resolved
  - top_incidents[]
- Usage Highcharts:
  - `bar` empile ou `column` empile

#### BT-004 - Capacite agents d execution et pression de queue

- Priorite: P0
- Ecran: Overview, Workers
- Widget: capacite agents d execution / queue
- Type de contrat: agregat runtime
- Proposition:
  - `GET /capacity/summary`
  - `GET /capacity/timeseries?window=6h&bucket=5m`
- Donnees attendues:
  - total_workers
  - busy_workers
  - idle_workers
  - unhealthy_workers
  - queue_depth
  - oldest_queue_age_seconds
  - occupancy_percent_history[]
  - queue_depth_history[]
- Usage Highcharts:
  - `bar`, `bullet`, `areaspline`

#### BT-005 - Repartition des statuts de runs

- Priorite: P0
- Ecran: Overview
- Widget: taux succes / echec
- Type de contrat: distribution de statuts
- Proposition:
  - `GET /builds/status-summary?window=24h`
- Donnees attendues:
  - success_count
  - failed_count
  - canceled_count
  - blocked_count
  - total_count
- Usage Highcharts:
  - `stacked bar` prefere a `pie` pour lecture dashboard dense

#### BT-006 - Contrat SLO / budget erreur

- Priorite: P1
- Ecran: Overview
- Widget: SLO / disponibilite
- Type de contrat: reliability summary
- Proposition:
  - `GET /slo/summary?window=30d`
- Donnees attendues:
  - objective_name
  - target_percent

---

## UX-009 - Decoupage de App.tsx en widgets React dedies

- Date: 2026-04-15
- Statut: implementee
- Responsable: Engineering
- Type: frontend architecture

### Contexte

Le dashboard etait majoritairement rendu depuis un seul fichier `dashboard/src/App.tsx`, ce qui rendait les evolutions de layout et les revues plus couteuses.

### Decision

Extraire les zones UI stables en widgets dedies, tout en conservant l'orchestration d'etat/handlers dans `App.tsx`:

1. Header global (`DashboardHeader`).
2. Navigation laterale (`SideNav`).
3. Panneau perimetre API (`ApiCoveragePanel`).
4. Pages implementees Pipelines/Overview (`ImplementedPagesWidget`).
5. Pages roadmap/lecture seule (`RoadmapPagesWidget`).
6. Journal operateur (`ConsoleWidget`).

### Impact attendu

- Meilleure maintenabilite front (responsabilites visuelles explicites).
- Refactor plus sur pour les ecrans individuels.
- Base plus propre pour poursuivre la migration des pages roadmap vers des composants API-backes.

### Evidence (code)

- Composition principale: [dashboard/src/App.tsx](dashboard/src/App.tsx)
- Widgets: [dashboard/src/widgets/DashboardHeader.tsx](dashboard/src/widgets/DashboardHeader.tsx)
- Widgets: [dashboard/src/widgets/SideNav.tsx](dashboard/src/widgets/SideNav.tsx)
- Widgets: [dashboard/src/widgets/ApiCoveragePanel.tsx](dashboard/src/widgets/ApiCoveragePanel.tsx)
- Widgets: [dashboard/src/widgets/ImplementedPagesWidget.tsx](dashboard/src/widgets/ImplementedPagesWidget.tsx)
- Widgets: [dashboard/src/widgets/RoadmapPagesWidget.tsx](dashboard/src/widgets/RoadmapPagesWidget.tsx)
- Widgets: [dashboard/src/widgets/ConsoleWidget.tsx](dashboard/src/widgets/ConsoleWidget.tsx)

---

## UX-010 - Separation logique metier vs composition UI dans App

- Date: 2026-04-15
- Statut: implementee
- Responsable: Engineering
- Type: frontend architecture

### Contexte

Apres le decoupage en widgets, `App.tsx` contenait encore une grande partie de la logique metier (state, effets, appels API, derivees). Cette concentration ralentissait les evolutions de comportement et la testabilite de la couche UI.

### Decision

Introduire un hook de controleur dedie (`useDashboardController`) pour centraliser:

1. Etats reactifs dashboard.
2. Handlers metier (jobs/builds, polling, plugins, agents d execution, observability).
3. Effets de lifecycle (health polling, SSE, refresh schedule).
4. View-models derives consommes par les widgets.

`App.tsx` devient une couche de composition qui assemble les widgets et relie leurs props au controleur.

### Impact attendu

- Lisibilite accrue de `App.tsx` (focus layout/composition).
- Maintenance plus sure de la logique metier sans toucher au markup.
- Base plus nette pour tests unitaires de logique et tests d'integration UI.

### Evidence (code)

- Composition UI: [dashboard/src/App.tsx](dashboard/src/App.tsx)
- Controleur metier: [dashboard/src/hooks/useDashboardController.ts](dashboard/src/hooks/useDashboardController.ts)

---

## UX-011 - Un composant TSX par page du sidenav

- Date: 2026-04-15
- Statut: implementee
- Responsable: Engineering
- Type: frontend architecture

### Contexte

Apres le passage a un `App.tsx` plus fin, le rendu des pages restait mutualise dans des widgets aggreges. Cela masquait encore la frontiere entre les 7 ecrans de navigation metier.

### Decision

Creer un composant TSX dedie pour chaque page du sidenav:

1. `PipelinesPage`
2. `OverviewPage`
3. `WorkersPage`
4. `ScmSecurityPage`
5. `PluginsPolicyPage`
6. `ObservabilityPage`
7. `AdministrationPage`

`App.tsx` selectionne desormais explicitement la page active et lui transmet uniquement les props necessaires.

### Impact attendu

- Frontiere claire entre les ecrans metier.
- Evolutions par page plus simples et moins risquees.
- Base plus propre pour ajouter tests et contrats API page par page.

### Evidence (code)

- Composition/switch de page: [dashboard/src/App.tsx](dashboard/src/App.tsx)
- Pages: [dashboard/src/pages/PipelinesPage.tsx](dashboard/src/pages/PipelinesPage.tsx)
- Pages: [dashboard/src/pages/OverviewPage.tsx](dashboard/src/pages/OverviewPage.tsx)
- Pages: [dashboard/src/pages/WorkersPage.tsx](dashboard/src/pages/WorkersPage.tsx)
- Pages: [dashboard/src/pages/ScmSecurityPage.tsx](dashboard/src/pages/ScmSecurityPage.tsx)
- Pages: [dashboard/src/pages/PluginsPolicyPage.tsx](dashboard/src/pages/PluginsPolicyPage.tsx)
- Pages: [dashboard/src/pages/ObservabilityPage.tsx](dashboard/src/pages/ObservabilityPage.tsx)
- Pages: [dashboard/src/pages/AdministrationPage.tsx](dashboard/src/pages/AdministrationPage.tsx)

---

## UX-012 - Nettoyage post-decoupage des pages

- Date: 2026-04-15
- Statut: implementee
- Responsable: Engineering
- Type: frontend architecture

### Contexte

Une fois les 7 pages extraites, les widgets agreges historiques etaient devenus redondants et les interfaces de props restaient dispersees dans plusieurs fichiers de page.

### Decision

1. Supprimer les anciens widgets agreges devenus inutiles.
2. Centraliser les interfaces de props de page dans `dashboard/src/pages/types.ts`.

### Impact attendu

- Arborescence plus nette apres refactor.
- Moins de duplication des types de props.
- Maintenance plus simple lors des evolutions page par page.

### Evidence (code)

- Types de page: [dashboard/src/pages/types.ts](dashboard/src/pages/types.ts)
- Pages actives: [dashboard/src/pages/PipelinesPage.tsx](dashboard/src/pages/PipelinesPage.tsx)
- Pages actives: [dashboard/src/pages/OverviewPage.tsx](dashboard/src/pages/OverviewPage.tsx)
- Pages actives: [dashboard/src/pages/WorkersPage.tsx](dashboard/src/pages/WorkersPage.tsx)
- Pages actives: [dashboard/src/pages/ScmSecurityPage.tsx](dashboard/src/pages/ScmSecurityPage.tsx)
  - observed_percent
  - error_budget_total
  - error_budget_consumed
  - trend[]
- Usage Highcharts:
  - `bullet`, `solidgauge`, `column`

#### BT-007 - Builds critiques / prioritaires

- Priorite: P0
- Ecran: Overview, Pipelines
- Widget: builds critiques
- Type de contrat: liste priorisee runtime
- Proposition:
  - `GET /builds/critical?limit=10`
- Donnees attendues:
  - build_id
  - pipeline_name
  - status
  - progress_percent
  - started_at
  - eta_seconds
  - current_phase
  - criticality_reason
- Usage Highcharts:
  - `bar` horizontal pour progression synthétique

#### BT-008 - Resume live evenements

---

## UX-013 - Decoupage interne du controleur dashboard en sous-modules

- Date: 2026-04-15
- Statut: implementee
- Responsable: Engineering
- Type: frontend architecture

### Contexte

Apres extraction de `useDashboardController`, le hook restait encore trop dense: types inline, constantes GraphQL, utilitaires, derivees memoisees et effets runtime partageaient le meme fichier.

### Decision

Scinder le controleur en modules de support explicites tout en conservant l'API publique du hook:

1. `dashboardTypes.ts` pour les types et contrats locaux du dashboard.
2. `dashboardConstants.ts` pour les constantes d'interface et les documents GraphQL.
3. `dashboardUtils.ts` pour les helpers purs (formatage, filtres, exports, stardate).
4. `useDashboardDerivedState.ts` pour les view-models memoises.
5. `useDashboardRuntimeEffects.ts` pour les effets lifecycle/runtime (health, SSE, polling, timers).

`useDashboardController.ts` garde desormais principalement l'orchestration d'etat et les handlers metier.

### Impact attendu

- Meilleure lisibilite du controleur principal.
- Frontieres plus nettes entre logique pure, logique derivee et effets runtime.
- Refactor futur plus sur pour tester ou faire evoluer les endpoints page par page.

### Evidence (code)

- Controleur orchestreur: [dashboard/src/hooks/useDashboardController.ts](dashboard/src/hooks/useDashboardController.ts)
- Types dashboard: [dashboard/src/hooks/dashboardTypes.ts](dashboard/src/hooks/dashboardTypes.ts)
- Constantes dashboard: [dashboard/src/hooks/dashboardConstants.ts](dashboard/src/hooks/dashboardConstants.ts)
- Utilitaires dashboard: [dashboard/src/hooks/dashboardUtils.ts](dashboard/src/hooks/dashboardUtils.ts)
- Etats derives: [dashboard/src/hooks/useDashboardDerivedState.ts](dashboard/src/hooks/useDashboardDerivedState.ts)
- Effets runtime: [dashboard/src/hooks/useDashboardRuntimeEffects.ts](dashboard/src/hooks/useDashboardRuntimeEffects.ts)

---

## UX-014 - Extraction des actions roadmap dans un sous-hook dedie

- Date: 2026-04-15
- Statut: implementee
- Responsable: Engineering
- Type: frontend architecture

### Contexte

Apres `UX-013`, le controleur principal restait encore charge par une longue serie de callbacks roadmap (SCM security/polling, plugins/policy, agents d execution control) qui n'alimentent pas encore directement les pages API fully-wired.

### Decision

Extraire ces callbacks dans un hook dedie `useDashboardRoadmapActions` et garder dans `useDashboardController`:

1. Les etats partages.
2. Les actions coeur (`refreshAll`, `createJob`, `runJob`, `cancelBuild`).
3. L'assemblage final des derives/effects + retour public du hook.

Le nouveau sous-hook encapsule la logique actionnelle roadmap tout en conservant les memes contrats et messages operateur.

### Impact attendu

- Reduction du volume de `useDashboardController.ts`.
- Separation plus claire entre noyau API actuel et fonctions roadmap progressives.
- Evolutions futures des ecrans roadmap plus localisees et moins risquées.

### Evidence (code)

- Controleur principal: [dashboard/src/hooks/useDashboardController.ts](dashboard/src/hooks/useDashboardController.ts)
- Sous-hook actions roadmap: [dashboard/src/hooks/useDashboardRoadmapActions.ts](dashboard/src/hooks/useDashboardRoadmapActions.ts)

---

## UX-015 - Decoupage des actions roadmap par domaine fonctionnel

- Date: 2026-04-15
- Statut: implementee
- Responsable: Engineering
- Type: frontend architecture

### Contexte

Le hook `useDashboardRoadmapActions` restait volumineux car il melangeait 3 domaines distincts (SCM, Plugins/Policy, Workers), ce qui limitait la lisibilite et la capacite a faire evoluer un domaine sans toucher aux autres.

### Decision

Decouper les callbacks roadmap en 3 hooks de domaine:

1. `useDashboardScmActions` pour webhook-security, polling et diagnostics SCM.
2. `useDashboardPluginActions` pour lifecycle plugin et policy authorization.
3. `useDashboardWorkerActions` pour claim/complete/refresh agent d execution control.

### Impact attendu

- Frontiere claire par domaine fonctionnel roadmap.
- Refactors plus localises (moins de risque de regression croisee).
- Base plus propre pour introduire tests unitaires par domaine.

### Evidence (code)

- Domaine SCM: [dashboard/src/hooks/useDashboardScmActions.ts](dashboard/src/hooks/useDashboardScmActions.ts)
- Domaine Plugins: [dashboard/src/hooks/useDashboardPluginActions.ts](dashboard/src/hooks/useDashboardPluginActions.ts)
- Domaine Workers: [dashboard/src/hooks/useDashboardWorkerActions.ts](dashboard/src/hooks/useDashboardWorkerActions.ts)

---

## UX-016 - Domaines autonomes: etat + actions hors du controleur

- Date: 2026-04-15
- Statut: implementee
- Responsable: Engineering
- Type: frontend architecture

### Contexte

Apres le decoupage des callbacks par domaine, `useDashboardController` conservait encore l'etat roadmap (SCM, Plugins, Workers), ce qui limitait l'autonomie effective de chaque domaine.

### Decision

Introduire des hooks de domaine qui possedent leur propre etat et composent leurs actions:

1. `useDashboardScmDomain` (forms/messages/state + actions SCM).
2. `useDashboardPluginDomain` (forms/messages/inventory/policy state + actions plugin).
3. `useDashboardWorkerDomain` (agent d execution control state + actions agent d execution).

`useDashboardController` orchestre uniquement les domaines, les derivees globales et les actions coeur API-backed.

### Impact attendu

- Composants/hooks plus autonomes par domaine metier.
- Couplage reduit entre domains roadmap et controleur global.
- Base plus nette pour brancher des pages domaine et des tests par domaine.

### Evidence (code)

- Controleur orchestration: [dashboard/src/hooks/useDashboardController.ts](dashboard/src/hooks/useDashboardController.ts)
- Domaine SCM autonome: [dashboard/src/hooks/useDashboardScmDomain.ts](dashboard/src/hooks/useDashboardScmDomain.ts)
- Domaine Plugins autonome: [dashboard/src/hooks/useDashboardPluginDomain.ts](dashboard/src/hooks/useDashboardPluginDomain.ts)
- Domaine Workers autonome: [dashboard/src/hooks/useDashboardWorkerDomain.ts](dashboard/src/hooks/useDashboardWorkerDomain.ts)

---

## UX-017 - Pages roadmap branchees sur les domaines autonomes

- Date: 2026-04-15
- Statut: implementee
- Responsable: Engineering
- Type: frontend architecture

### Contexte

Les hooks de domaine etaient autonomes, mais plusieurs pages roadmap restaient principalement statiques et ne consommaient pas encore les signaux/actions de domaine.

### Decision

Brancher les pages roadmap sur les objets domaine exposes par le controleur:

1. `WorkersPage` consomme `workerDomain` (message, dernier claim, refresh agents d execution).
2. `ScmSecurityPage` consomme `scmDomain` (messages operations, tick summary, refresh diagnostics, tick manuel).
3. `PluginsPolicyPage` consomme `pluginDomain` (inventory, messages plugin/policy, refresh inventory).
4. `AdministrationPage` consomme des signaux gouvernance (`adminActivity`, `roleCapabilities`).

### Impact attendu

- Composants UI davantage alignes sur les domaines metier.
- Moins de placeholders figes sur les pages roadmap.
- Transition facilitee vers des parcours operationnels complets page par page.

### Evidence (code)

- Wiring principal: [dashboard/src/App.tsx](dashboard/src/App.tsx)
- Workers page: [dashboard/src/pages/WorkersPage.tsx](dashboard/src/pages/WorkersPage.tsx)
- SCM Security page: [dashboard/src/pages/ScmSecurityPage.tsx](dashboard/src/pages/ScmSecurityPage.tsx)
- Plugins page: [dashboard/src/pages/PluginsPolicyPage.tsx](dashboard/src/pages/PluginsPolicyPage.tsx)
- Administration page: [dashboard/src/pages/AdministrationPage.tsx](dashboard/src/pages/AdministrationPage.tsx)

---

## UX-018 - Pages proprietaires des hooks domaine + arborescence domaine

- Date: 2026-04-15
- Statut: implementee
- Responsable: Engineering
- Type: frontend architecture

### Contexte

Le controleur exposait encore des objets domaine aux pages et conservait une logique de reference legacy (`keepRoadmapReferences`). De plus, les hooks n'etaient pas ranges par domaine physique avec un nommage de fichier neutre.

### Decision

1. Les pages roadmap instancient directement leurs hooks domaine (`Workers`, `SCM Security`, `Plugins & Policy`) a partir de dependances coeur.
2. Suppression de `keepRoadmapReferences` dans le controleur.
3. Reorganisation des hooks par dossiers de domaine, avec noms de fichiers sans prefixe `useDashboard`:
  - `hooks/core/controller.ts`, `hooks/core/derivedState.ts`, `hooks/core/runtimeEffects.ts`
  - `hooks/scm/actions.ts`, `hooks/scm/domain.ts`
  - `hooks/plugins/actions.ts`, `hooks/plugins/domain.ts`
  - `hooks/agents d execution/actions.ts`, `hooks/agents d execution/domain.ts`

### Impact attendu

- Autonomie plus forte page <-> domaine.
- Controleur recentre sur l'orchestration coeur.
- Arborescence plus lisible et orientee metier.

### Evidence (code)

- Controleur coeur: [dashboard/src/hooks/core/controller.ts](dashboard/src/hooks/core/controller.ts)
- Domaine Workers: [dashboard/src/hooks/agents d execution/domain.ts](dashboard/src/hooks/agents d execution/domain.ts)
- Domaine SCM: [dashboard/src/hooks/scm/domain.ts](dashboard/src/hooks/scm/domain.ts)
- Domaine Plugins: [dashboard/src/hooks/plugins/domain.ts](dashboard/src/hooks/plugins/domain.ts)
- Pages proprietaires: [dashboard/src/pages/WorkersPage.tsx](dashboard/src/pages/WorkersPage.tsx)
- Pages proprietaires: [dashboard/src/pages/ScmSecurityPage.tsx](dashboard/src/pages/ScmSecurityPage.tsx)
- Pages proprietaires: [dashboard/src/pages/PluginsPolicyPage.tsx](dashboard/src/pages/PluginsPolicyPage.tsx)

- Priorite: P1
- Ecran: Overview, Observability
- Widget: flux live resume
- Type de contrat: stream + snapshot recent
- Proposition:
  - `GET /events/summary?window=15m&limit=20`
  - ou SSE/WebSocket + snapshot initial
- Donnees attendues:
  - event_id
  - timestamp
  - severity
  - kind
  - resource_type
  - resource_id
  - message
- Usage Highcharts:
  - probablement hors chart en liste/timeline UI

#### BT-009 - Contrat DAG pipeline detaille

- Priorite: P0
- Ecran: Pipelines
- Widget: graphe de phases pipeline
- Type de contrat: graphe d'execution detaille
- Proposition:
  - `GET /builds/{id}/graph`
  - `GET /builds/{id}/logs?step_id=...`
- Donnees attendues:
  - nodes[]: id, label, phase, status, started_at, finished_at, duration_seconds
  - edges[]: from, to, type
  - groups/phases[]
  - current_node_id
  - log streams par step
- Usage UI:
  - composant custom recommande

#### BT-010 - Data contract filtrage logs par etape

- Priorite: P0
- Ecran: Pipelines
- Widget: log detaille filtre
- Type de contrat: logs segmentes
- Proposition:
  - `GET /builds/{id}/steps/{stepId}/logs?cursor=...`
- Donnees attendues:
  - step_id
  - command
  - stream (`stdout`/`stderr`)
  - timestamp
  - line
  - cursor_next
- Contraintes:
  - pagination / streaming obligatoire si logs volumineux

#### BT-011 - Meta contrat frontend pour widgets

- Priorite: P1
- Portee: tous ecrans dashboard
- Objectif: normaliser loading, empty, stale, error
- Donnees meta recommandees sur chaque endpoint:
  - `generated_at`
  - `window`
  - `partial`
  - `stale`
  - `errors[]`

#### BT-012 - Ordre d'implementation recommande

- Priorite sequencee:
  1. Pipelines build graph + logs segmentes
  2. Overview summary agregé
  3. Capacity + status summary
  4. Critical builds
  5. SLO summary
  6. Live event summary

#### M-011 - Conversion de la matrice UX en backlog technique API/data contracts

- Date: 2026-04-03
- Statut: implementee
- Portee: preparation implementation reelle
- Pourquoi: rendre la phase suivante directement actionnable pour le backend et le frontend
- Changement:
  - Ajout d'un backlog technique structure par widget.
  - Definition des contrats de donnees cibles, endpoints proposes, priorites et frequence de rafraichissement.
  - Clarification des zones UI custom vs Highcharts.
- Fichiers impactes:
	- [UX.md](UX.md)

#### M-007 - Validation de l'ecran 1 Pipelines et passage a l'ecran 2 Overview

- Date: 2026-04-03
- Statut: implementee
- Portee: atelier maquette
- Pourquoi: figer la validation de l'ecran 1 avant d'ouvrir une nouvelle iteration sur l'ecran 2
- Changement:
  - Ecran 1 Pipelines considere comme valide pour cette iteration.
  - L'atelier se deplace sur l'ecran 2 Overview.
- Fichiers impactes:
  - [UX.md](UX.md)

### Suite

- Revue visuelle de l'ecran 2 Overview et ouverture de l'iteration suivante si besoin.

---

## UX-019 - Normalisation finale du nommage des hooks domaine/core

- Date: 2026-04-15
- Statut: implementee
- Responsable: Engineering
- Type: frontend architecture

### Contexte

Apres la reorganisation des hooks par dossiers de domaine (`core`, `scm`, `plugins`, `agents d execution`), il restait un legacy de nommage `useDashboard*` sur plusieurs symboles exportes/importes.

### Decision

Finaliser le nommage pour aligner les symboles avec l'architecture cible, sans modifier les comportements:

1. `useDashboardController` -> `useController`
2. `useDashboardDerivedState` -> `useDerivedState`
3. `useDashboardRuntimeEffects` -> `useRuntimeEffects`
4. `useDashboardScmDomain`/`Actions` -> `useScmDomain`/`useScmActions`
5. `useDashboardPluginDomain`/`Actions` -> `usePluginDomain`/`usePluginActions`
6. `useDashboardWorkerDomain`/`Actions` -> `useWorkerDomain`/`useWorkerActions`

### Impact attendu

- Nommage coherent avec l'arborescence orientee domaine.
- Lecture plus rapide des dependances dans les pages et le controleur coeur.
- Reduction de la dette de migration issue des etapes precedentes.

### Evidence (code)

- Controleur coeur: [dashboard/src/hooks/core/controller.ts](dashboard/src/hooks/core/controller.ts)
- Etats derives: [dashboard/src/hooks/core/derivedState.ts](dashboard/src/hooks/core/derivedState.ts)
- Effets runtime: [dashboard/src/hooks/core/runtimeEffects.ts](dashboard/src/hooks/core/runtimeEffects.ts)
- Domaine SCM: [dashboard/src/hooks/scm/domain.ts](dashboard/src/hooks/scm/domain.ts)
- Domaine Plugins: [dashboard/src/hooks/plugins/domain.ts](dashboard/src/hooks/plugins/domain.ts)
- Domaine Workers: [dashboard/src/hooks/agents d execution/domain.ts](dashboard/src/hooks/agents d execution/domain.ts)
- Composition UI: [dashboard/src/App.tsx](dashboard/src/App.tsx)

---

## UX-020 - Transition vers la premiere verticale Workers API-backed

- Date: 2026-04-15
- Statut: acceptee
- Responsable: Engineering
- Type: frontend architecture

### Contexte

Le cleanup de nommage et l'architecture orientee domaines sont stabilises. La prochaine valeur produit attendue est de reduire le mode roadmap sur une page metier complete.

### Decision

Prioriser la page Workers comme premiere verticale API-backed post-refactor:

1. Utiliser les endpoints agents d execution existants pour les actions claim/complete/list.
2. Exposer les retours de succes/erreur/conflit de maniere explicite dans la page.
3. Garder les conventions de role/audit/log deja en place dans l'architecture domaine.
4. Faire evoluer la couverture de page Workers de `roadmap` vers `partial` une fois les interactions critiques stabilisees.

### Impact attendu

- Passage d'une page majoritairement statique a une page operable sur flux agent d execution.
- Validation de l'architecture pages proprietaires des hooks domaine sur un cas concret.
- Reduction du risque de regressions sur les prochaines verticales (SCM/Plugins).

### Evidence (code)

- Page Workers: [dashboard/src/pages/WorkersPage.tsx](dashboard/src/pages/WorkersPage.tsx)
- Domaine Workers: [dashboard/src/hooks/agents d execution/domain.ts](dashboard/src/hooks/agents d execution/domain.ts)
- Actions Workers: [dashboard/src/hooks/agents d execution/actions.ts](dashboard/src/hooks/agents d execution/actions.ts)
- Orchestration coeur: [dashboard/src/hooks/core/controller.ts](dashboard/src/hooks/core/controller.ts)

---

## UX-021 - Synchronisation backend scheduler multi-backend

- Date: 2026-04-15
- Statut: implementee
- Responsable: Engineering
- Type: backend capability sync

### Contexte

Le runtime supporte maintenant une selection explicite du backend scheduler, avec ajout d'un backend PostgreSQL en plus des backends in-memory, file et Redis.

### Decision

Synchroniser le journal UX avec la capacite backend pour eviter les divergences de communication produit/plateforme:

1. Documenter les backends scheduler disponibles (`in-memory`, `file`, `redis`, `postgres`).
2. Confirmer que le mode runtime peut etre surcharge via variable d'environnement dediee.
3. Aligner la documentation d'exploitation sur le nouveau mode de selection explicite.

### Impact attendu

- Meilleure lisibilite cross-equipe sur les capacites backend actuelles.
- Reduction du risque de malentendu pendant les ateliers UX/ops.

### Evidence (code)

- Selection backend scheduler: [crates/server/src/main.rs](crates/server/src/main.rs)
- Backend PostgreSQL scheduler: [crates/scheduler/src/backend/postgres_scheduler.rs](crates/scheduler/src/backend/postgres_scheduler.rs)
- Guide migration scheduler: [docs/scheduler-migration.md](docs/scheduler-migration.md)
