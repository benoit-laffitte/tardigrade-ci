# UX Decision Log

Ce document trace toutes les decisions UX importantes du projet.
Chaque decision doit etre ajoutee ici avant implementation (ou juste apres en cas d'urgence), avec son contexte et son impact attendu.

## Regles de tracabilite

- Ajouter une entree par decision importante (pas de regroupement flou).
- Renseigner date, statut, owner, impact utilisateur, risques.
- Mettre a jour le statut plutot que supprimer une entree.
- Lier les sections de code impactees si elles existent deja.

## Statuts

- proposed: idee formulee, pas encore validee.
- accepted: decision validee et a executer.
- implemented: decision livree dans l'interface.
- deprecated: decision abandonnee ou remplacee.

---

## UX-001 - Analyse de la situation actuelle (baseline)

- Date: 2026-04-03
- Statut: accepted
- Owner: Copilot + equipe produit
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
- Concentration des panels metier (jobs/builds/workers/ops/plugins/observabilite): [dashboard/src/App.tsx](dashboard/src/App.tsx#L1693)
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

### Next

- Cadrer les hypotheses de refonte sous forme d'options comparees (Mission Control vs Workflow guide vs UI par role).
- Definir les criteres de choix (time-to-action, taux d'erreur, effort technique, maintenabilite).

---

## UX-002 - Navigation multi-pages par vision metier CI

- Date: 2026-04-03
- Statut: accepted
- Owner: Product + Design + Engineering
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
- Capacite workers, claim/complete, saturation, troubleshooting execution.

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

### Next

- Produire une matrice Action -> Page (source de verite IA).
- Definir le menu de navigation (ordre, labels, badges d'alerte) avec Delivery en premier.
- Lancer un premier decoupage technique du monolithe [dashboard/src/App.tsx](dashboard/src/App.tsx).

---

## UX-003 - Page Administration dediee

- Date: 2026-04-03
- Statut: accepted
- Owner: Product + Design + Engineering
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

### Next

- Mettre a jour la matrice Action -> Page avec un bloc Administration explicite.
- Definir les garde-fous UX admin (confirmations, niveau de criticite, traces).

---

## UX-004 - Structure IHM cible + maquettes navigables

- Date: 2026-04-03
- Statut: accepted
- Owner: Product + Design + Engineering
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
- Statut: implemented
- Owner: Engineering
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
- Statut: implemented
- Owner: Engineering
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
- Statut: implemented
- Owner: Engineering
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
- Statut: implemented
- Owner: Engineering
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

### Next

- Faire une revue metier de la maquette (par role: viewer, operator, admin).
- Finaliser la matrice Action -> Page et les composants partages.
- Planifier le decoupage technique en routes/pages React.

---

## UX-005 - Atelier maquette et tracabilite des changements

- Date: 2026-04-03
- Statut: accepted
- Owner: Product + Design + Engineering
- Type: process

### Contexte

Demande utilisateur explicite: travailler iterativement sur la maquette en tracant chaque changement dans ce document.

### Decision

Tous les changements de maquette seront traces dans un journal d'iterations unique ci-dessous.

Regles d'entree pour chaque changement:

- ID: M-XXX (incremental)
- Statut: proposed | accepted | implemented | deprecated
- Portee: quelle page ou composant est impacte
- Pourquoi: probleme utilisateur vise
- Changement: description concrete avant/apres
- Fichiers impactes: liens vers la maquette

### Journal des iterations maquette

#### M-001 - Lancement de l'atelier maquette trace

- Date: 2026-04-03
- Statut: implemented
- Portee: gouvernance des iterations
- Pourquoi: garantir un historique clair des decisions et eviter les retours arriere implicites
- Changement: creation d'un cadre formel de suivi des modifications maquette
- Fichiers impactes:
	- [UX.md](UX.md)

#### M-002 - Pipelines: liste builds recents/en cours + detail build interactif

- Date: 2026-04-03
- Statut: implemented
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
- Statut: implemented
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
- Statut: implemented
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
- Statut: implemented
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
- Statut: implemented
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
- Statut: implemented
- Portee: page Overview
- Pourquoi: aligner l'ecran 2 sur une vision admin de sante globale CI, majoritairement en lecture, avec mix temps reel + tendance
- Reponses utilisateur integrees:
  - Mission prioritaire: vision sante globale CI
  - Audience primaire: admin
  - Nature de l'ecran: lecture uniquement
  - Horizon dominant: mix temps reel + tendance
  - Blocs prioritaires: KPI globaux, incidents, capacite workers/queue, succes/echec, builds critiques, SLO, flux live resume
- Changement:
  - Refonte de la page Overview en dashboard de pilotage admin.
  - Suppression de la logique d'action principale sur cette page.
  - Ajout des blocs: sante globale, incidents, capacite, succes/echec, SLO, builds critiques, flux live resume.
- Fichiers impactes:
	- [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
	- [UX.md](UX.md)

#### M-009 - Overview: dashboard plus graphique et sans trous de layout

- Date: 2026-04-03
- Statut: implemented
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
- Statut: implemented
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

#### M-012 - Workers: cockpit SRE mixte flotte/pools + drill-down worker

- Date: 2026-04-03
- Statut: implemented
- Portee: page Workers
- Pourquoi: donner aux profils SRE / plateforme une vue unique qui combine pilotage capacitaire, signaux d'incident et diagnostic individuel sans devoir changer d'ecran
- Reponses utilisateur integrees:
  - Mission: mix capacite, operations et diagnostic
  - Audience primaire: SRE / plateforme
  - Nature de l'ecran: mix analytique + operations
  - Granularite: equilibre strict entre vue flotte/pool et vue worker individuel
  - Preference: widgets graphiques quand c'est pertinent
- Changement:
  - Refonte de la page Workers en dashboard plus dense et plus graphique.
  - Ajout d'un bloc de synthese flotte et d'un bloc capacite / saturation par pool.
  - Ajout de widgets de surveillance rapide: workers unhealthy / silencieux, repartition des builds actifs, timeline d'incidents.
  - Conservation d'un bloc d'actions operatoires claim / complete, mais reduit a un role de support.
  - Ajout d'un explorateur interactif de workers avec selection dans la flotte et detail individuel (etat, heartbeat, capacite locale, evenements recents).
- Fichiers impactes:
  - [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
  - [docs/ux-mockups/styles.css](docs/ux-mockups/styles.css)
  - [UX.md](UX.md)

#### M-013 - Workers: passe de finition operationnelle (triage, actions guidees, taxonomie statuts)

- Date: 2026-04-03
- Statut: implemented
- Portee: page Workers
- Pourquoi: transformer l'ecran Workers en outil de decision immediate pour SRE/plateforme, sans perdre la profondeur de diagnostic individuel
- Changement:
  - Ajout d'un bandeau Triage en tete de page (Workers down, Silent > 5m, Queue at risk) pour priorisation instantanee.
  - Ajout d'un bloc Next best actions avec actions recommandees (drain/reassign/acknowledge) pour guider la reponse operationnelle.
  - Evolution du bloc capacite en vue Capacity vs Demand par pool avec tendance 30 minutes et indicateurs d'attente.
  - Enrichissement du drill-down worker en sections explicites: Runtime health, Build workload, Failure signals, Impacted runs.
  - Ajout de quick actions contextuelles sur le worker (drain, cordon, restart check) avec indication de confirmation requise.
  - Harmonisation de la taxonomie des statuts workers (healthy, degraded, unhealthy, silent) dans la vue.
- Alignement implementation:
  - Mapping conserve avec la cible Highcharts pour la vue capacitaire (column/bar + line trend), tout en gardant le drill-down en composant UI custom.
- Fichiers impactes:
  - [docs/ux-mockups/app.js](docs/ux-mockups/app.js)
  - [docs/ux-mockups/styles.css](docs/ux-mockups/styles.css)
  - [UX.md](UX.md)

#### M-014 - Ecran 4 SCM Security: triage confiance + runbook de confinement

- Date: 2026-04-04
- Statut: implemented
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
- Statut: implemented
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
- Statut: implemented
- Portee: page Observability
- Pourquoi: transformer Observability en poste d'investigation priorisee, capable de relier rapidement les signaux techniques a l'impact delivery et d'alimenter les post-mortems
- Changement:
  - Ajout d'un bandeau de triage observabilite (critical alerts, event burst, signal lag).
  - Ajout d'un bloc d'actions guidees d'investigation (open traces, suppression, forensic snapshot).
  - Refonte du flux live events avec signaux plus actionnables et filtres explicites.
  - Ajout d'un bloc Signal quality pour mesurer la sante du systeme d'observabilite (coverage/freshness/noise).
  - Ajout d'un bloc incidents par severite pour priorisation P1/P2/P3.
  - Ajout d'un bloc Correlation map (build/worker/plugin) pour reduire les allers-retours entre ecrans.
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
- Statut: implemented
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
- Statut: implemented
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
| Capacite workers / queue | Overview | `bar` horizontal, `bullet`, ou `xrange` simplifie | capacite totale, busy/idle/unhealthy, queue depth, wait time, trend |
| Ratio succes / echec | Overview | `pie`, `stacked bar`, ou `item chart` | total runs par statut sur fenetre donnee |
| SLO / disponibilite | Overview | `solidgauge`, `bullet`, ou `column` compare objectif/reel | objectifs SLO, valeur observee, budget erreur consomme, historique |
| Builds critiques | Overview | `xrange`, `bar`, ou `columnrange` selon finesse voulue | runs critiques, progression, statut, duree, phase courante, criticite |
| Flux live resume | Overview / Observability | `timeline` si licence/plugin adapte, sinon liste enrichie hors chart | evenements ordonnes, timestamp, severite, ressource, message |
| Graphe de phases pipeline | Pipelines | pas un fit naturel Highcharts standard; option `xrange` custom ou composant UI dedie | DAG de build: phases, jobs, dependances, statuts, durees |
| Progression des runs critiques | Overview | `bar` horizontal | id run, pourcentage progression, statut, ETA |
| Historique capacite / saturation | Workers / Overview | `areaspline` ou `line` | charge workers, queue depth, taux d'occupation dans le temps |

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

#### BT-004 - Capacite workers et pression de queue

- Priorite: P0
- Ecran: Overview, Workers
- Widget: capacite workers / queue
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
- Scope: tous ecrans dashboard
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
- Statut: implemented
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
- Statut: implemented
- Portee: atelier maquette
- Pourquoi: figer la validation de l'ecran 1 avant d'ouvrir une nouvelle iteration sur l'ecran 2
- Changement:
  - Ecran 1 Pipelines considere comme valide pour cette iteration.
  - L'atelier se deplace sur l'ecran 2 Overview.
- Fichiers impactes:
  - [UX.md](UX.md)

### Next

- Revue visuelle de l'ecran 2 Overview et ouverture de l'iteration suivante si besoin.
