# PLAN — Permitt
<!-- Project plan (French). -->

## Etape 0. Cadrage et sources

- [x] Lecture des douleurs de la communaute ; choix du troisieme outil — 2026-10-16
- [x] Six decisions une par une (D1–D6), nom verifie — 2026-10-16
- [x] BRIEF rev. 1 ecrit — 2026-10-16
- [x] Validation du BRIEF par Alexandre (D7 : SQLite hors v0) — 2026-10-16
- [ ] `hostrules` extrait de MCProster (depot, tests), MCProster 0.2.1 migre dessus
- [x] Sources des reglages de permission, 16 hotes (doc + code source), 4 rapports + synthese dans `00_cadrage/` — 2026-10-16
- [x] D8 `judged`, D9 defauts sources, D10 hotes a reglage non lisible — validees ; observation machine (Cursor → `state.vscdb` confirme) — 2026-10-16
- [x] Schema des regles Permitt valide (D11) — 2026-10-16
- [x] hostrules 0.2.0 : extension par hote, tag pousse — 2026-10-16
- [ ] lanceurs `.desktop` / `~/.local/bin` : a trancher a l'ecriture du lecteur de shell
- [ ] Concurrents verifies (cc-safe-setup, guides, bacs a sable) — tableau dans `00_cadrage/`
- [ ] Depot git local, identite noreply **avant le premier commit**, `.gitignore`, LICENSE MIT, README — fichiers ecrits, commit 1 a lancer

## Etape 1. Regles et coeur
- [x] Modele, extension de regles (D11), traducteurs de motifs, lecteur de shell, resolution, constats — 2026-10-16
- [x] 16 fiches d'hotes sourcees + `_not-read` + `_shell` (31 drapeaux) — 2026-10-16
- [x] Faux home (44 fichiers), 11 tests dont « rien n'est ecrit » et « aucun secret » ; clippy propre — 2026-10-16
## Etape 2. CLI et passage reel
- [x] CLI roster | findings | files | shell | hosts — 2026-10-16
- [x] Passage reel : 4 hotes, OpenCode tout `auto (default)`, Cursor `unknown` (Run Mode), Claude Code `judged`, hook PreToolUse de Nimeria_poa vu ; constats de projet dedoublonnes — 2026-10-16
- [x] Commit 1 (2ebe97c) — 2026-10-16
## Etape 3. Application iced (maquette validee avant le code)
- [x] Maquette HTML validee (detail par cellule, Scope, ligne de contexte) — 2026-10-16
- [x] App iced : feuille, detail, constats, EN/FR, clair/sombre ; captures — 2026-10-16
## Etape 4. Paquets, depot, release 0.1.0 — sur accord
- [x] deb / rpm / AppImage construits dans le cloud, SHA256SUMS verifie sur la machine — 2026-10-16
- [x] Commit 2 ; .deb installe et teste sur la machine d'Alexandre (« Ok, c'est bon ») — 2026-10-16
- [x] Depot public blackberrylabs/permitt, push, tag v0.1.0, release avec les 3 paquets ; verifie depuis le cloud (sommes, binaire, arborescence, 00_cadrage absent) — 2026-10-16
## Etape 5. README, captures, textes — decision d'Alexandre
- [x] README avec captures et « Reading the sheet » — 2026-10-16
- [ ] Textes de presentation (posts) — decision d'Alexandre, pas le jour de la release

## Vers 0.2 — module de la tour de controle
- [x] `permitt report` (`bbl-report/1`), `--select` dans l'app, mapping des gravites dans le CHANGELOG — 2026-10-16
- [x] Coeur `bbl-hostrules` 0.3.0 (renommage) — 2026-10-16
- [x] Release 0.2.0 publiee (commit 5bd84c3, tag v0.2.0, trois paquets + SHA256SUMS), verifiee depuis le cloud : somme OK, `permitt report` repond bbl-report/1 — 2026-10-17

## Apres 0.1.0 (idees, non engagees)
- Lecture SQLite `mode=ro` de `state.vscdb` pour le Run Mode de Cursor (cles `yolo*` observees) et Roo Code (D7 → 0.2)
- Observer VS Code sur une machine qui a approuve un outil ; Kiro IDE `settings.json`
- Lanceurs `.desktop` et scripts `~/.local/bin` passant des drapeaux
- Antigravity CLI, Kilo Code quand leurs fichiers seront documentes
- ~~Fondation commune : extraire l'UI partagee~~ — fait, `bbl-ui` 0.1.0 (16/10)
