# CLAUDE.md — Permitt
<!-- Working rules for Claude in this repository (French). Code and docs are in English. -->

Permitt : inventaire en lecture seule des permissions des agents IA d'une machine Linux —
ce que chaque hote peut faire sans demander, et quel reglage le dit. Troisieme logiciel
Blackberrylabs, apres Cleanator et MCProster. Fiche dans `Projets/00_moi/PROJETS.md`.

## Regles de travail

- BRIEF valide avant le code ; questions a Alexandre une par une ; PLAN et JOURNAL tenus a
  chaque etape ; `00_cadrage/` hors git.
- Lecture seule sans exception, aucune sortie reseau, secrets masques et jamais ecrits,
  rien d'ecrit hors `~/.config/permitt/`. Un etat, pas une note.
- Toute regle d'hote cite sa source (doc, code source ou `observed`), sa date, sa citation.
  Rien de memoire. Les fichiers de shell : liste fermee de drapeaux sources, seule la ligne
  trouvee est gardee.
- Commits locaux libres ; `git push`, suppression de fichiers, timers : accord explicite.
  Commandes donnees completes, chemin absolu (`cd /home/nushoko/Bureau/Logiciels/Permitt && …`).
- Anglais : code, commentaires, commits, README, interface par defaut. Francais : BRIEF,
  PLAN, JOURNAL, CLAUDE.md, option de langue.
- Seul nom cite : Alexandre Do.

## Identite git (incident du 2026-10-16, deux fois)

Avant le **premier commit** : `git config user.name "Alexandre Do"` et
`git config user.email "277202463+Nimeria2026@users.noreply.github.com"` en config locale ;
verifier par `git log --format='%an <%ae>'` avant tout push.

## Fondations communes — ne pas recopier, completer a la source

- `bbl-hostrules` (`Logiciels/Bbl_hostrules`, depot `blackberrylabs/bbl-hostrules`, ex-`hostrules`) :
  regles d'hotes, env, lecteurs, masquage. Ce qui manque s'ajoute la-bas.
- `bbl-ui` (`Logiciels/Bbl_ui`) : palette, briques iced, reglages, i18n — meme visage partout.
- `bbl-report` (`Logiciels/Bbl_report`) : le contrat de la tour. `permitt report` produit un
  `bbl-report/1` (`crates/core/src/report.rs`) ; le mapping des constats sur les quatre
  gravites est dans `severity_of` et dans le CHANGELOG — toute nouvelle `FindingKind` y passe.
  `permitt-app --select host[:capability]` ouvre l'app ou la tour le demande.

## Tour de controle

Permitt est un module de Blackberrylabs (`Logiciels/Blackberrylabs`) ; la marche a suivre
d'un module est `docs/new-module.md` dans la tour. Publier le module avant la tour quand le
contrat bouge. Avant tout git lance par Alexandre : `find .git -name '*.lock' -delete`.
