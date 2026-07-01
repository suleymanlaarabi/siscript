# Résumé du langage

Ce langage est un langage de script **léger, rapide et statiquement typé**, conçu pour être facilement embarqué dans des projets C, C++, Rust, Zig ou autres langages hôtes.

Son objectif principal est de permettre une intégration simple avec du code natif, avec une **interopérabilité ABI C**, un runtime minimal et le moins de surcharge possible.

## Objectifs principaux

- Typage statique et fort.
- Syntaxe moderne et claire.
- Excellente expérience développeur.
- Runtime léger et performant.
- Embedding facile dans d’autres langages.
- Appels natifs sans boxing ni conversions coûteuses.
- Support des fonctions script appelables depuis l’hôte.

## Concepts importants

Le langage distingue trois types de fonctions :

- `fn` : fonction interne au script.
- `export fn` : fonction écrite dans le script et appelable depuis l’hôte.
- `extern fn` : fonction déclarée dans le script mais implémentée par l’hôte.

L’interopérabilité repose sur des types **ABI-safe**, comme les entiers fixes, flottants, `bool`, `char`, `cstr`, `extern struct`, `extern enum`, `&T` et `&mut T`.

Les types internes au runtime comme `str`, `T[]`, les slices, les tuples et les structures normales ne peuvent pas traverser directement l’ABI.

## Système de types

Le langage possède :

- des `struct` normales pour le code script ;
- des `extern struct` avec layout C strict ;
- des `enum` normales ;
- des `extern enum` compatibles ABI C ;
- des tableaux dynamiques `T[]` ;
- des slices `&[T]` et `&mut [T]` ;
- des tuples ;
- des références `&T` et `&mut T`.

La mutabilité est explicite avec `let mut`.

## Mémoire et sécurité

Le langage utilise un modèle simple de `Copy` / `Move`.

Les types simples sont copiés automatiquement.  
Les types possédés par le runtime, comme `str` ou `T[]`, sont déplacés par défaut et doivent être clonés explicitement si nécessaire.

Un borrow checker lexical simple garantit :

- plusieurs références immutables possibles ;
- une seule référence mutable à la fois ;
- pas de mélange entre référence mutable et immutable ;
- pas de référence survivant à la valeur référencée.

## Contrôle de flux

Le langage supporte :

- `if / else`;
- `while`;
- `for`;
- `match` exhaustif sur les enums ;
- `break` et `continue`.

Les blocs peuvent retourner une valeur via leur dernière expression.

## Roadmap

La V1 se concentre sur un langage minimal mais complet :

- lexer ;
- parser ;
- AST ;
- resolver ;
- typechecker ;
- borrow checker simple ;
- interpréteur ou bytecode ;
- embedding API ;
- `export fn` / `extern fn` ;
- types ABI-safe ;
- métadonnées de module.

Les versions futures pourront ajouter :

- diagnostics avancés ;
- conversion explicite `str -> cstr` ;
- borrow checker plus précis ;
- generics ;
- `Result<T, E>` ;
- modules ;
- closures ;
- JIT ;
- enums avec données associées.
