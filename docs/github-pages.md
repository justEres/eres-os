# GitHub Pages für die Rust-Doku

Diese Repo-Konfiguration nutzt **GitHub Actions + GitHub Pages Artifact Deploy** (ohne manuell gepflegten `gh-pages`-Branch).

## Warum diese Variante sinnvoll ist

- Kein manuelles Committen von generierten HTML-Dateien.
- Keine Merge-Konflikte in einem `gh-pages`-Branch.
- Reproduzierbar: Jede Doku wird aus dem aktuellen Quellstand gebaut.

## Enthaltene Automatisierung

Workflow: `.github/workflows/docs-pages.yml`

- Trigger bei Push auf `main` oder `master`.
- Führt `cargo doc --no-deps` aus.
- Packt `target/doc` als Pages-Artefakt.
- Deployt es mit `actions/deploy-pages`.

## Einrichtung im GitHub-Repo (einmalig)

1. **Settings → Pages** öffnen.
2. Bei **Build and deployment** als Source **GitHub Actions** wählen.
3. Commit mit diesem Workflow in Default-Branch pushen.
4. Nach erfolgreichem Workflow erscheint die Pages-URL.

## Wichtige Hinweise

- Du brauchst **keinen** separaten `gh-pages`-Branch.
- Die Doku wird bei jedem Push auf `main`/`master` neu veröffentlicht.
- Falls dein Default-Branch nur `master` oder nur `main` ist: der Workflow unterstützt bereits beide.
