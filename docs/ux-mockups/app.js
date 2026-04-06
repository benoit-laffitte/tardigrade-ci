const PAGES = {
  pipelines: {
    kicker: "Delivery",
    title: "Pipelines",
    panels: [
      {
        type: "build-explorer",
        title: "Builds recents et en cours",
        badge: "interactive",
        desc: "Selectionne un build pour visualiser son graphe d'etapes et le log detaille des commandes executees.",
        width: "full"
      },
      {
        title: "Queue execution",
        badge: "11 pending",
        desc: "Etat de file global pour detecter saturation et prioriser.",
        list: ["11 pending", "3 running", "2 blocked"],
        actions: [
          { label: "GET /jobs", kind: "secondary" },
          { label: "GET /builds", kind: "secondary" }
        ]
      },
      {
        title: "Incidents delivery",
        badge: "2",
        desc: "Blocage run-time impactant la livraison.",
        list: ["Runner timeout", "Artifact upload failed"],
        actions: [{ label: "POST /builds/{id}/cancel", kind: "danger" }]
      }
    ]
  },
  overview: {
    kicker: "System Health",
    title: "Overview",
    panels: [
      {
        title: "Sante globale CI",
        badge: "admin / mix realtime+tendance",
        desc: "Vue de pilotage pour suivre la sante globale de la plateforme et detecter rapidement les deviations.",
        width: "half",
        visual: {
          type: "sparkbars",
          metrics: [
            { label: "Avail.", value: "99.4%", delta: "-0.3 pt", level: "good", bars: [88, 90, 92, 95, 94, 91, 89] },
            { label: "Latency", value: "6m32", delta: "+18s", level: "warn", bars: [42, 44, 43, 48, 51, 56, 58] },
            { label: "Throughput", value: "428", delta: "+7%", level: "good", bars: [40, 48, 44, 51, 57, 62, 60] }
          ]
        },
        list: [
          "Availability: 99.4% (-0.3 pt vs 7j)",
          "Median duration: 6m32 (+18s)",
          "Throughput: 428 runs / 24h"
        ]
      },
      {
        title: "Incidents / alertes ouvertes",
        badge: "2 critiques",
        desc: "Synthese priorisee des incidents qui degradent la sante CI.",
        width: "half",
        visual: {
          type: "severity-stack",
          segments: [
            { label: "P1", value: 2, tone: "danger" },
            { label: "P2", value: 5, tone: "warn" },
            { label: "P3", value: 7, tone: "neutral" }
          ]
        },
        list: [
          "P1: High failure rate on deploy pipelines",
          "P2: Runner saturation in eu-west pool",
          "3 alerts acknowledged / 2 unresolved"
        ]
      },
      {
        title: "Capacite workers / queue",
        badge: "72% load",
        desc: "Lecture rapide de la pression d'execution et des risques de saturation.",
        width: "third",
        visual: {
          type: "meter-list",
          items: [
            { label: "Workers busy", value: 72, tone: "warn" },
            { label: "Queue pressure", value: 64, tone: "warn" },
            { label: "Retry churn", value: 21, tone: "good" }
          ]
        },
        list: [
          "Workers busy: 18/25",
          "Queue trend: +12% vs 1h",
          "Longest wait time: 14m"
        ]
      },
      {
        title: "Taux succes / echec",
        badge: "91.8% success",
        desc: "Qualite recente de l'execution CI, utile pour detecter une degradation diffuse.",
        width: "third",
        visual: {
          type: "ratio-split",
          segments: [
            { label: "Success", value: 91.8, tone: "good" },
            { label: "Failed", value: 6.4, tone: "danger" },
            { label: "Canceled", value: 1.8, tone: "neutral" }
          ]
        },
        list: [
          "Success rate: 91.8%",
          "Failure rate: 6.4%",
          "Canceled / blocked: 1.8%"
        ]
      },
      {
        title: "SLO / disponibilite",
        badge: "SLO watch",
        desc: "Vue de fiabilite orientee engagement de service.",
        width: "third",
        visual: {
          type: "slo-pills",
          items: [
            { label: "Start < 2m", value: "96.2%", tone: "good" },
            { label: "Complete < 15m", value: "89.7%", tone: "warn" },
            { label: "Error budget", value: "41%", tone: "neutral" }
          ]
        },
        list: [
          "Build start < 2m: 96.2%",
          "Pipeline completion < 15m: 89.7%",
          "Error budget consumed: 41%"
        ]
      },
      {
        title: "Builds critiques",
        badge: "5 a surveiller",
        desc: "Runs a fort impact metier ou techniquement a risque.",
        width: "two-thirds",
        visual: {
          type: "run-bars",
          items: [
            { label: "deploy-prod #1927", status: "running", progress: 62 },
            { label: "release-web #1926", status: "failed", progress: 100 },
            { label: "db-migration #1924", status: "blocked", progress: 47 },
            { label: "security-hotfix #1923", status: "pending", progress: 15 }
          ]
        },
        list: [
          "deploy-prod #1927 running",
          "release-web #1926 failed",
          "db-migration #1924 blocked",
          "security-hotfix #1923 waiting approval"
        ]
      },
      {
        title: "Flux live resume",
        badge: "rolling 15 min",
        desc: "Resume des evenements chauds sans basculer sur la page Observability.",
        width: "third",
        visual: {
          type: "pulse-feed",
          items: [
            { time: "09:12", label: "deploy-api entered test phase", tone: "good" },
            { time: "09:10", label: "release-web failed on bundle-web", tone: "danger" },
            { time: "09:07", label: "worker eu-west-3 marked unhealthy", tone: "warn" },
            { time: "09:04", label: "queue pressure crossed threshold", tone: "warn" }
          ]
        },
        list: [
          "09:12 deploy-api entered test phase",
          "09:10 release-web failed on bundle-web",
          "09:07 worker eu-west-3 marked unhealthy",
          "09:04 queue pressure crossed warning threshold"
        ]
      }
    ]
  },
  workers: {
    kicker: "Execution Plane",
    title: "Workers",
    panels: [
      {
        title: "Triage execution (au-dessus de la ligne de flottaison)",
        badge: "action now",
        desc: "Lecture en 5 secondes des signaux critiques qui demandent une action immediate.",
        width: "full",
        visual: {
          type: "triage-strip",
          items: [
            { label: "Workers down", value: "1", hint: "worker-eu-03", tone: "danger" },
            { label: "Silent > 5m", value: "1", hint: "worker-eu-11", tone: "warn" },
            { label: "Queue at risk", value: "2 pools", hint: "eu-west, shared-linux", tone: "warn" }
          ]
        },
        list: ["MTTR en cours: 7m", "Claims timeout: +11% vs 30m", "Escalade SRE active"]
      },
      {
        title: "Next best actions",
        badge: "guided ops",
        desc: "Actions recommandees pour stabiliser rapidement l'execution.",
        width: "third",
        list: [
          "Drain pool eu-west (pressure 86%)",
          "Reassign pending builds from worker-eu-03",
          "Open incident bridge for silent worker"
        ],
        actions: [
          { label: "Drain eu-west", kind: "danger" },
          { label: "Reassign builds", kind: "secondary" },
          { label: "Acknowledge", kind: "primary" }
        ]
      },
      {
        title: "Etat global de la flotte",
        badge: "SRE cockpit",
        desc: "Vue de synthese de la flotte workers avec accent sur la stabilite et la disponibilite.",
        width: "two-thirds",
        visual: {
          type: "sparkbars",
          metrics: [
            { label: "Healthy", value: "22/25", delta: "-2 vs 1h", level: "good", bars: [96, 96, 96, 92, 92, 88, 88] },
            { label: "Degraded", value: "2", delta: "+1", level: "warn", bars: [4, 4, 4, 8, 8, 12, 8] },
            { label: "Claims/min", value: "41", delta: "+8%", level: "good", bars: [28, 31, 34, 37, 35, 39, 41] }
          ]
        },
        list: ["22 healthy", "2 degraded", "1 unhealthy", "1 silent"]
      },
      {
        title: "Capacite vs demande par pool",
        badge: "3 pools",
        desc: "Vue decisionnelle charge/backlog/attente avec tendance 30 min.",
        width: "full",
        visual: {
          type: "pool-demand",
          items: [
            { label: "eu-west", capacity: 72, demand: 86, queue: "14m", trend: "+9%", tone: "danger" },
            { label: "shared-linux", capacity: 66, demand: 74, queue: "8m", trend: "+4%", tone: "warn" },
            { label: "macos-signing", capacity: 59, demand: 34, queue: "2m", trend: "-3%", tone: "good" }
          ]
        },
        list: ["2 pools au-dessus du seuil", "Tendance 30m: pression en hausse", "Recommandation: rebalancer claims"]
      },
      {
        title: "Workers unhealthy / silencieux",
        badge: "2 a traiter",
        desc: "Detection rapide des workers en derive avant impact plus large.",
        width: "third",
        visual: {
          type: "pulse-feed",
          items: [
            { time: "09:11", label: "worker-eu-03 heartbeat missing", tone: "danger" },
            { time: "09:10", label: "worker-eu-11 silent > 5m", tone: "warn" },
            { time: "09:08", label: "worker-shared-07 high retry churn", tone: "warn" },
            { time: "09:02", label: "worker-macos-02 back to healthy", tone: "good" }
          ]
        },
        list: ["1 unhealthy", "1 silent > 5m", "1 degraded escalating"]
      },
      {
        title: "Repartition des builds actifs",
        badge: "11 actifs",
        desc: "Ou se concentre le travail en cours cote execution.",
        width: "third",
        visual: {
          type: "ratio-split",
          segments: [
            { label: "Tests", value: 45, tone: "warn" },
            { label: "Build", value: 33, tone: "good" },
            { label: "Deploy", value: 22, tone: "neutral" }
          ]
        },
        list: ["Tests: 5", "Build: 4", "Deploy: 2"]
      },
      {
        title: "Actions claim / complete",
        badge: "ops",
        desc: "Operations directes sous garde-fous avant execution.",
        width: "third",
        list: ["Claim next build", "Complete build", "Inject log note", "Confirmation obligatoire"],
        actions: [
          { label: "Claim", kind: "secondary" },
          { label: "Complete", kind: "primary" }
        ]
      },
      {
        type: "worker-explorer",
        title: "Details d'un worker selectionne",
        badge: "drill-down",
        desc: "Navigue dans la flotte pour inspecter rapidement un worker individuel.",
        width: "full"
      },
      {
        title: "Timeline des incidents workers",
        badge: "rolling 30 min",
        desc: "Chronologie des signaux execution pour correlation rapide.",
        width: "full",
        visual: {
          type: "pulse-feed",
          items: [
            { time: "09:14", label: "worker-eu-03 reassigned pending builds", tone: "warn" },
            { time: "09:11", label: "worker-eu-03 heartbeat missing", tone: "danger" },
            { time: "09:08", label: "shared-linux queue saturation warning", tone: "warn" },
            { time: "08:59", label: "worker-macos-02 recovered", tone: "good" }
          ]
        },
        list: ["4 incidents in 30m", "1 unresolved", "MTTR current: 7m"]
      }
    ]
  },
  "scm-security": {
    kicker: "Trust Boundary",
    title: "SCM Security",
    panels: [
      {
        title: "Triage de confiance SCM",
        badge: "action now",
        desc: "Signaux critiques de la frontiere de confiance webhook/polling.",
        width: "full",
        visual: {
          type: "triage-strip",
          items: [
            { label: "Invalid signatures", value: "7", hint: "last 15m", tone: "danger" },
            { label: "IP rejects", value: "12", hint: "2 ranges unknown", tone: "warn" },
            { label: "Secrets expiring", value: "2", hint: "< 7 days", tone: "warn" }
          ]
        },
        list: ["1 provider en etat degrade", "Replay queue backlog: 4", "Audit trail synchronized"]
      },
      {
        title: "Actions rapides de confinement",
        badge: "runbook",
        desc: "Operations guidees pour restaurer vite un niveau de confiance acceptable.",
        width: "third",
        list: [
          "Rotate signing secret (provider: github)",
          "Quarantine unknown IP range",
          "Disable polling on compromised repo"
        ],
        actions: [
          { label: "Rotate secret", kind: "danger" },
          { label: "Quarantine IP", kind: "secondary" },
          { label: "Acknowledge", kind: "primary" }
        ]
      },
      {
        title: "Webhook security controls",
        badge: "coverage",
        desc: "Etat des controles de securite applies aux webhooks entrants.",
        width: "two-thirds",
        visual: {
          type: "meter-list",
          items: [
            { label: "Signature verification", value: 98, tone: "good" },
            { label: "IP allowlist coverage", value: 84, tone: "warn" },
            { label: "Replay protection", value: 91, tone: "good" }
          ]
        },
        list: ["Provider mapping: 4 active", "Fallback secret enabled", "Strict timestamp window: 5m"],
        actions: [{ label: "Save controls", kind: "primary" }]
      },
      {
        title: "Polling governance",
        badge: "runtime",
        desc: "Pilotage du polling par criticite repo et risques de duplication.",
        width: "half",
        visual: {
          type: "ratio-split",
          segments: [
            { label: "Enabled", value: 72, tone: "good" },
            { label: "Paused", value: 18, tone: "warn" },
            { label: "Manual", value: 10, tone: "neutral" }
          ]
        },
        list: ["Default interval: 90s", "High-risk repos: manual only", "Duplicate guard active"],
        actions: [
          { label: "Save polling", kind: "primary" },
          { label: "Run manual tick", kind: "secondary" }
        ]
      },
      {
        title: "Diagnostics rejets et doublons",
        badge: "live feed",
        desc: "Lecture forensique des rejets webhook et collisions payload.",
        width: "half",
        visual: {
          type: "pulse-feed",
          items: [
            { time: "09:21", label: "reject: invalid signature (repo api/core)", tone: "danger" },
            { time: "09:19", label: "reject: source IP not allowed", tone: "warn" },
            { time: "09:16", label: "duplicate payload id=gh-evt-8831", tone: "warn" },
            { time: "09:13", label: "accept: signature revalidated after rotate", tone: "good" }
          ]
        },
        list: ["Top reason: invalid signature", "Provider impacted: github", "Trend: +14% vs 1h"],
        actions: [{ label: "Refresh diagnostics", kind: "secondary" }]
      },
      {
        title: "Replay et evidences audit",
        badge: "compliance",
        desc: "Capacite de rejouer, tracer et exporter les preuves de controle SCM.",
        width: "full",
        list: [
          "Replay queue: 4 events pending",
          "Audit export available (JSON/CSV)",
          "Policy drift check: no drift"
        ],
        actions: [
          { label: "Replay selected", kind: "secondary" },
          { label: "Export evidence", kind: "secondary" },
          { label: "Open full audit", kind: "primary" }
        ]
      },
      {
        title: "Configuration sensible",
        badge: "guard rails",
        desc: "Edition des parametres critiques avec confirmations renforcees.",
        width: "full",
        list: ["Two-step confirmation", "Reason mandatory", "Change log mandatory"],
        actions: [{ label: "Open sensitive config", kind: "danger" }]
      }
    ]
  },
  "plugins-policy": {
    kicker: "Extensibility",
    title: "Plugins & Policy",
    panels: [
      {
        title: "Triage extensions & policy",
        badge: "action now",
        desc: "Signaux prioritaires sur la sante plugin et l'application des politiques.",
        width: "full",
        visual: {
          type: "triage-strip",
          items: [
            { label: "Plugin failures", value: "3", hint: "execute hook", tone: "danger" },
            { label: "Policy violations", value: "5", hint: "2 critical", tone: "warn" },
            { label: "Drift detected", value: "1 env", hint: "staging", tone: "warn" }
          ]
        },
        list: ["Last 15m: 2 rollbacks", "Auth dry-run healthy", "Registry latency stable"]
      },
      {
        title: "Runbook actions",
        badge: "guided ops",
        desc: "Actions recommandees pour contenir rapidement les incidents plugin/policy.",
        width: "third",
        list: [
          "Disable failing plugin version",
          "Enforce deny-all fallback policy",
          "Trigger dry-run auth replay"
        ],
        actions: [
          { label: "Disable plugin", kind: "danger" },
          { label: "Enforce fallback", kind: "secondary" },
          { label: "Acknowledge", kind: "primary" }
        ]
      },
      {
        title: "Plugin lifecycle health",
        badge: "registry",
        desc: "Etat du cycle load/init/execute/unload avec focus sur la stabilite runtime.",
        width: "two-thirds",
        visual: {
          type: "run-bars",
          items: [
            { label: "plugin-authz #2.1.0", status: "running", progress: 73 },
            { label: "plugin-sast #1.4.2", status: "failed", progress: 100 },
            { label: "plugin-notify #3.0.1", status: "pending", progress: 25 },
            { label: "plugin-cache #0.9.8", status: "success", progress: 100 }
          ]
        },
        list: ["Loaded: 11", "Init failures: 1", "Auto-recovery: enabled"],
        actions: [{ label: "Refresh lifecycle", kind: "secondary" }]
      },
      {
        title: "Policy enforcement coverage",
        badge: "governance",
        desc: "Couverture des controles policy par contexte d'execution.",
        width: "half",
        visual: {
          type: "meter-list",
          items: [
            { label: "Global policy", value: 96, tone: "good" },
            { label: "Environment policy", value: 88, tone: "warn" },
            { label: "Capability checks", value: 92, tone: "good" }
          ]
        },
        list: ["Dry-run authorization active", "2 overrides expiring", "No unsigned policy bundle"],
        actions: [{ label: "Save policy", kind: "primary" }]
      },
      {
        title: "Violations et drift",
        badge: "forensics",
        desc: "Evenements policy rejects, capabilities denied et derive de configuration.",
        width: "half",
        visual: {
          type: "pulse-feed",
          items: [
            { time: "09:33", label: "deny: deploy capability blocked (prod)", tone: "danger" },
            { time: "09:29", label: "drift: staging policy differs from baseline", tone: "warn" },
            { time: "09:26", label: "allow: dry-run replay validated", tone: "good" },
            { time: "09:23", label: "deny: plugin-sast exceeded permission scope", tone: "warn" }
          ]
        },
        list: ["5 violations in 30m", "1 critical unresolved", "Trend: +6% vs 1h"],
        actions: [{ label: "Open violation log", kind: "secondary" }]
      },
      {
        title: "Capability governance",
        badge: "guard rails",
        desc: "Edition et validation des capabilities avec garde-fous renforces.",
        width: "full",
        list: [
          "Context global / env / repository",
          "Two-step confirmation on destructive capabilities",
          "Reason + change log mandatory"
        ],
        actions: [
          { label: "Edit capabilities", kind: "secondary" },
          { label: "Dry-run auth", kind: "secondary" },
          { label: "Publish policy", kind: "primary" }
        ]
      },
      {
        title: "Inventory & provenance",
        badge: "11 loaded",
        desc: "Inventaire des plugins, versioning, provenance manifeste et statut de signature.",
        list: ["Name + state", "Manifest source", "Signature status", "Policy summary"],
        actions: [{ label: "Refresh inventory", kind: "secondary" }],
        width: "full"
      }
    ]
  },
  observability: {
    kicker: "Operational Evidence",
    title: "Observability",
    panels: [
      {
        title: "Triage observabilite",
        badge: "action now",
        desc: "Signaux critiques pour prioriser rapidement l'investigation.",
        width: "full",
        visual: {
          type: "triage-strip",
          items: [
            { label: "Critical alerts", value: "4", hint: "2 unresolved > 10m", tone: "danger" },
            { label: "Event burst", value: "+38%", hint: "last 15m", tone: "warn" },
            { label: "Signal lag", value: "22s", hint: "ingest delay", tone: "warn" }
          ]
        },
        list: ["Incident bridge active", "Correlation context loaded", "Export pipeline healthy"]
      },
      {
        title: "Actions guidees d'investigation",
        badge: "runbook",
        desc: "Actions recommandees pour accelerer le diagnostic et limiter le temps de resolution.",
        width: "third",
        list: [
          "Open correlated traces for top incident",
          "Pin noisy source and apply suppression",
          "Capture forensic snapshot"
        ],
        actions: [
          { label: "Open traces", kind: "secondary" },
          { label: "Apply suppression", kind: "danger" },
          { label: "Acknowledge", kind: "primary" }
        ]
      },
      {
        title: "Live event stream",
        badge: "stream",
        desc: "Flux temps reel filtre par severite, service, worker et build.",
        width: "two-thirds",
        visual: {
          type: "pulse-feed",
          items: [
            { time: "09:42", label: "alert: deploy-prod latency crossed threshold", tone: "danger" },
            { time: "09:40", label: "trace: worker-eu-03 timeout spike detected", tone: "warn" },
            { time: "09:38", label: "metric: queue depth recovering", tone: "good" },
            { time: "09:36", label: "event: release-web rollback completed", tone: "warn" }
          ]
        },
        list: ["Filters: severity/service/resource/time", "Sampling: adaptive", "Retention hot window: 24h"],
        actions: [{ label: "Apply filters", kind: "primary" }]
      },
      {
        title: "Signal quality",
        badge: "health",
        desc: "Qualite de l'observabilite elle-meme: couverture, bruit et fraicheur.",
        width: "half",
        visual: {
          type: "meter-list",
          items: [
            { label: "Trace coverage", value: 89, tone: "warn" },
            { label: "Log freshness", value: 94, tone: "good" },
            { label: "Noise ratio", value: 37, tone: "warn" }
          ]
        },
        list: ["Missing spans: 11%", "Dropped logs: 0.3%", "Suppression rules: 6 active"]
      },
      {
        title: "Incidents par severite",
        badge: "rolling 30m",
        desc: "Distribution rapide des incidents ouverts pour orienter la priorisation.",
        width: "half",
        visual: {
          type: "severity-stack",
          segments: [
            { label: "P1", value: 2, tone: "danger" },
            { label: "P2", value: 4, tone: "warn" },
            { label: "P3", value: 7, tone: "neutral" }
          ]
        },
        list: ["P1 unresolved: 2", "P2 acknowledged: 3", "Trend: +2 incidents vs 1h"]
      },
      {
        title: "Correlation map (build / worker / plugin)",
        badge: "cross-domain",
        desc: "Corrige le changement de contexte en reliant les signaux techniques aux impacts delivery.",
        width: "full",
        list: [
          "deploy-prod #1927 -> worker-eu-03 timeout -> plugin-sast delay",
          "release-web #1926 -> bundle-web failure -> policy deny scope",
          "shared-linux saturation -> queue pressure -> increased retries"
        ],
        actions: [
          { label: "Open correlated timeline", kind: "secondary" },
          { label: "Pin investigation", kind: "secondary" },
          { label: "Create incident", kind: "primary" }
        ]
      },
      {
        title: "Exports & forensic snapshots",
        badge: "compliance",
        desc: "Exports JSON/CSV et snapshots de preuves pour analyses externes et post-mortems.",
        width: "half",
        list: ["JSON export", "CSV export", "Snapshot with trace links", "Freshness indicator"],
        actions: [
          { label: "Export JSON", kind: "secondary" },
          { label: "Export CSV", kind: "secondary" },
          { label: "Capture snapshot", kind: "primary" }
        ]
      },
      {
        title: "Operations journal",
        badge: "audit trail",
        desc: "Journal unifie des messages systeme et actions operateur pour post-mortems.",
        list: ["Chronologie", "Messages systeme", "User actions", "Linked incidents"],
        actions: [{ label: "Open full journal", kind: "secondary" }],
        width: "full"
      }
    ]
  },
  administration: {
    kicker: "Governance",
    title: "Administration",
    panels: [
      {
        title: "Triage administration",
        badge: "action now",
        desc: "Signaux prioritaires sur la gouvernance, les acces et les operations sensibles.",
        width: "full",
        visual: {
          type: "triage-strip",
          items: [
            { label: "Pending approvals", value: "6", hint: "2 critical", tone: "danger" },
            { label: "Privilege drift", value: "3", hint: "role mismatch", tone: "warn" },
            { label: "Sensitive ops today", value: "9", hint: "100% traced", tone: "warn" }
          ]
        },
        list: ["1 high-risk change waiting", "MFA enforcement stable", "Audit pipeline healthy"]
      },
      {
        title: "Runbook de gouvernance",
        badge: "guided ops",
        desc: "Actions recommandees pour contenir les risques d'administration.",
        width: "third",
        list: [
          "Revoke temporary admin grant",
          "Force session re-auth for privileged user",
          "Freeze sensitive operation window"
        ],
        actions: [
          { label: "Revoke grant", kind: "danger" },
          { label: "Force re-auth", kind: "secondary" },
          { label: "Acknowledge", kind: "primary" }
        ]
      },
      {
        title: "Role & capability coverage",
        badge: "rbac",
        desc: "Couverture des controles RBAC et ecarts de permissions par role.",
        width: "two-thirds",
        visual: {
          type: "meter-list",
          items: [
            { label: "Role policy compliance", value: 93, tone: "good" },
            { label: "Least-privilege adherence", value: 86, tone: "warn" },
            { label: "MFA on privileged roles", value: 97, tone: "good" }
          ]
        },
        list: ["Viewer/Operator/Admin matrix synced", "3 temporary grants expiring", "No orphan admin account"],
        actions: [{ label: "Update roles", kind: "danger" }]
      },
      {
        title: "Sensitive operations control",
        badge: "guard rails",
        desc: "Actions a haut impact avec workflow de validation renforce.",
        width: "half",
        visual: {
          type: "ratio-split",
          segments: [
            { label: "Approved", value: 68, tone: "good" },
            { label: "Pending", value: 22, tone: "warn" },
            { label: "Rejected", value: 10, tone: "danger" }
          ]
        },
        list: ["Two-step confirmation", "Reason mandatory", "Dual sign-off for critical ops"],
        actions: [{ label: "Open sensitive ops", kind: "danger" }]
      },
      {
        title: "Admin access anomalies",
        badge: "forensics",
        desc: "Detection des anomalies de connexion et d'usage privilegie.",
        width: "half",
        visual: {
          type: "pulse-feed",
          items: [
            { time: "09:47", label: "anomaly: admin login from new location", tone: "warn" },
            { time: "09:43", label: "alert: repeated failed privileged action", tone: "danger" },
            { time: "09:38", label: "info: temporary grant revoked", tone: "good" },
            { time: "09:35", label: "warn: expired session used for retry", tone: "warn" }
          ]
        },
        list: ["2 unresolved anomalies", "1 session forced re-auth", "Trend: +3 vs 24h"]
      },
      {
        title: "Change approvals & maintenance windows",
        badge: "governance flow",
        desc: "Pilotage des demandes de changement et fenetres d'operations sensibles.",
        width: "full",
        list: [
          "Pending approvals by criticality",
          "Maintenance windows calendar",
          "Emergency override path with trace"
        ],
        actions: [
          { label: "Open approvals", kind: "secondary" },
          { label: "Plan window", kind: "secondary" },
          { label: "Create change request", kind: "primary" }
        ]
      },
      {
        title: "Admin activity",
        badge: "audit",
        desc: "Historique complet des actions d'administration et preuves associees.",
        list: ["Actor role", "Target", "Timestamp", "Outcome", "Evidence link"],
        actions: [
          { label: "Export admin log", kind: "secondary" },
          { label: "Open full audit", kind: "primary" }
        ],
        width: "full"
      }
    ]
  }
};

const PIPELINE_BUILDS = [
  {
    id: "#1927",
    name: "deploy-api",
    branch: "main",
    status: "running",
    duration: "03m21",
    startedAt: "09:12",
    phases: [
      {
        name: "Prepare",
        jobs: [
          {
            id: "checkout",
            name: "Checkout",
            status: "success",
            dependsOn: [],
            logs: ["$ git checkout main", "$ git submodule update --init --recursive"]
          },
          {
            id: "install",
            name: "Install deps",
            status: "success",
            dependsOn: ["checkout"],
            logs: ["$ pnpm install --frozen-lockfile", "Packages: +1320"]
          }
        ]
      },
      {
        name: "Quality gates",
        parallel: true,
        jobs: [
          {
            id: "unit-tests",
            name: "Unit tests",
            status: "running",
            dependsOn: ["install"],
            logs: ["$ pnpm test --filter api", "PASS src/health.spec.ts", "RUNNING src/integration/deploy.spec.ts"]
          },
          {
            id: "lint",
            name: "Lint",
            status: "success",
            dependsOn: ["install"],
            logs: ["$ pnpm lint", "No lint errors"]
          },
          {
            id: "security-scan",
            name: "Security scan",
            status: "pending",
            dependsOn: ["install"],
            logs: ["Pending: waiting for available security runner"]
          }
        ]
      },
      {
        name: "Package",
        jobs: [
          {
            id: "build-image",
            name: "Build image",
            status: "pending",
            dependsOn: ["unit-tests", "lint", "security-scan"],
            logs: ["Pending: quality gates not completed"]
          }
        ]
      },
      {
        name: "Deploy",
        jobs: [
          {
            id: "deploy-staging",
            name: "Deploy staging",
            status: "pending",
            dependsOn: ["build-image"],
            logs: ["Pending: image unavailable"]
          }
        ]
      }
    ],
    logs: ["Use step filter to inspect commands per step."]
  },
  {
    id: "#1926",
    name: "build-web",
    branch: "release/2.4",
    status: "failed",
    duration: "08m04",
    startedAt: "08:58",
    phases: [
      {
        name: "Prepare",
        jobs: [
          {
            id: "checkout",
            name: "Checkout",
            status: "success",
            dependsOn: [],
            logs: ["$ git checkout release/2.4"]
          },
          {
            id: "install",
            name: "Install deps",
            status: "success",
            dependsOn: ["checkout"],
            logs: ["$ pnpm install --frozen-lockfile", "Packages: +1334"]
          }
        ]
      },
      {
        name: "Quality gates",
        parallel: true,
        jobs: [
          {
            id: "unit-tests",
            name: "Unit tests",
            status: "success",
            dependsOn: ["install"],
            logs: ["$ pnpm test --filter web", "PASS 214 tests"]
          },
          {
            id: "lint",
            name: "Lint",
            status: "success",
            dependsOn: ["install"],
            logs: ["$ pnpm lint", "No lint errors"]
          },
          {
            id: "type-checks",
            name: "Type checks",
            status: "success",
            dependsOn: ["install"],
            logs: ["$ pnpm tsc --noEmit", "No type errors"]
          }
        ]
      },
      {
        name: "Build",
        jobs: [
          {
            id: "bundle-web",
            name: "Bundle web",
            status: "failed",
            dependsOn: ["unit-tests", "lint", "type-checks"],
            logs: [
              "$ pnpm build",
              "ERROR in src/pages/home.tsx:42:18",
              "Type 'undefined' is not assignable to type 'string'"
            ]
          }
        ]
      },
      {
        name: "Publish",
        jobs: [
          {
            id: "upload-artifact",
            name: "Upload artifact",
            status: "blocked",
            dependsOn: ["bundle-web"],
            logs: ["Blocked: previous step failed"]
          }
        ]
      }
    ],
    logs: ["Use step filter to inspect commands per step."]
  },
  {
    id: "#1925",
    name: "lint-backend",
    branch: "feature/scm-hardening",
    status: "success",
    duration: "02m11",
    startedAt: "08:47",
    phases: [
      {
        name: "Prepare",
        jobs: [
          {
            id: "checkout",
            name: "Checkout",
            status: "success",
            dependsOn: [],
            logs: ["$ git checkout feature/scm-hardening"]
          },
          {
            id: "install",
            name: "Install deps",
            status: "success",
            dependsOn: ["checkout"],
            logs: ["$ cargo fetch", "$ cargo metadata --format-version=1"]
          }
        ]
      },
      {
        name: "Quality gates",
        parallel: true,
        jobs: [
          {
            id: "fmt",
            name: "Fmt",
            status: "success",
            dependsOn: ["install"],
            logs: ["$ cargo fmt --check", "Formatting check passed"]
          },
          {
            id: "clippy",
            name: "Clippy",
            status: "success",
            dependsOn: ["install"],
            logs: ["$ cargo clippy --workspace -- -D warnings", "No lint violations"]
          },
          {
            id: "audit",
            name: "Audit",
            status: "success",
            dependsOn: ["install"],
            logs: ["$ cargo audit", "No vulnerable packages found"]
          }
        ]
      },
      {
        name: "Report",
        jobs: [
          {
            id: "publish-report",
            name: "Publish report",
            status: "success",
            dependsOn: ["fmt", "clippy", "audit"],
            logs: ["$ ./scripts/coverage.sh", "Coverage report published"]
          }
        ]
      }
    ],
    logs: ["Use step filter to inspect commands per step."]
  }
];

let selectedPipelineBuildId = PIPELINE_BUILDS[0].id;
let selectedPipelineStepId = "all";

const WORKERS = [
  {
    id: "worker-eu-03",
    pool: "eu-west",
    status: "unhealthy",
    heartbeat: "missing for 6m",
    activeBuilds: 2,
    successRate: "84%",
    notes: ["retry churn high", "possible network partition"],
    metrics: [
      { label: "CPU", value: 91, tone: "danger" },
      { label: "Mem", value: 74, tone: "warn" },
      { label: "Claims", value: 62, tone: "warn" }
    ],
    events: [
      { time: "09:11", label: "heartbeat missing", tone: "danger" },
      { time: "09:09", label: "claim timeout on build #1927", tone: "warn" },
      { time: "09:05", label: "retry spike detected", tone: "warn" }
    ],
    workload: ["2 builds actifs", "queue ownership conflicts: 3", "claim timeout rate: 11%"],
    affectedBuilds: ["deploy-prod #1927", "release-web #1926"]
  },
  {
    id: "worker-shared-07",
    pool: "shared-linux",
    status: "degraded",
    heartbeat: "12s ago",
    activeBuilds: 3,
    successRate: "94%",
    notes: ["high queue pressure", "stable heartbeat"],
    metrics: [
      { label: "CPU", value: 78, tone: "warn" },
      { label: "Mem", value: 63, tone: "warn" },
      { label: "Claims", value: 58, tone: "good" }
    ],
    events: [
      { time: "09:08", label: "high retry churn", tone: "warn" },
      { time: "09:03", label: "claimed build #1928", tone: "good" }
    ],
    workload: ["3 builds actifs", "queue lag: 8m", "retry churn: +9%"],
    affectedBuilds: ["test-matrix #1930", "lint-backend #1929"]
  },
  {
    id: "worker-macos-02",
    pool: "macos-signing",
    status: "healthy",
    heartbeat: "4s ago",
    activeBuilds: 1,
    successRate: "98%",
    notes: ["recovered from previous outage", "queue nominal"],
    metrics: [
      { label: "CPU", value: 39, tone: "good" },
      { label: "Mem", value: 48, tone: "good" },
      { label: "Claims", value: 24, tone: "good" }
    ],
    events: [
      { time: "08:59", label: "recovered", tone: "good" },
      { time: "08:52", label: "healthcheck passed", tone: "good" }
    ],
    workload: ["1 build actif", "queue lag: 2m", "no failures in 24h"],
    affectedBuilds: ["ios-sign #1922"]
  },
  {
    id: "worker-eu-11",
    pool: "eu-west",
    status: "silent",
    heartbeat: "missing for 8m",
    activeBuilds: 0,
    successRate: "n/a",
    notes: ["silent state detected", "awaiting node probe"],
    metrics: [
      { label: "CPU", value: 0, tone: "neutral" },
      { label: "Mem", value: 0, tone: "neutral" },
      { label: "Claims", value: 0, tone: "neutral" }
    ],
    events: [
      { time: "09:10", label: "silent > 5m threshold reached", tone: "warn" },
      { time: "09:07", label: "heartbeat jitter observed", tone: "warn" }
    ],
    workload: ["0 builds actifs", "reassign candidate", "node probe pending"],
    affectedBuilds: ["none"]
  }
];

let selectedWorkerId = WORKERS[0].id;

const SUPPORTED_API_ACTIONS = new Set([
  "GET /health",
  "POST /jobs",
  "GET /jobs",
  "POST /jobs/{id}/run",
  "POST /builds/{id}/cancel",
  "GET /builds"
]);

const PAGE_API_COVERAGE = {
  pipelines: "full",
  overview: "partial",
  workers: "roadmap",
  "scm-security": "roadmap",
  "plugins-policy": "roadmap",
  observability: "roadmap",
  administration: "roadmap"
};

const pageTitle = document.getElementById("page-title");
const pageKicker = document.getElementById("page-kicker");
const pageGrid = document.getElementById("page-grid");
const panelTemplate = document.getElementById("panel-template");
const navItems = Array.from(document.querySelectorAll(".nav-item"));

function getCurrentPage() {
  const hash = globalThis.location.hash.replace("#", "").trim();
  return PAGES[hash] ? hash : "pipelines";
}

function statusLabel(status) {
  return status.toUpperCase();
}

function apiCoverageBadge(pageKey) {
  const coverage = PAGE_API_COVERAGE[pageKey] || "roadmap";
  if (coverage === "full") {
    return "API coverage: full";
  }
  if (coverage === "partial") {
    return "API coverage: partial";
  }
  return "API coverage: roadmap";
}

function renderApiScopePanel(pageKey) {
  const coverage = PAGE_API_COVERAGE[pageKey] || "roadmap";
  const panelNode = panelTemplate.content.firstElementChild.cloneNode(true);
  panelNode.classList.add("full", "api-scope-panel");
  panelNode.querySelector(".panel-title").textContent = "Perimetre API reel (etat actuel)";
  panelNode.querySelector(".panel-badge").textContent = apiCoverageBadge(pageKey);

  const descByCoverage = {
    full: "Cette page est directement alignee avec les endpoints disponibles.",
    partial: "Cette page combine des indicateurs derives de l'API actuelle et des zones encore prospectives.",
    roadmap: "Cette page est un design cible: les actions sont desactivees tant que les endpoints ne sont pas exposes."
  };

  panelNode.querySelector(".panel-desc").textContent = descByCoverage[coverage] || descByCoverage.roadmap;

  const listNode = panelNode.querySelector(".panel-list");
  [
    "GET /health",
    "POST /jobs",
    "GET /jobs",
    "POST /jobs/{id}/run",
    "POST /builds/{id}/cancel",
    "GET /builds"
  ].forEach((entry) => {
    const li = document.createElement("li");
    li.textContent = entry;
    listNode.appendChild(li);
  });

  panelNode.querySelector(".panel-actions").remove();
  return panelNode;
}

function renderPanelVisual(panelData) {
  if (!panelData.visual) {
    return null;
  }

  const visual = document.createElement("div");
  visual.className = `panel-visual ${panelData.visual.type}`;

  if (panelData.visual.type === "sparkbars") {
    panelData.visual.metrics.forEach((metric) => {
      const block = document.createElement("div");
      block.className = "viz-block";
      const bars = metric.bars
        .map((value) => `<span class="sparkbar ${metric.level}" style="height:${value}%"></span>`)
        .join("");
      block.innerHTML = `
        <div class="viz-head">
          <span>${metric.label}</span>
          <strong>${metric.value}</strong>
        </div>
        <div class="viz-sub">${metric.delta}</div>
        <div class="sparkbar-row">${bars}</div>
      `;
      visual.appendChild(block);
    });
  }

  if (panelData.visual.type === "severity-stack" || panelData.visual.type === "ratio-split") {
    const total = panelData.visual.segments.reduce((sum, segment) => sum + segment.value, 0);
    const stack = document.createElement("div");
    stack.className = "stack-bar";
    panelData.visual.segments.forEach((segment) => {
      const item = document.createElement("span");
      item.className = `stack-segment ${segment.tone}`;
      item.style.width = `${(segment.value / total) * 100}%`;
      stack.appendChild(item);
    });
    visual.appendChild(stack);

    const legend = document.createElement("div");
    legend.className = "stack-legend";
    panelData.visual.segments.forEach((segment) => {
      const row = document.createElement("div");
      row.className = "legend-row";
      row.innerHTML = `<span class="legend-dot ${segment.tone}"></span><span>${segment.label}</span><strong>${segment.value}${panelData.visual.type === "ratio-split" ? "%" : ""}</strong>`;
      legend.appendChild(row);
    });
    visual.appendChild(legend);
  }

  if (panelData.visual.type === "meter-list") {
    panelData.visual.items.forEach((item) => {
      const row = document.createElement("div");
      row.className = "meter-row";
      row.innerHTML = `
        <div class="meter-head"><span>${item.label}</span><strong>${item.value}%</strong></div>
        <div class="meter-track"><span class="meter-fill ${item.tone}" style="width:${item.value}%"></span></div>
      `;
      visual.appendChild(row);
    });
  }

  if (panelData.visual.type === "slo-pills") {
    panelData.visual.items.forEach((item) => {
      const pill = document.createElement("div");
      pill.className = `slo-pill ${item.tone}`;
      pill.innerHTML = `<span>${item.label}</span><strong>${item.value}</strong>`;
      visual.appendChild(pill);
    });
  }

  if (panelData.visual.type === "run-bars") {
    panelData.visual.items.forEach((item) => {
      const row = document.createElement("div");
      row.className = "run-row";
      let tone = "neutral";
      if (item.status === "failed") {
        tone = "danger";
      } else if (item.status === "running") {
        tone = "warn";
      }
      row.innerHTML = `
        <div class="run-head"><span>${item.label}</span><span class="inline-status ${item.status}">${statusLabel(item.status)}</span></div>
        <div class="meter-track"><span class="meter-fill ${tone}" style="width:${item.progress}%"></span></div>
      `;
      visual.appendChild(row);
    });
  }

  if (panelData.visual.type === "pulse-feed") {
    panelData.visual.items.forEach((item) => {
      const row = document.createElement("div");
      row.className = "pulse-row";
      row.innerHTML = `<span class="legend-dot ${item.tone}"></span><span class="pulse-time">${item.time}</span><span>${item.label}</span>`;
      visual.appendChild(row);
    });
  }

  if (panelData.visual.type === "triage-strip") {
    const strip = document.createElement("div");
    strip.className = "triage-strip";
    panelData.visual.items.forEach((item) => {
      const triage = document.createElement("div");
      triage.className = `triage-item ${item.tone}`;
      triage.innerHTML = `
        <p class="triage-label">${item.label}</p>
        <p class="triage-value">${item.value}</p>
        <p class="triage-hint">${item.hint}</p>
      `;
      strip.appendChild(triage);
    });
    visual.appendChild(strip);
  }

  if (panelData.visual.type === "pool-demand") {
    const container = document.createElement("div");
    container.className = "pool-demand-grid";
    panelData.visual.items.forEach((item) => {
      let queueStatus = "healthy";
      if (item.tone === "danger") {
        queueStatus = "unhealthy";
      } else if (item.tone === "warn") {
        queueStatus = "degraded";
      }

      const row = document.createElement("div");
      row.className = "pool-demand-row";
      row.innerHTML = `
        <div class="pool-demand-head">
          <strong>${item.label}</strong>
          <span class="inline-status ${queueStatus}">${item.queue} wait</span>
        </div>
        <div class="pool-demand-bars">
          <div class="pool-demand-line"><span>Capacity</span><div class="meter-track"><span class="meter-fill good" style="width:${item.capacity}%"></span></div><strong>${item.capacity}%</strong></div>
          <div class="pool-demand-line"><span>Demand</span><div class="meter-track"><span class="meter-fill ${item.tone}" style="width:${item.demand}%"></span></div><strong>${item.demand}%</strong></div>
        </div>
        <p class="pool-demand-trend">Trend 30m: ${item.trend}</p>
      `;
      container.appendChild(row);
    });
    visual.appendChild(container);
  }

  return visual;
}

function allJobsForBuild(build) {
  return build.phases.flatMap((phase) => phase.jobs.map((job) => ({ ...job, phaseName: phase.name })));
}

function dependencyLabel(job, allJobs) {
  if (!job.dependsOn || job.dependsOn.length === 0) {
    return "depends on: none";
  }

  const byId = new Map(allJobs.map((item) => [item.id, item.name]));
  const names = job.dependsOn.map((id) => byId.get(id) || id);
  return `depends on: ${names.join(", ")}`;
}

function renderBuildDetail(container, build) {
  container.innerHTML = "";
  const jobs = allJobsForBuild(build);

  if (selectedPipelineStepId !== "all" && !jobs.some((job) => job.id === selectedPipelineStepId)) {
    selectedPipelineStepId = "all";
  }

  const meta = document.createElement("div");
  meta.className = "build-meta";
  meta.innerHTML = `
    <p><strong>${build.name} ${build.id}</strong></p>
    <p>branch=${build.branch} | status=<span class="inline-status ${build.status}">${statusLabel(build.status)}</span> | started=${build.startedAt} | duration=${build.duration}</p>
  `;

  const graph = document.createElement("div");
  graph.className = "pipeline-graph";

  build.phases.forEach((phase, index) => {
    const stage = document.createElement("div");
    stage.className = "graph-stage";

    const stageHead = document.createElement("div");
    stageHead.className = "stage-head";
    stageHead.innerHTML = `<span class="stage-title">${phase.name}</span>${
      phase.parallel ? '<span class="parallel-flag">parallel</span>' : ""
    }`;

    const jobsContainer = document.createElement("div");
    jobsContainer.className = "stage-jobs";

    phase.jobs.forEach((job) => {
      const jobNode = document.createElement("div");
      jobNode.className = `step-node ${job.status}`;
      if (job.id === selectedPipelineStepId) {
        jobNode.classList.add("active");
      }

      jobNode.innerHTML = `
        <button class="step-button" type="button" data-step-id="${job.id}">
          <span class="step-dot"></span><span class="step-name">${job.name}</span>
        </button>
        <span class="step-deps">${dependencyLabel(job, jobs)}</span>
      `;

      const stepButton = jobNode.querySelector(".step-button");
      stepButton.addEventListener("click", () => {
        selectedPipelineStepId = job.id;
        renderBuildDetail(container, build);
      });

      jobsContainer.appendChild(jobNode);
    });

    stage.appendChild(stageHead);
    stage.appendChild(jobsContainer);
    graph.appendChild(stage);

    if (index < build.phases.length - 1) {
      const connector = document.createElement("span");
      connector.className = "stage-connector";
      connector.textContent = "->";
      graph.appendChild(connector);
    }
  });

  const logLabel = document.createElement("p");
  logLabel.className = "build-log-label";
  logLabel.textContent = "Log detaille des commandes executees (filtre par etape)";

  const filterBar = document.createElement("div");
  filterBar.className = "step-filter-bar";
  const allButton = document.createElement("button");
  allButton.type = "button";
  allButton.className = selectedPipelineStepId === "all" ? "filter-chip active" : "filter-chip";
  allButton.textContent = "All steps";
  allButton.addEventListener("click", () => {
    selectedPipelineStepId = "all";
    renderBuildDetail(container, build);
  });
  filterBar.appendChild(allButton);

  jobs.forEach((job) => {
    const chip = document.createElement("button");
    chip.type = "button";
    chip.className = selectedPipelineStepId === job.id ? "filter-chip active" : "filter-chip";
    chip.textContent = job.name;
    chip.addEventListener("click", () => {
      selectedPipelineStepId = job.id;
      renderBuildDetail(container, build);
    });
    filterBar.appendChild(chip);
  });

  const log = document.createElement("pre");
  log.className = "build-log";
  const filteredLines =
    selectedPipelineStepId === "all"
      ? jobs.flatMap((job) => [
          `[${job.phaseName}] ${job.name}`,
          ...job.logs,
          ""
        ])
      : (() => {
          const selected = jobs.find((job) => job.id === selectedPipelineStepId);
          if (!selected) {
            return ["No step selected."];
          }
          return [
            `[${selected.phaseName}] ${selected.name}`,
            ...selected.logs
          ];
        })();

  log.textContent = filteredLines.join("\n");

  container.appendChild(meta);
  container.appendChild(graph);
  container.appendChild(logLabel);
  container.appendChild(filterBar);
  container.appendChild(log);
}

function renderBuildExplorer(panelData, index) {
  const panelNode = panelTemplate.content.firstElementChild.cloneNode(true);
  panelNode.style.animationDelay = `${index * 0.06}s`;
  panelNode.classList.add("full", "build-explorer");

  panelNode.querySelector(".panel-title").textContent = panelData.title;
  panelNode.querySelector(".panel-badge").textContent = panelData.badge || "";
  panelNode.querySelector(".panel-desc").textContent = panelData.desc || "";
  panelNode.querySelector(".panel-list").remove();
  panelNode.querySelector(".panel-actions").remove();

  const wrapper = document.createElement("div");
  wrapper.className = "build-explorer-layout";

  const listCol = document.createElement("div");
  listCol.className = "build-list-col";
  const listTitle = document.createElement("p");
  listTitle.className = "build-col-title";
  listTitle.textContent = "Liste des builds";
  listCol.appendChild(listTitle);

  const list = document.createElement("div");
  list.className = "build-list";
  const detailCol = document.createElement("div");
  detailCol.className = "build-detail-col";

  PIPELINE_BUILDS.forEach((build) => {
    const row = document.createElement("button");
    row.type = "button";
    row.className = "build-row";
    row.dataset.buildId = build.id;
    row.innerHTML = `
      <span class="build-row-main">${build.name} ${build.id}</span>
      <span class="build-row-sub">${build.branch} | ${statusLabel(build.status)}</span>
    `;

    if (build.id === selectedPipelineBuildId) {
      row.classList.add("active");
    }

    row.addEventListener("click", () => {
      selectedPipelineBuildId = build.id;
      selectedPipelineStepId = "all";
      list.querySelectorAll(".build-row").forEach((node) => node.classList.remove("active"));
      row.classList.add("active");
      renderBuildDetail(detailCol, build);
    });

    list.appendChild(row);
  });

  listCol.appendChild(list);

  const initialBuild =
    PIPELINE_BUILDS.find((build) => build.id === selectedPipelineBuildId) || PIPELINE_BUILDS[0];
  renderBuildDetail(detailCol, initialBuild);

  wrapper.appendChild(listCol);
  wrapper.appendChild(detailCol);
  panelNode.appendChild(wrapper);

  return panelNode;
}

function renderWorkerExplorer(panelData, index) {
  const panelNode = panelTemplate.content.firstElementChild.cloneNode(true);
  panelNode.style.animationDelay = `${index * 0.06}s`;
  panelNode.classList.add("full", "worker-explorer");

  panelNode.querySelector(".panel-title").textContent = panelData.title;
  panelNode.querySelector(".panel-badge").textContent = panelData.badge || "";
  panelNode.querySelector(".panel-desc").textContent = panelData.desc || "";
  panelNode.querySelector(".panel-list").remove();
  panelNode.querySelector(".panel-actions").remove();

  const wrapper = document.createElement("div");
  wrapper.className = "build-explorer-layout";

  const listCol = document.createElement("div");
  listCol.className = "build-list-col";
  const listTitle = document.createElement("p");
  listTitle.className = "build-col-title";
  listTitle.textContent = "Flotte workers";
  listCol.appendChild(listTitle);

  const list = document.createElement("div");
  list.className = "build-list";
  const detailCol = document.createElement("div");
  detailCol.className = "build-detail-col worker-detail-col";

  WORKERS.forEach((worker) => {
    const row = document.createElement("button");
    row.type = "button";
    row.className = "build-row";
    row.innerHTML = `
      <span class="build-row-main">${worker.id}</span>
      <span class="build-row-sub">${worker.pool} | ${statusLabel(worker.status)}</span>
    `;

    if (worker.id === selectedWorkerId) {
      row.classList.add("active");
    }

    row.addEventListener("click", () => {
      selectedWorkerId = worker.id;
      renderPage(getCurrentPage());
    });

    list.appendChild(row);
  });

  const worker = WORKERS.find((item) => item.id === selectedWorkerId) || WORKERS[0];

  const meta = document.createElement("div");
  meta.className = "build-meta";
  meta.innerHTML = `
    <p><strong>${worker.id}</strong></p>
    <p>pool=${worker.pool} | status=<span class="inline-status ${worker.status}">${statusLabel(worker.status)}</span></p>
    <p>heartbeat=${worker.heartbeat} | active_builds=${worker.activeBuilds} | success_rate=${worker.successRate}</p>
  `;

  const metricsVisual = document.createElement("div");
  metricsVisual.className = "panel-visual meter-list";
  worker.metrics.forEach((metric) => {
    const row = document.createElement("div");
    row.className = "meter-row";
    row.innerHTML = `
      <div class="meter-head"><span>${metric.label}</span><strong>${metric.value}%</strong></div>
      <div class="meter-track"><span class="meter-fill ${metric.tone}" style="width:${metric.value}%"></span></div>
    `;
    metricsVisual.appendChild(row);
  });

  const notesTitle = document.createElement("p");
  notesTitle.className = "worker-section-title";
  notesTitle.textContent = "Runtime health";

  const notes = document.createElement("ul");
  notes.className = "panel-list";
  worker.notes.forEach((note) => {
    const item = document.createElement("li");
    item.textContent = note;
    notes.appendChild(item);
  });

  const workloadTitle = document.createElement("p");
  workloadTitle.className = "worker-section-title";
  workloadTitle.textContent = "Build workload";

  const workload = document.createElement("ul");
  workload.className = "panel-list";
  worker.workload.forEach((entry) => {
    const item = document.createElement("li");
    item.textContent = entry;
    workload.appendChild(item);
  });

  const eventTitle = document.createElement("p");
  eventTitle.className = "worker-section-title";
  eventTitle.textContent = "Failure signals";

  const eventBlock = document.createElement("div");
  eventBlock.className = "panel-visual pulse-feed";
  worker.events.forEach((event) => {
    const row = document.createElement("div");
    row.className = "pulse-row";
    row.innerHTML = `<span class="legend-dot ${event.tone}"></span><span class="pulse-time">${event.time}</span><span>${event.label}</span>`;
    eventBlock.appendChild(row);
  });

  const affectedTitle = document.createElement("p");
  affectedTitle.className = "worker-section-title";
  affectedTitle.textContent = "Impacted runs";

  const affectedRuns = document.createElement("ul");
  affectedRuns.className = "panel-list";
  worker.affectedBuilds.forEach((build) => {
    const item = document.createElement("li");
    item.textContent = build;
    affectedRuns.appendChild(item);
  });

  const quickActions = document.createElement("div");
  quickActions.className = "panel-actions worker-quick-actions";
  quickActions.innerHTML = `
    <button type="button" class="danger">Drain (confirm)</button>
    <button type="button" class="secondary">Cordon (confirm)</button>
    <button type="button" class="secondary">Restart check</button>
  `;

  detailCol.appendChild(meta);
  detailCol.appendChild(notesTitle);
  detailCol.appendChild(metricsVisual);
  detailCol.appendChild(notes);
  detailCol.appendChild(workloadTitle);
  detailCol.appendChild(workload);
  detailCol.appendChild(eventTitle);
  detailCol.appendChild(eventBlock);
  detailCol.appendChild(affectedTitle);
  detailCol.appendChild(affectedRuns);
  detailCol.appendChild(quickActions);

  listCol.appendChild(list);
  wrapper.appendChild(listCol);
  wrapper.appendChild(detailCol);
  panelNode.appendChild(wrapper);

  return panelNode;
}

function renderPage(pageKey) {
  const page = PAGES[pageKey];
  pageKicker.textContent = page.kicker;
  pageTitle.textContent = page.title;

  pageGrid.innerHTML = "";
  pageGrid.appendChild(renderApiScopePanel(pageKey));

  page.panels.forEach((panelData, index) => {
    if (panelData.type === "build-explorer") {
      const explorerNode = renderBuildExplorer(panelData, index);
      pageGrid.appendChild(explorerNode);
      return;
    }

    if (panelData.type === "worker-explorer") {
      const explorerNode = renderWorkerExplorer(panelData, index);
      pageGrid.appendChild(explorerNode);
      return;
    }

    const panelNode = panelTemplate.content.firstElementChild.cloneNode(true);
    panelNode.style.animationDelay = `${index * 0.06}s`;
    if (panelData.width) {
      panelNode.classList.add(panelData.width);
    }

    panelNode.querySelector(".panel-title").textContent = panelData.title;
    panelNode.querySelector(".panel-badge").textContent = panelData.badge || "";
    panelNode.querySelector(".panel-desc").textContent = panelData.desc || "";

    const visualNode = renderPanelVisual(panelData);
    if (visualNode) {
      panelNode.querySelector(".panel-desc").insertAdjacentElement("afterend", visualNode);
    }

    const listNode = panelNode.querySelector(".panel-list");
    (panelData.list || []).forEach((entry) => {
      const li = document.createElement("li");
      li.textContent = entry;
      listNode.appendChild(li);
    });

    const actionsNode = panelNode.querySelector(".panel-actions");
    (panelData.actions || []).forEach((action) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = action.kind || "secondary";
      const isSupported = SUPPORTED_API_ACTIONS.has(action.label);
      button.textContent = isSupported ? action.label : `${action.label} (roadmap)`;
      button.disabled = !isSupported;
      if (!isSupported) {
        button.classList.add("disabled");
      }
      actionsNode.appendChild(button);
    });

    pageGrid.appendChild(panelNode);
  });

  navItems.forEach((item) => {
    const active = item.dataset.page === pageKey;
    item.classList.toggle("active", active);
    item.setAttribute("aria-current", active ? "page" : "false");
  });
}

function navigate(pageKey) {
  if (PAGES[pageKey]) {
    globalThis.location.hash = pageKey;
  }
}

navItems.forEach((item) => {
  item.addEventListener("click", () => navigate(item.dataset.page));
});

globalThis.addEventListener("hashchange", () => {
  renderPage(getCurrentPage());
});

renderPage(getCurrentPage());
