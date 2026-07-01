# AGENTS.md

Ce document définit les règles que toute IA ou agent de développement doit suivre lorsqu’il modifie ce projet.

## Objectif principal

Le langage doit rester :

- simple ;
- rapide ;
- fiable ;
- embeddable ;
- ABI-stable par défaut ;
- facile à maintenir.

La performance est une priorité majeure, autant pour :

- le lexer ;
- le parser ;
- le resolver ;
- le typechecker ;
- le borrow checker ;
- la génération bytecode ;
- la VM ;
- les appels `extern fn` ;
- les appels `export fn` ;
- l’API d’embedding.

## Règles obligatoires

### 1. Toujours écrire des tests

Pour chaque changement, l’agent doit ajouter ou mettre à jour les tests correspondants.

Cela inclut :

- tests unitaires ;
- tests d’intégration ;
- tests d’erreurs ;
- tests ABI ;
- tests runtime ;
- tests de non-régression si un bug est corrigé.

Un changement sans test est considéré incomplet.

### 2. Toujours vérifier les cas limites

Avant d’implémenter une fonctionnalité, l’agent doit se demander :

- Est-ce que ce cas est défini dans `LANGUAGE.md` ?
- Est-ce que ce comportement est clair ?
- Est-ce qu’il y a une ambiguïté ?
- Est-ce que les erreurs possibles sont documentées ?
- Est-ce que les cas invalides sont rejetés proprement ?
- Est-ce que les cas limites sont testés ?

Si un comportement n’est pas défini, il ne faut pas l’inventer silencieusement : il faut soit l’ajouter clairement à la documentation, soit refuser l’implémentation tant que la règle n’est pas décidée.

### 3. Toujours penser performance

Chaque décision doit être évaluée avec la performance en tête.

L’agent doit éviter :

- les allocations inutiles ;
- les copies inutiles ;
- le boxing inutile ;
- les conversions cachées ;
- les recherches répétées coûteuses ;
- les structures de données trop génériques ;
- les abstractions qui ralentissent la VM ou l’embedding.

Le code critique doit rester simple, prévisible et cache-friendly.

### 4. Ne jamais casser l’ABI sans raison

Le langage est ABI-stable par défaut.

L’agent doit faire attention à :

- la représentation mémoire des types ;
- l’ordre des champs des structs ;
- la taille des types primitifs ;
- l’alignement ;
- les signatures de `extern fn` ;
- les signatures de `export fn` ;
- les appels depuis C/Rust/Zig/autres hôtes.

Toute modification liée à l’ABI doit avoir des tests dédiés.

### 5. Toujours lancer les tests

Après chaque modification, l’agent doit vérifier son travail en lançant les tests disponibles.

Commandes attendues selon le contexte :

```sh
cargo test
cargo clippy
cargo fmt --check
```

Si certains tests ne peuvent pas être lancés, l’agent doit l’indiquer clairement.

### 6. Garder un fichier par responsabilité

Le code doit rester découpé proprement.

Éviter les fichiers énormes qui mélangent plusieurs responsabilités.

Exemples :

- lexer séparé du parser ;
- AST séparé du typechecker ;
- resolver séparé du borrow checker ;
- bytecode séparé de la VM ;
- API Rust séparée de l’API C ;
- diagnostics séparés de la logique métier.

Chaque fichier doit avoir une responsabilité claire.

### 7. Ne pas ajouter de features inutiles

La V1 doit rester petite et stable.

Ne pas ajouter sans validation :

- generics ;
- traits ;
- classes ;
- héritage ;
- macros ;
- closures ;
- JIT ;
- exceptions ;
- surcharge ;
- modules complexes.

Une fonctionnalité non prévue doit d’abord être discutée et documentée.

## Checklist avant de finir une tâche

Avant de considérer une tâche terminée, l’agent doit vérifier :

- Le comportement est-il défini dans la spec ?
- Le code est-il bien séparé ?
- Les erreurs sont-elles propres ?
- Les cas invalides sont-ils testés ?
- Les cas limites sont-ils testés ?
- Les performances sont-elles acceptables ?
- L’ABI est-elle respectée ?
- Les tests passent-ils ?
- Le formatage est-il correct ?
- La documentation doit-elle être mise à jour ?

## Principe final

Ne jamais chercher seulement à “faire marcher”.

Chaque changement doit être :

- correct ;
- testé ;
- rapide ;
- simple ;
- maintenable ;
- cohérent avec la spec.

et bien decouper le code en plusieur fichier aucun fichier de PLUS DE 600 lignes !
