## Tardigrade CI - Copilot Instructions

### Contexte du projet
- Tardigrade CI est un workspace Rust pour une plateforme CI/CD open source.
- Principales crates et roles :
	- `crates/server` : point d entree du serveur Axum, console web statique.
	- `crates/api` : routes HTTP et etat de l API.
	- `crates/core` : entites du domaine (`JobDefinition`, `BuildRecord`, `JobStatus`).
	- `crates/storage` : trait de stockage avec implementation en memoire.
	- `crates/scheduler` : trait d ordonnancement avec file en memoire.
	- `crates/plugins` : contrat et registre des plugins.
	- `crates/auth` : primitives d authentification.
	- `crates/worker` : agent d execution externe (claim/complete) pour traiter les builds.

### Surface API actuelle
- `POST /graphql`
- `GET /graphql`
- Le point d entree natif des webhooks SCM reste expose par `crates/server` sur `/webhooks/scm`.

### Construire, tester et executer
- Utiliser systematiquement des commandes compatibles avec le proxy dans ce depot :
	- `env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo test --workspace`
	- `env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server`
- Respecter les surcharges locales du registre Cargo definies dans `.cargo/config.toml` (le workspace utilise `cargo-public`).

### Exigences de codage
- Garder les changements minimaux, cibles et alignes avec l architecture modulaire actuelle.
- Preserver les API publiques existantes sauf si une demande de changement impose autre chose.
- Ajouter ou mettre a jour les tests lorsque le comportement change.
- Appliquer une passe anti code mort sur chaque evolution significative (au minimum `cargo clippy --workspace --all-targets -- -W dead_code`) et supprimer les composants orphelins detectes.
- Configuration runtime: utiliser exclusivement les fichiers TOML (pas de variables d environnement applicatives pour server/worker/API).
- Garder les tests hors des fichiers source de production : ne pas placer de blocs inline `mod tests { ... }` dans les fichiers d implementation principaux.
- Preferer des fichiers de test dedies (par exemple `src/tests.rs`) ou des tests d integration au niveau de la crate dans `tests/`.
- Maintenir la documentation et les exemples synchronises avec les changements d implementation.
- Le code doit etre correctement commente :
	- Ajouter des commentaires clairs pour la logique non evidente, les decisions, les cas limites et les invariants.
	- Privilegier des commentaires centres sur l intention plutot qu une narration ligne par ligne.
	- Eviter les commentaires redondants qui repetent un code deja explicite.
- Chaque fonction, structure et test doit etre commente.
- Toujours valider avec un make ci

### Lignes directrices de collaboration
- Traiter les taches de maniere systematique et rendre compte des progres de facon concise.
- Suivre les bonnes pratiques Rust et Axum pour la gestion des erreurs, le code asynchrone et la surete de typage.
- Maintenir ce fichier d instructions a jour au fil du temps avec les orientations majeures du projet et toute regle de developpement adoptee formellement par l equipe.
- Tracer toutes les actions dans BACKLOG.md et UX.md.

### Regles de dependance hexagonale (phase pragmatique)
- Respecter le flux de dependance entrant -> application -> domaine.
- Les adaptateurs (`crates/server`, `crates/api/graphql`, `crates/api/handlers`, `crates/api/state`) doivent appeler la facade use-case (`crates/api/application`) et eviter de porter l orchestration metier.
- La couche application/service doit consommer les ports (`Storage`, `Scheduler`) via trait objects, jamais des backends concrets.
- Le domaine (`crates/core`) ne doit jamais dependre de crates d adaptateurs ou d infrastructure.
- Tout couplage residuel temporaire accepte pour la phase pragmatique doit etre isole, documente dans `BACKLOG.md`, et planifie pour suppression en phase stricte.
- Un garde-fou de dependances est execute via `make arch-guard` (integre dans `make lint`/`make ci`) et doit rester vert.
