# BRIEF — Permitt
<!-- Project brief (French). Code, comments, commits and README are in English. -->

Rev. 1 — 2026-10-16, sept decisions prises une par une avec Alexandre. **Validee le 2026-10-16.**
Rev. 1.1 — meme soir, apres l'etape 0 : D8 a D10 (etat `judged`, defauts sources, hotes a reglage non lisible).

## 1. Objet

**Pour chaque assistant installe sur cette machine : qu'est-ce qu'il peut faire sans
demander — lancer des commandes, modifier des fichiers, acceder au reseau, appeler des
outils MCP — et quel reglage, dans quel fichier, le dit.**

Permitt lit les fichiers de reglages des hotes (et les fichiers de shell, D2), produit une
feuille d'appel hotes × capacites, et des constats verifiables. Il ne change rien.

Phrase d'accroche : *Permitt — what your AI agents are allowed to do on this machine
without asking, and which setting says so.*

## 2. Pourquoi

Lecture des douleurs de la communaute du 16/10 (`Mcproster/00_cadrage/2026-10-16_douleurs_communaute.md`) :
la douleur la plus documentee est l'agent qui execute sans demander — modes YOLO / Turbo /
Full-Access, listes de refus contournables (`bash -c`, base64, scripts). Incidents
verifies : Cursor `rm -rf ~/` malgre la protection, Cursor Windows ~250 Go, Codex
Full-Access ~370 Go (10+ cas), Copilot `D:` entier, Claude Code #49129 (~50 Go) et #99193
(~116 Go), Windsurf `rm` au lieu de `git rm` ; CVE-2025-53773 (injection qui active
`chat.tools.autoApprove`). Ce qui existe **ecrit** (cc-safe-setup installe des hooks, Claude
Code seulement) ou explique outil par outil. Personne ne **montre** l'etat reel d'une
machine, tous hotes confondus. Un tiers des visiteurs du gist qui liste les incidents
arrivent par moteur de recherche : les gens cherchent apres coup.

## 3. Posture : l'inspecteur, pas le garde-fou

Permitt n'installe pas de hooks, ne modifie aucun reglage, ne bloque rien. Il dit ce qui
est, cite d'ou il le tient, et affiche l'action possible (le reglage a changer, la
commande de l'hote) — jamais executee. Meme ligne que Cleanator et MCProster : montrer,
expliquer, ne pas agir a la place de l'utilisateur. Famille Blackberrylabs : meme look,
anglais par defaut, francais en option.

Frontiere avec MCProster (D1) : MCProster = quels serveurs MCP, ou, combien ils pesent ;
Permitt = quelles permissions chaque hote s'est donnees. `autoApprove` / `trust` **par
serveur** restent a MCProster ; les reglages **de l'hote** sont a Permitt. Un lien dans
chaque sens, pas de doublon.

## 4. Le modele

- **Capacites** (D1) : `commands` (lancer des commandes shell), `files` (creer, modifier,
  supprimer des fichiers), `network` (acces reseau de l'agent ou de son bac a sable : web
  fetch, requetes), `mcp` (appeler des outils MCP sans confirmation, au niveau de l'hote).
- **Etat d'une capacite** (D3, un etat, pas une note) : `asks` (demande a chaque fois),
  `auto` (n'importe quoi sans demander), `allowlist(n)` (sans demander pour n motifs
  autorises), `denylist(n)` (tout sauf n motifs refuses — contournable, on le dit),
  `sandboxed` (sans demander mais dans un bac a sable : lequel), `off` (capacite
  desactivee), `unknown` (reglage non documente ou illisible : on le dit, on ne devine pas).
- **Niveaux** (D4) : `managed` (politique d'organisation), `user`, `project`, `local`
  (fichier de projet non versionne), `shell` (alias / fonction / script de lancement, D2),
  `flag` (drapeau documente d'une ligne de commande vue dans un alias). Pour chaque projet
  connu (racines `--root` + projets connus de Claude Code), la **resolution** : ce qui
  s'applique en fait, avec l'ordre de priorite **cite** quand la doc de l'hote le donne,
  « ordre non documente » sinon.
- **Garde-fous presents** : hooks (Claude Code `PreToolUse`), bac a sable actif, liste de
  refus — montres comme tels, avec leur limite connue (une liste de refus se contourne).

## 5. Hotes (a etablir a l'etape 0, chaque reglage avec sa source)

| Hote | Reglages connus ou a verifier |
|---|---|
| Claude Code | `~/.claude/settings.json`, `.claude/settings.json`, `.claude/settings.local.json`, politique managee : `permissions.allow/deny/ask`, `defaultMode` (`default`, `acceptEdits`, `plan`, `bypassPermissions`), hooks `PreToolUse` ; drapeaux `--dangerously-skip-permissions`, `--permission-mode` (alias de shell) |
| Codex CLI | `~/.codex/config.toml`, `.codex/config.toml` : `approval_policy` (`untrusted`, `on-failure`, `on-request`, `never`), `sandbox_mode` (`read-only`, `workspace-write`, `danger-full-access`), `[sandbox_workspace_write] network_access` ; drapeau `--dangerously-bypass-approvals-and-sandbox` |
| Cursor | Run Mode de l'agent (Allowlist / Allowlist with Sandbox / auto), listes allow/deny, protection de suppression — emplacement sur disque a etablir |
| VS Code / Copilot | `settings.json` : `chat.tools.autoApprove`, `chat.tools.terminal.autoApprove` (allow/deny), Workspace Trust |
| Copilot CLI | `--allow-all-tools`, `--allow-tool`, `--deny-tool` ; fichier de config a etablir |
| Gemini CLI / Qwen Code | `settings.json` : `autoAccept`, `sandbox`, `tools.allowed` / `coreTools` ; drapeau `--yolo` |
| Windsurf | Turbo mode, listes allow/deny — emplacement a etablir |
| Cline / Roo Code / Kilo | reglages d'auto-approbation (lecture, ecriture, commandes, navigateur, MCP) — emplacement a etablir (globalState ?) |
| OpenCode | `opencode.json` : `permission` (`edit`, `bash`, `webfetch` : `ask` / `allow` / `deny`, motifs) |
| Goose | `config.yaml` : `GOOSE_MODE` (`auto`, `approve`, `chat`, `smart_approve`) |
| Zed | `settings.json` : agent `always_allow_tool_actions` |
| Kiro | mode `autopilot` / `supervised`, `trustedCommands` — a etablir |
| Amp | `settings.json` : `amp.commands.allowlist`, permissions — a etablir |
| Aider | `--yes-always`, `.aider.conf.yml` `yes-always` |

Six hotes ou plus sans fichier lisible resteront hors v0, avec la raison, comme pour
MCProster.

## 6. Regles

- **R1 Lecture seule, sans exception.** Aucun fichier d'hote ni de shell n'est ecrit.
- **R2 Aucune sortie reseau.**
- **R3 Secrets masques, jamais ecrits.** Les fichiers de reglages peuvent contenir des
  cles (meme module que MCProster, via `hostrules`).
- **R4 Fichiers de shell (D2)** : `~/.bashrc`, `~/.zshrc`, `~/.profile`, `~/.bash_aliases`,
  `~/.config/fish/config.fish` — liste fermee de drapeaux dangereux **sources dans la doc
  de chaque hote** ; seule la ligne qui correspond est gardee (fichier, numero, ligne) ;
  option pour desactiver cette lecture (`--no-shell`, reglage de l'app).
- **R5 Un etat, pas une note (D3).** Pas de score, pas de couleur-jugement ; les constats
  sont verifiables et ordonnes : ce qui execute sans demander d'abord.
- **R6 Hote = regle declarative sourcee** (`hosts/*.toml`, schema `hostrules`) ;
  documentation, code source ou `observed`, date, citation. Rien de memoire.
- **R7 Resolution honnete (D4)** : ordre de priorite cite ou « non documente ».
- **R8 Rien d'ecrit hors `~/.config/permitt/`** (reglages de l'interface ; pas de resultats).
- **R9 Anglais partout, francais en option** ; app en anglais au demarrage.
- **R10 Reutiliser** : `hostrules` (regles, env, lecteurs, masquage), recette Cleanator /
  MCProster pour l'app et les paquets ; pas de fusion.
- **R11 Actions affichees, jamais executees** : le reglage a changer et ou, la commande de
  l'hote quand elle existe.

## 7. Architecture

- `hostrules` (crate commune, depot `blackberrylabs/hostrules`, D6) : modele d'hote et
  d'emplacement sources, validation, resolution des emplacements (OS, portees, racines de
  projet), lecteurs JSON/JSONC/TOML/YAML, `dig`, masquage des secrets.
- `crates/core` : regles Permitt (`hosts/*.toml` avec les cles de reglages par capacite),
  lecture des fichiers de shell (R4), modele `Capability`/`State`/`Level`, resolution par
  projet, constats, actions.
- `crates/cli` : `permitt hosts | roster | findings | files | --json | --home | --root | --no-shell`.
- `crates/app` (iced, look Cleanator) : table hotes × capacites, detail d'un hote (chaque
  niveau, chaque reglage, fichier:ligne, Open / Copy), constats avec actions, selecteur de
  projet pour la resolution.
- Tests : faux home avec tous les hotes et leurs niveaux, « rien n'est ecrit », « aucun
  secret en clair », fichiers de shell avec et sans alias dangereux.

## 8. Etapes

0. Sources : pour chaque hote, la doc des reglages de permission (cles, valeurs, ordre de
   priorite, drapeaux) ; concurrents verifies ; `hostrules` extrait de MCProster et
   MCProster migre dessus (0.2.1, sans changement visible).
1. Regles et coeur : `hosts/*.toml`, lecteur de shell, modele, resolution, constats ; faux
   home ; tests.
2. CLI ; passage reel sur la machine d'Alexandre.
3. App iced (maquette validee avant le code).
4. Paquets, depot, release 0.1.0 — sur accord.
5. README avec captures, textes de presentation — decision d'Alexandre.

## 9. Hors perimetre

Installer ou modifier des hooks, des reglages, des bacs a sable ; surveiller en continu ;
juger (« risque eleve ») ; lire les traces et transcriptions (autre outil, parque) ;
sonder les agents en les lancant.

## 10. Decisions (datees)

- **D1 2026-10-16** — Phrase et frontiere : quatre capacites, une question, une source par
  reglage ; `autoApprove`/`trust` par serveur restent a MCProster.
- **D2 2026-10-16** — Lire les fichiers de shell, liste fermee de drapeaux sources, seule la
  ligne gardee, desactivable.
- **D3 2026-10-16** — Un etat, pas une note ; constats verifiables seulement.
- **D4 2026-10-16** — Tous les niveaux avec resolution ; ordre cite ou « non documente ».
- **D5 2026-10-16** — Nom : Permitt (crates.io, PyPI, npm libres ; voisin Permit.io =
  autorisation web, l'accroche dit « on this machine »).
- **D6 2026-10-16** — Cœur commun extrait maintenant (`hostrules`), UI commune apres Permitt.

- **D8 2026-10-16** — Etat `judged` ajoute : un classifieur (modele) decide, l'humain n'est pas
  consulte (Claude Code `auto`, Cursor Auto-review, Qwen `auto`, Goose `smart_approve`, VS Code
  Assisted, Devin Smart). Trois regimes : tu decides / un modele decide / personne ne decide.
- **D9 2026-10-16** — Defauts sources : chaque regle d'hote declare l'etat par defaut de chaque
  capacite (URL + citation) ; Permitt affiche l'etat effectif et marque « (defaut) ». Doc
  contradictoire → `unknown` avec les deux citations.
- **D10 2026-10-16** — Hotes dont le reglage central n'est pas dans un fichier documente (Cursor
  editeur, Devin Desktop, VS Code par outil, Zed confiance) : gardes dans la feuille, ce qui est
  sur disque est lu, le reste est `unknown` **motive** (ou il est range, ou verifier). Observe le
  soir meme sur la machine d'Alexandre : Cursor range tout dans `state.vscdb` (cles `yolo*`).
- **D11 2026-10-16** — Schema des regles Permitt valide : partie commune hostrules + `level` par
  emplacement ; par hote `[defaults]` (D9, source), `[[settings]]` (`kind` = `mode` | `rules` |
  `guard`, `states` valeur → etat par capacite, `syntax` du traducteur de motifs,
  `not_honoured_in`), `[precedence]` (`order`, `lists`, `status` documented | undocumented |
  contradictory, source). Syntaxe de motifs inconnue → `unknown`, jamais une devinette.
  Consequence : hostrules 0.2.0 avec extension par hote (`HostRule<L, H = NoExt>`).
- **D7 2026-10-16** — Reglages vivant dans une base SQLite `globalState` (Cline, Roo Code,
  Cursor pour partie) : **hors v0, avec la raison** (blobs internes non documentes, base
  ouverte en WAL par l'editeur, verrou possible → `unknown` aleatoire). Lecture `mode=ro`
  envisagee en 0.2. Les hotes concernes apparaissent dans la feuille avec cette raison.

Ouverte : faut-il lire aussi les lanceurs `.desktop` et les scripts de `~/.local/bin` qui
passent des drapeaux ? (a trancher a l'etape 0, avec les sources.)
