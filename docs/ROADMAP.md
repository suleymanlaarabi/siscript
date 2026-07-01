# Roadmap 8 semaines — Langage script embeddable en Rust

Objectif : obtenir une V1 production-ready en 8 semaines.

Le langage doit être :

- léger ;
- rapide ;
- statiquement typé ;
- facilement embeddable ;
- ABI-stable par défaut ;
- simple à utiliser depuis C, C++, Rust, Zig ou tout autre langage hôte ;
- capable d’appeler des fonctions fournies par l’hôte ;
- capable d’exposer des fonctions de script à l’hôte.

## Semaine 1 — Base du compilateur

### Objectif

Transformer un fichier source en AST propre.

### À faire

- Finaliser la grammaire V1.
- Implémenter le lexer.
- Implémenter le parser.
- Créer l’AST.
- Gérer les erreurs de syntaxe.
- Ajouter les tests du lexer.
- Ajouter les tests du parser.
- Parser les éléments principaux :
  - `struct` ;
  - `enum` ;
  - `fn` ;
  - `export fn` ;
  - `extern fn` ;
  - `let` ;
  - `let mut` ;
  - `const` ;
  - `if / else` ;
  - `while` ;
  - `for` ;
  - `match` ;
  - expressions simples ;
  - appels de fonctions ;
  - accès aux champs ;
  - initialisation de structs.

### Résultat attendu

- Un fichier script peut être lu.
- Le langage produit un AST valide.
- Les erreurs de syntaxe sont compréhensibles.
- L’AST distingue clairement :
  - fonction interne ;
  - fonction exportée ;
  - fonction externe fournie par l’hôte.

---

## Semaine 2 — Resolver, scopes et symboles

### Objectif

Comprendre les noms, les scopes et les déclarations.

### À faire

- Créer une table des symboles.
- Gérer les scopes lexicaux.
- Résoudre :
  - variables ;
  - constantes ;
  - fonctions internes `fn` ;
  - fonctions exportées `export fn` ;
  - fonctions externes `extern fn` ;
  - structs ;
  - enums ;
  - champs de structs ;
  - variants d’enums.
- Détecter :
  - variable inconnue ;
  - fonction inconnue ;
  - nom déjà défini ;
  - champ inexistant ;
  - variant inexistant ;
  - appel à une déclaration non appelable.
- Empêcher les collisions dangereuses :
  - deux fonctions du même nom ;
  - un type et une fonction avec un nom ambigu si la grammaire ne peut pas les séparer proprement.
- Ajouter les tests du resolver.

### Résultat attendu

- Chaque nom pointe vers une déclaration connue.
- Les erreurs de résolution sont propres.
- Le compilateur est prêt pour le typechecker.
- Les fonctions `extern fn` sont connues comme des symboles requis par l’hôte.

---

## Semaine 3 — Typechecker et ABI stable par défaut

### Objectif

Rendre le langage statiquement typé et poser les règles ABI de base.

### À faire

- Implémenter les types primitifs :
  - `i8` ;
  - `i16` ;
  - `i32` ;
  - `i64` ;
  - `u8` ;
  - `u16` ;
  - `u32` ;
  - `u64` ;
  - `f32` ;
  - `f64` ;
  - `bool` ;
  - `char` ;
  - `cstr` ;
  - `void`.
- Implémenter les types composés :
  - structs ;
  - enums ;
  - tuples ;
  - tableaux dynamiques `T[]` ;
  - slices `&[T]` ;
  - slices mutables `&mut [T]` ;
  - références `&T` ;
  - références mutables `&mut T`.
- Vérifier :
  - appels de fonctions ;
  - types des arguments ;
  - types de retour ;
  - accès aux champs ;
  - affectations ;
  - conditions booléennes ;
  - `match` exhaustif sur enums ;
  - initialisation complète des structs ;
  - absence de conversions implicites dangereuses.
- Définir les règles de layout stable :
  - ordre des champs conservé ;
  - alignement/padding déterministes ;
  - métadonnées de taille et d’alignement disponibles ;
  - type discriminant explicite pour les enums exposables proprement.
- Ajouter des diagnostics clairs.
- Ajouter les tests du typechecker.

### Résultat attendu

- Un programme incorrect est rejeté avant exécution.
- Les types sont fiables.
- Les structs ont un layout stable.
- Les fonctions `fn`, `export fn` et `extern fn` sont typées de façon uniforme.

---

## Semaine 4 — Move, Copy et borrow checker simple

### Objectif

Sécuriser la mémoire sans faire un Rust complet.

### À faire

- Définir quels types sont `Copy`.
- Définir quels types sont `Move`.
- Gérer les moves pour :
  - `str` ;
  - `T[]` ;
  - structs contenant des valeurs non-copy ;
  - tuples contenant des valeurs non-copy.
- Interdire l’utilisation après move.
- Implémenter un borrow checker lexical simple :
  - plusieurs `&T` autorisés ;
  - un seul `&mut T` autorisé ;
  - pas de mélange entre `&T` et `&mut T` ;
  - pas de référence qui survit à la valeur référencée ;
  - pas de mutation d’un conteneur pendant qu’une référence vers un élément existe.
- Vérifier `let mut`.
- Interdire la mutation sans `mut`.
- Définir les règles des références reçues depuis l’hôte :
  - `&T` = pointeur non-null en lecture ;
  - `&mut T` = pointeur non-null mutable ;
  - pas de stockage d’une référence hôte après retour d’un appel.
- Ajouter les tests mémoire/sécurité.

### Résultat attendu

- Pas d’utilisation après move.
- Pas d’aliasing mutable dangereux.
- Pas de mutation non autorisée.
- Les références utilisées par l’API d’embedding sont sûres et prévisibles.

---

## Semaine 5 — Backend d’exécution

### Objectif

Rendre le langage exécutable.

### Choix conseillé

Pour une V1 production-ready : commencer par une VM bytecode simple.

Éviter le JIT en V1, car cela ajoute trop de complexité.

### À faire

- Définir un bytecode minimal.
- Compiler l’AST typé vers bytecode.
- Implémenter une VM simple.
- Gérer :
  - variables locales ;
  - appels de fonctions internes ;
  - retours de fonctions ;
  - branches ;
  - boucles ;
  - structs ;
  - enums ;
  - tableaux ;
  - tuples ;
  - références ;
  - slices.
- Ajouter une représentation runtime des valeurs.
- Limiter les allocations inutiles.
- Préparer une représentation compatible avec les appels hôte :
  - valeurs primitives directes ;
  - pointeurs pour `&T` / `&mut T` ;
  - structs avec layout stable.
- Ajouter les tests d’exécution.

### Résultat attendu

- Les scripts peuvent être exécutés.
- Les fonctions internes marchent.
- Les structures, enums, tableaux, tuples et boucles fonctionnent.
- La VM est prête à appeler des `extern fn` et à exposer des `export fn`.

---

## Semaine 6 — Embedding, `extern fn` et `export fn`

### Objectif

Rendre le langage réellement embeddable.

### À faire

- Définir précisément les types autorisés aux frontières hôte/script.
- Valider les signatures de `extern fn`.
- Valider les signatures de `export fn`.
- Gérer les appels script vers hôte :
  - résolution d’une `extern fn` ;
  - vérification que l’hôte l’a enregistrée ;
  - appel du pointeur de fonction ;
  - conversion minimale des arguments runtime vers ABI stable.
- Gérer les appels hôte vers script :
  - récupération d’une `export fn` ;
  - vérification de la signature ;
  - appel avec des valeurs hôte ;
  - retour propre vers l’hôte.
- Créer l’API Rust d’embedding :
  - créer un runtime ;
  - charger un script ;
  - compiler un module ;
  - enregistrer une fonction externe ;
  - appeler une fonction exportée ;
  - récupérer les erreurs.
- Créer l’API C :
  - créer/détruire un runtime ;
  - charger un script ;
  - enregistrer une fonction externe ;
  - appeler une fonction exportée ;
  - lire les erreurs.
- Empêcher les panic Rust de traverser la FFI.
- Créer un format stable pour les métadonnées de module :
  - fonctions exportées ;
  - fonctions externes requises ;
  - structs ;
  - enums ;
  - tailles ;
  - alignements ;
  - signatures.

### Résultat attendu

- Un programme C peut charger un script.
- Le script peut appeler une fonction fournie par l’hôte via `extern fn`.
- L’hôte peut appeler une `export fn`.
- Les types traversent l’interface sans boxing inutile.
- Les erreurs d’intégration sont propres.

---

## Semaine 7 — DX, diagnostics et tooling

### Objectif

Rendre le langage agréable à utiliser.

### À faire

- Améliorer tous les messages d’erreur.
- Ajouter :
  - position ligne/colonne ;
  - extrait de code ;
  - message clair ;
  - suggestion simple quand possible.
- Créer une CLI :
  - compiler un fichier ;
  - exécuter un fichier ;
  - vérifier un fichier sans exécuter ;
  - afficher les métadonnées d’un module ;
  - afficher les exports ;
  - afficher les externs requis.
- Ajouter une commande de test.
- Ajouter des exemples officiels :
  - hello world ;
  - structs ;
  - enums ;
  - tableaux ;
  - `extern fn` ;
  - `export fn` ;
  - embedding depuis Rust ;
  - embedding depuis C.
- Écrire la documentation minimale :
  - syntaxe ;
  - types ;
  - mémoire ;
  - ABI stable par défaut ;
  - embedding ;
  - limites de la V1.

### Résultat attendu

- Le langage est utilisable par quelqu’un d’autre que toi.
- Les erreurs sont compréhensibles.
- Les exemples montrent clairement comment intégrer le langage.
- Le modèle `fn` / `export fn` / `extern fn` est évident.

---

## Semaine 8 — Stabilisation production-ready

### Objectif

Transformer le prototype en V1 solide.

### À faire

- Ajouter des tests d’intégration.
- Ajouter des tests ABI/layout.
- Ajouter des tests d’embedding C.
- Ajouter des tests d’embedding Rust.
- Ajouter des tests d’erreurs.
- Ajouter des tests de non-régression.
- Faire du fuzzing sur :
  - lexer ;
  - parser ;
  - typechecker.
- Mesurer les performances :
  - temps de compilation ;
  - temps d’exécution ;
  - coût d’appel `extern fn` ;
  - coût d’appel `export fn` ;
  - coût de passage d’une struct par valeur ;
  - coût de passage d’une struct par référence.
- Corriger les crashs.
- Vérifier qu’aucune panic Rust ne traverse la FFI.
- Stabiliser l’API publique.
- Versionner la V1.
- Écrire un `README.md` propre.
- Écrire un `LANGUAGE.md` propre.
- Écrire un `EMBEDDING.md` propre.
- Écrire un `CHANGELOG.md`.

### Résultat attendu

- La V1 est testée.
- La V1 est documentée.
- L’API embedding est stable.
- Le langage peut être utilisé dans un vrai projet.
- La compatibilité hôte/script est claire, testée et stable.

---

# Priorités absolues

## 1. Garder la V1 petite

Ne pas ajouter en V1 :

- generics ;
- traits ;
- closures ;
- macros ;
- classes ;
- héritage ;
- surcharge ;
- exceptions ;
- JIT ;
- modules complexes ;
- async ;
- GC complexe.

La V1 doit être simple, stable et embeddable.

---

## 2. ABI stable par défaut

Le langage ne doit pas demander à l’utilisateur de marquer tous ses types comme “C-compatible”.

La règle V1 :

```txt
struct = layout stable par défaut
fn = convention d’appel stable par défaut
export fn = visible depuis l’hôte
extern fn = fournie par l’hôte
```

Tous les types ne sont pas forcément simples à transmettre directement à l’hôte, mais leur comportement doit être clair.

Types simples à transmettre directement :

- entiers fixes ;
- flottants ;
- `bool` ;
- `char` ;
- `cstr` ;
- structs contenant uniquement des types simples ;
- enums avec discriminant explicite ;
- `&T` ;
- `&mut T`.

Types runtime à encadrer strictement :

- `str` ;
- `T[]` ;
- slices ;
- tuples contenant des types runtime ;
- structs contenant des champs runtime.

Ces types peuvent exister dans le langage, mais leur passage à l’hôte doit être soit interdit en V1, soit représenté par un handle/runtime opaque documenté.

---

## 3. L’embedding doit être simple

L’utilisateur doit pouvoir :

1. créer un runtime ;
2. charger un script ;
3. enregistrer ses fonctions hôte correspondant aux `extern fn` ;
4. appeler une fonction `export fn` ;
5. récupérer les erreurs proprement ;
6. lire les métadonnées du module.

---

## 4. Les erreurs doivent être propres

Une erreur doit toujours dire :

- où est l’erreur ;
- ce qui ne va pas ;
- ce qui était attendu ;
- comment corriger si possible.

Exemples importants :

- fonction `extern fn` non enregistrée ;
- signature hôte incompatible ;
- type incorrect dans un appel ;
- référence invalide ;
- champ inexistant ;
- utilisation après move ;
- mutation sans `mut`.

---

## 5. La V1 doit être stable avant d’être ambitieuse

Mieux vaut une V1 petite, rapide et fiable qu’un langage énorme mais fragile.

La priorité n’est pas d’avoir beaucoup de features.

La priorité est :

- embedding simple ;
- ABI stable ;
- erreurs propres ;
- performances correctes ;
- comportement prévisible.

---

# Livrables finaux de la V1

À la fin des 8 semaines, le projet doit contenir :

- compilateur Rust ;
- lexer ;
- parser ;
- AST ;
- resolver ;
- typechecker ;
- borrow checker simple ;
- VM bytecode ;
- API Rust ;
- API C ;
- support `extern fn` ;
- support `export fn` ;
- layout stable pour les structs ;
- représentation stable pour les enums ;
- métadonnées de module ;
- diagnostics propres ;
- CLI ;
- exemples ;
- tests ;
- documentation ;
- README ;
- LANGUAGE.md ;
- EMBEDDING.md ;
- CHANGELOG.md.

---

# Définition de production-ready

Le langage est considéré production-ready V1 si :

- il compile les scripts valides ;
- il rejette les scripts invalides proprement ;
- il ne crash pas sur une erreur utilisateur ;
- il peut être embeddé depuis C ;
- il peut être embeddé depuis Rust ;
- les fonctions `extern fn` marchent ;
- les fonctions `export fn` marchent ;
- les structs ont un layout stable ;
- les enums exposées ont une représentation stable ;
- les erreurs sont lisibles ;
- les tests principaux passent ;
- l’API publique est documentée ;
- les limites de la V1 sont clairement indiquées.

---

# Ordre conseillé d’implémentation

1. Lexer
2. Parser
3. AST
4. Resolver
5. Typechecker
6. Layout stable des types
7. Move / Copy
8. Borrow checker simple
9. Bytecode
10. VM
11. `extern fn`
12. `export fn`
13. API Rust
14. API C
15. Métadonnées de module
16. Diagnostics
17. CLI
18. Tests
19. Documentation
20. Stabilisation V1
