# Roadmap LSP production-ready — 4 semaines

## Objectif

Développer un LSP complet en Rust pour le langage.

Le LSP doit être une crate séparée dans le même workspace Rust :

```txt
crates/si_lsp
```

Il doit réutiliser le compilateur existant :

* lexer ;
* parser ;
* AST ;
* resolver ;
* typechecker ;
* borrow checker ;
* diagnostics ;
* métadonnées ABI.

Le LSP ne doit pas réimplémenter la logique du langage.

---

# 1. Architecture

## Crate à ajouter

```txt
crates/si_lsp/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── server.rs
    ├── backend.rs
    ├── document.rs
    ├── workspace.rs
    ├── analysis.rs
    ├── diagnostics.rs
    ├── hover.rs
    ├── completion.rs
    ├── goto.rs
    ├── references.rs
    ├── rename.rs
    ├── semantic_tokens.rs
    ├── symbols.rs
    ├── code_actions.rs
    └── config.rs
```

## Dépendances internes

```txt
si_lsp
 ├── si_core
 ├── si_diagnostics
 ├── si_lexer
 ├── si_parser
 ├── si_ast
 ├── si_resolver
 ├── si_typecheck
 └── lang_runtime ou si_bytecode si nécessaire
```

## Dépendances Rust recommandées

```txt
tower-lsp
lsp-types
tokio
serde
serde_json
ropey
dashmap
tracing
```

---

# 2. Principe du LSP

Le LSP est une couche tooling au-dessus du compilateur.

Pipeline utilisé :

```txt
source
  -> lexer
  -> parser
  -> resolver
  -> typechecker
  -> borrow checker
  -> diagnostics
```

Le LSP transforme ensuite les résultats du compilateur en réponses éditeur :

```txt
diagnostics
hover
completion
goto definition
references
rename
semantic tokens
symbols
code actions
```

---

# 3. Semaine 1 — Serveur LSP et diagnostics

## Objectif

Avoir un serveur LSP fonctionnel qui ouvre un fichier, suit les modifications et affiche les erreurs du langage.

## À faire

Créer la crate :

```txt
crates/si_lsp
```

Ajouter la crate au workspace.

Créer le binaire :

```txt
si-lsp --stdio
```

Implémenter les requêtes et notifications LSP de base :

```txt
initialize
initialized
shutdown
textDocument/didOpen
textDocument/didChange
textDocument/didSave
textDocument/didClose
```

Créer les structures internes :

```txt
WorkspaceState
DocumentStore
DocumentSnapshot
AnalysisResult
```

Chaque document doit stocker :

```txt
uri
version
text
line index
dernier résultat d’analyse
```

Créer une fonction centrale :

```txt
analyze_document(document) -> AnalysisResult
```

Cette fonction appelle le pipeline existant du langage.

## Diagnostics à supporter

Le LSP doit remonter :

* erreurs lexicales ;
* erreurs de syntaxe ;
* erreurs de résolution ;
* erreurs de type ;
* erreurs de mutabilité ;
* erreurs de move ;
* erreurs de borrow ;
* erreurs ABI sur `export fn` ;
* erreurs ABI sur `extern fn`.

## Robustesse obligatoire

Le serveur ne doit pas crash sur :

* fichier vide ;
* fichier incomplet ;
* syntaxe invalide ;
* string non terminée ;
* changement rapide ;
* document fermé pendant une analyse ;
* span invalide ;
* erreur interne récupérable.

## Résultat fin semaine 1

Le serveur LSP démarre.

L’éditeur affiche les diagnostics en direct.

Le LSP reste stable même sur du code cassé.

---

# 4. Semaine 2 — Navigation et compréhension du code

## Objectif

Permettre à l’utilisateur de comprendre et naviguer dans le code.

## À faire

Implémenter :

```txt
textDocument/hover
textDocument/definition
textDocument/declaration
textDocument/references
textDocument/documentSymbol
workspace/symbol
textDocument/documentHighlight
```

## Hover

Le hover doit afficher les informations utiles selon le symbole.

Pour une variable :

```txt
let mut pos: Position
```

Pour une fonction normale :

```txt
fn add(a: i32, b: i32) -> i32
```

Pour une fonction exportée :

```txt
export fn update(pos: &mut Position, dt: f32) -> void
```

Pour une fonction externe :

```txt
extern fn draw(pos: &Position)
```

Pour une struct :

```txt
struct Position
fields:
- x: f32
- y: f32
layout: stable
```

Pour une enum :

```txt
enum EntityType : i32
variants:
- Player = 0
- Enemy = 1
```

Pour un champ :

```txt
field x: f32
```

Pour un variant :

```txt
EntityType::Player
```

## Goto definition

Doit marcher sur :

* variables locales ;
* paramètres ;
* constantes ;
* fonctions ;
* fonctions exportées ;
* fonctions externes ;
* structs ;
* enums ;
* champs ;
* variants ;
* méthodes ;
* alias de type.

## References

Priorité :

1. références dans le document courant ;
2. références dans le workspace si le langage supporte plusieurs fichiers.

## Document symbols

Retourner :

* structs ;
* enums ;
* const ;
* type aliases ;
* fonctions ;
* méthodes ;
* fonctions externes ;
* fonctions exportées.

## Résultat fin semaine 2

L’utilisateur peut :

* survoler un symbole ;
* aller à sa définition ;
* voir ses références ;
* explorer la structure du fichier ;
* chercher des symboles dans le workspace.

---

# 5. Semaine 3 — Completion, semantic tokens, rename, code actions

## Objectif

Rendre le LSP réellement confortable à utiliser au quotidien.

## Completion

Implémenter :

```txt
textDocument/completion
completionItem/resolve
```

Completions attendues :

* mots-clés ;
* types primitifs ;
* variables visibles dans le scope ;
* paramètres visibles ;
* constantes ;
* fonctions ;
* fonctions exportées ;
* fonctions externes ;
* structs ;
* enums ;
* variants après `EnumName::` ;
* champs après `value.` ;
* méthodes après `value.` ;
* built-ins V1 ;
* snippets simples.

## Mots-clés à proposer

```txt
struct
enum
type
const
fn
export
extern
let
mut
if
else
while
for
in
match
return
break
continue
true
false
```

## Types primitifs à proposer

```txt
i8
i16
i32
i64
u8
u16
u32
u64
f32
f64
bool
char
void
str
cstr
```

## Semantic tokens

Implémenter :

```txt
textDocument/semanticTokens/full
```

Types à colorer :

* keyword ;
* function ;
* method ;
* variable ;
* parameter ;
* property ;
* enum ;
* enumMember ;
* struct ;
* type ;
* number ;
* string ;
* operator ;
* builtin.

Modifiers utiles :

* declaration ;
* readonly ;
* mutable ;
* exported ;
* external.

## Rename

Implémenter :

```txt
textDocument/prepareRename
textDocument/rename
```

Rename autorisé sur :

* variable locale ;
* paramètre ;
* fonction ;
* struct ;
* enum ;
* champ ;
* variant ;
* const ;
* type alias.

Rename refusé sur :

* keyword ;
* type primitif ;
* built-in ;
* symbole sans définition claire.

## Code actions simples

Implémenter :

```txt
textDocument/codeAction
```

Actions prioritaires :

* ajouter `mut` quand une variable immutable est modifiée ;
* ajouter un bras manquant dans un `match` non exhaustif ;
* proposer `c"..."` quand un `cstr` est attendu ;
* proposer un cast explicite quand une conversion implicite est refusée ;
* compléter les champs manquants dans une initialisation de struct ;
* supprimer un champ inconnu dans une initialisation de struct ;
* corriger un nom de champ proche si une suggestion fiable existe.

## Résultat fin semaine 3

Le LSP fournit :

* completion intelligente ;
* coloration sémantique ;
* rename fiable ;
* corrections rapides simples ;
* meilleure productivité dans l’éditeur.

---

# 6. Semaine 4 — Stabilisation production-ready

## Objectif

Rendre le LSP fiable, testé et utilisable dans un vrai projet.

## Tests obligatoires

Créer :

```txt
tests/lsp/
├── diagnostics/
├── hover/
├── goto/
├── references/
├── completion/
├── rename/
├── semantic_tokens/
├── code_actions/
└── symbols/
```

Tester :

* fichier vide ;
* fichier valide simple ;
* fichier avec syntaxe invalide ;
* struct ;
* enum ;
* type alias ;
* const ;
* fonction ;
* méthode ;
* fonction exportée ;
* fonction externe ;
* tableau dynamique ;
* slice ;
* tuple ;
* référence immutable ;
* référence mutable ;
* move ;
* borrow ;
* match exhaustif ;
* match non exhaustif ;
* mutation sans `mut` ;
* type incorrect ;
* champ inexistant ;
* variant inexistant ;
* signature ABI invalide ;
* conversion implicite interdite ;
* cast invalide.

## Performance

Ne pas faire d’incrémental complexe en V1.

Approche recommandée :

```txt
recompiler le document complet
+ cache
+ debounce
+ annulation des analyses obsolètes
```

À mettre en place :

* cache par document ;
* cache du dernier résultat d’analyse ;
* cache des symboles ;
* debounce sur `didChange` ;
* annulation si une nouvelle version du document arrive ;
* pas de travail concurrent inutile sur le même document ;
* conversion efficace offset ↔ ligne/colonne.

## Robustesse

Le LSP doit résister à :

* changements très rapides ;
* fichier très incomplet ;
* fichier avec beaucoup d’erreurs ;
* document fermé pendant une analyse ;
* URI inconnue ;
* spans hors limites ;
* analyse obsolète ;
* erreur interne récupérable.

Aucune panic ne doit tuer le serveur LSP.

## Packaging

Binaire final :

```txt
si-lsp
```

Commande :

```txt
si-lsp --stdio
```

Optionnel :

```txt
lang lsp
```

Le binaire séparé est recommandé pour l’intégration éditeur.

## Extension VS Code minimale

Créer :

```txt
editors/vscode/
├── package.json
├── src/
│   └── extension.ts
├── language-configuration.json
└── syntaxes/
    └── language.tmLanguage.json
```

Fonctions minimales :

* lancer `si-lsp --stdio` ;
* associer les fichiers du langage ;
* syntax highlighting de base ;
* configuration des commentaires ;
* auto-closing brackets ;
* indentation simple.

## Résultat fin semaine 4

Le LSP est utilisable dans un vrai projet.

Il fournit :

* diagnostics ;
* hover ;
* goto definition ;
* references ;
* completion ;
* semantic tokens ;
* rename ;
* code actions simples ;
* document symbols ;
* workspace symbols ;
* extension VS Code minimale ;
* tests LSP ;
* packaging propre.

---

# 7. Priorités

## P0 — obligatoire

```txt
serveur LSP
didOpen / didChange / didClose
diagnostics
hover
goto definition
completion
semantic tokens
document symbols
tests
aucune panic
binaire si-lsp --stdio
```

## P1 — très important

```txt
references
rename
workspace symbols
code actions simples
completion champs / variants / méthodes
extension VS Code minimale
```

## P2 — bonus

```txt
formatting
inlay hints
signature help
folding ranges
selection ranges
call hierarchy
```

---

# 8. Couverture attendue du langage

| Feature langage | Support LSP attendu                                      |
| --------------- | -------------------------------------------------------- |
| `fn`            | hover, goto, refs, completion                            |
| `export fn`     | hover, diagnostics ABI, symbols                          |
| `extern fn`     | hover, diagnostics ABI, completion                       |
| `struct`        | hover, completion champs, goto champs                    |
| `enum`          | hover, completion variants, match diagnostics            |
| `type`          | hover, goto, rename                                      |
| `const`         | hover, goto, completion                                  |
| `let`           | hover, rename, diagnostics                               |
| `let mut`       | hover, rename, diagnostics mutabilité                    |
| `&T`            | hover, diagnostics borrow                                |
| `&mut T`        | hover, diagnostics borrow                                |
| `str`           | hover, diagnostics ABI si utilisé en frontière interdite |
| `cstr`          | hover, diagnostics conversion                            |
| `T[]`           | completion méthodes, diagnostics mutabilité              |
| slices          | hover, diagnostics borrow                                |
| tuples          | hover, accès `.0`, `.1`                                  |
| `if`            | diagnostics condition bool                               |
| `while`         | diagnostics condition bool                               |
| `for`           | diagnostics itération                                    |
| `match`         | exhaustivité, code action                                |
| casts           | diagnostics casts invalides                              |
| built-ins       | completion, hover                                        |

---

# 9. Design recommandé

## À faire

```txt
réutiliser le compilateur existant
convertir les diagnostics internes vers LSP
utiliser les spans de si_core
utiliser les symboles du resolver
utiliser les types du typechecker
garder un cache par document
garder le LSP séparé du runtime
tester les requêtes LSP comme une API publique
```

## À éviter

```txt
réécrire un parser spécial LSP
réécrire un typechecker spécial LSP
mettre le LSP dans si_cli
mettre le LSP dans lang_runtime
faire de l’incrémental complexe trop tôt
laisser une panic tuer le serveur
```

---

# 10. Livrable final

À la fin des 4 semaines, le repo doit contenir :

```txt
crates/si_lsp
tests/lsp
editors/vscode
```

Le binaire doit fonctionner avec :

```txt
si-lsp --stdio
```

Le LSP doit couvrir :

```txt
diagnostics
hover
goto definition
references
completion
semantic tokens
rename
code actions simples
document symbols
workspace symbols
```

Objectif final :

```txt
LSP production-ready pour la V1 du langage.
```
