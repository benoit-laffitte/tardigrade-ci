# Pipeline Content & Revision Tracking (CORECI-07)

## Nouveaux champs

- `JobDefinition.pipeline_content: Option<String>`
  - Contient le YAML inline du pipeline si fourni à la création du job, sinon `null` (utilise pipeline_path).
- `BuildRecord.pipeline_used: Option<String>`
  - Copie le YAML utilisé lors du déclenchement du build (pour traçabilité/reproductibilité).

## API GraphQL

### Créer un job avec pipeline inline
```graphql
mutation {
  create_job(input: {
    name: "job-inline",
    repository_url: "https://example.com/repo.git",
    pipeline_path: "pipeline.yml",
    pipeline_yaml: "stages:\n  - build\n  - test"
  }) {
    id
    name
    pipeline_content   # <= Contient le YAML fourni
  }
}
```

### Créer un job sans pipeline inline (pipeline_path)
```graphql
mutation {
  create_job(input: {
    name: "job-ref",
    repository_url: "https://example.com/repo.git",
    pipeline_path: "pipeline.yml"
    # pipeline_yaml omis ou null
  }) {
    id
    name
    pipeline_content   # <= null
  }
}
```

### Lire le pipeline utilisé pour un build
```graphql
query {
  builds {
    id
    job_id
    pipeline_used   # <= YAML utilisé ou null si pipeline_path
  }
}
```

## Notes de migration
- Les nouveaux champs sont optionnels et rétrocompatibles.
- Les tests et la documentation ont été mis à jour.
- La persistance Postgres nécessite une migration de schéma (voir storage/backend/postgres_storage.rs).

## Cas d’usage
- Reproductibilité : chaque build conserve le pipeline exact utilisé.
- Audit : possibilité de vérifier l’historique YAML pour chaque build/job.

---
Livraison : 2026-04-19
Responsable : équipe Platform
