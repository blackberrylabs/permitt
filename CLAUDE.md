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

## Fondation commune

Le coeur (regles d'hotes, env, lecteurs, masquage) vient de la crate `hostrules`
(`Logiciels/Hostrules`, depot `blackberrylabs/hostrules`). Ne pas le recopier ici ; ce qui
manque a `hostrules` s'ajoute dans `hostrules`.
