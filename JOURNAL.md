# JOURNAL — Permitt
<!-- Project journal (French), newest first. -->

---

## 2026-10-16, tard — le rapport pour la tour

- La tour de controle Blackberrylabs est cadree (BRIEF valide) et son contrat publie :
  `bbl-report` 0.1.0. Permitt est le premier module a le parler : `permitt report` (toujours
  JSON) et `permitt-app --select host[:capability]`, qui remplace la variable
  `PERMITT_SELECT` sans la casser.
- Rien d'inedit dans le rapport : les quatre chiffres des cartes, les constats, les fichiers,
  les `hints.act`, les URL de documentation ; secrets masques comme partout. Les constats
  `unknown` (D10) y entrent en information, avec la raison. Un constat de shell retrouve le
  texte de sa ligne (masque) et son fichier comme preuve ; son id est le drapeau.
- Mapping sur les quatre gravites ecrit dans le CHANGELOG : contournement shell = agir
  maintenant ; agit sans demander / un modele decide = a regarder ; liste de refus seule /
  non honore = bon a savoir ; illisible, contradictoire, inconnu = information.
- Test de conformite sur la maison factice (8 tests coeur) ; `--select codex:files` verifie
  a l'ecran dans le cloud. Interface et sortie `--json` inchangees.

---

## 2026-10-16, nuit — 0.1.0 publie

- Depot public, tag, release avec les trois paquets ; verifie depuis le cloud : sommes OK,
  binaire extrait de l'AppImage repond 0.1.0, arborescence complete, `00_cadrage/` absent.
- Bilan de la journee : MCProster 0.2.0 et 0.2.1, hostrules 0.1.0 et 0.2.0, Permitt 0.1.0 —
  du cadrage (sept decisions le soir, quatre le lendemain matin) a la release, avec un
  passage reel entre chaque etape. La convention « fondation commune au troisieme outil »
  a ete appliquee le jour meme ou elle a ete ecrite.
- Ce que Permitt a montre sur la machine d'Alexandre : OpenCode tout sans demander par
  defaut, Claude Code en mode `auto` (un modele decide) par defaut, Cursor au Run Mode
  illisible. Rien n'avait ete regle ; c'est exactement le cas que D9 rend visible.

---

## 2026-10-16, nuit — l'application

- Maquette HTML avec les quatre hotes reels, validee d'un mot. Deux choix propres a Permitt :
  le detail s'ouvre par **cellule** (la question est toujours « pourquoi cette case dit ca »),
  et une ligne de contexte sous le nom d'hote plutot qu'une colonne de plus.
- App iced ecrite d'une traite sur le squelette de MCProster ; compile du premier coup,
  clippy propre. Ajustements vus en capture (Xvfb) : glyphe ⚑ absent de la police → ▲ ;
  bloc de preuve sur deux lignes (la valeur en monospace se cassait caractere par
  caractere) ; libelles de filtres plus courts en francais ; marge droite du panneau.
  `PERMITT_SELECT=hote:capacite` pre-selectionne une cellule pour les captures de doc.
- Paquets construits, sommes verifiees sur la machine. Reste : le test du .deb et la release.

---

## 2026-10-16, nuit — le coeur et les seize fiches

- Etape 0 le soir (quatre enquetes, une synthese, une observation sur la machine), D8 a D11,
  hostrules 0.2.0, puis le coeur de Permitt et les seize fiches dans la foulee. Le schema
  D11 a tenu, avec quatre ajustements decouverts en ecrivant les fiches : `file` (un
  reglage ne vit que dans un fichier d'un niveau), `whole_tools` (Goose nomme des outils
  entiers, jamais des motifs), `not_honoured_values` (Claude Code ignore deux valeurs, pas
  la cle), `overrides_state` (une note d'inconnu qui n'enleve rien a l'etat — Kiro, Zed).
- Deux lecons de modele : un bac a sable n'est pas toujours une permission (Gemini, Qwen,
  VS Code et Kiro gardent leurs demandes dedans → garde-fou ; Codex, Claude Code, Cursor
  non → `sandboxed`) ; un motif seul ne tranche pas un inconnu (l'allowlist de Cursor sous
  un Run Mode illisible reste `unknown`, la liste dans les preuves).
- Le CLI Cline (`toolAutoApprove`) ecrasait l'extension au meme niveau : ramene a une note.
  Premier passage reel reussi : la feuille dit vrai sur quatre hotes reels ; les constats de
  projet repetaient ceux du niveau utilisateur quand l'etat etait le meme — corrige.

---

## 2026-10-16, naissance

- Soir de la publication de MCProster 0.1.0 et 0.2.0. Alexandre : « Quel est le prochain
  module le plus interessant ? », puis « regarde dans la communaute quelles sont les vraies
  douleurs ». Resultat : les agents qui executent sans demander — incidents de 50 a 370 Go,
  aucun inspecteur lecture seule multi-hotes. « Ok, on y va pour l'inventaire des
  permissions des agents. »
- Six decisions une par une (BRIEF §10). Nom : Permitt, verifie (crates.io, PyPI, npm
  libres ; Permit.io est un service d'autorisation web, voisin a connaitre).
- D6 : c'est le troisieme outil → extraction du coeur commun `hostrules` maintenant (regles
  d'hotes sourcees, env, lecteurs, masquage), UI commune apres Permitt. Convention posee le
  meme soir dans `00_moi/CONVENTIONS.md`.
- Dossiers `Logiciels/Permitt` et `Logiciels/Hostrules` crees par Alexandre. BRIEF rev. 1
  ecrit et valide le soir meme ; D7 : les reglages en SQLite restent hors v0, avec la raison.
