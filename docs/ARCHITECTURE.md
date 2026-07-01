## Principes
- Workspace Rust.
- Une crate = une responsabilité claire.
- Un fichier = un rôle précis.
- Dépendances orientées dans un seul sens.
- Aucun panic ne traverse la FFI.
- Le runtime exécute des modules compilés, il ne parse jamais du source.
- Les erreurs utilisateur produisent des diagnostics, jamais des panics.

## Règles du langage importantes
```txt
fn        = fonction script normale
export fn = fonction script exposée à l'hôte
extern fn = fonction déclarée dans le script mais fournie par l'hôte
struct    = layout stable par défaut
enum      = layout stable par défaut
```

## Pipeline global
```txt
source
  -> lexer
  -> parser
  -> AST
  -> resolver
  -> typechecker
  -> borrow/copy-move/ABI/layout checks
  -> lowering
  -> bytecode
  -> VM/runtime
  -> API Rust / API C
```

## Arborescence racine
```txt
language/
├── Cargo.toml
├── README.md
├── LANGUAGE.md
├── ROADMAP.md
├── ARCHITECTURE.md
├── EMBEDDING.md
├── CHANGELOG.md
├── crates/
│   ├── si_cli/
│   ├── si_core/
│   ├── si_diagnostics/
│   ├── si_lexer/
│   ├── si_ast/
│   ├── si_parser/
│   ├── si_resolver/
│   ├── si_typecheck/
│   ├── si_lowering/
│   ├── si_bytecode/
│   ├── si_vm/
│   ├── lang_runtime/
│   ├── lang_ffi/
│   └── si_tests/
├── tests/
│   ├── scripts/
│   ├── snapshots/
│   ├── integration/
│   └── abi/
├── examples/
│   ├── basic/
│   ├── embedding_c/
│   └── embedding_rust/
└── tools/
    ├── fuzz/
    └── scripts/
```

## `Cargo.toml` racine
```toml
[workspace]
members = [
    "crates/si_cli",
    "crates/si_core",
    "crates/si_diagnostics",
    "crates/si_lexer",
    "crates/si_ast",
    "crates/si_parser",
    "crates/si_resolver",
    "crates/si_typecheck",
    "crates/si_lowering",
    "crates/si_bytecode",
    "crates/si_vm",
    "crates/lang_runtime",
    "crates/lang_ffi",
    "crates/si_tests",
]
resolver = "2"
```

# Crates
## `si_core`
Types fondamentaux partagés. Doit rester petit, stable et sans dépendance vers les autres crates du projet.
```txt
src/
├── lib.rs
├── source.rs    # SourceFile, chemin, contenu, lignes, offset -> ligne/colonne
├── span.rs      # Span, BytePos, FileId, fusion de spans
├── id.rs        # NodeId, DefId, TypeId, FunctionId, StructId, EnumId
├── interner.rs  # stockage de chaînes internées
├── symbol.rs    # Symbol, comparaison, debug
├── target.rs    # cible ABI, arch, pointeur, alignement, convention ABI
└── hash.rs      # hash stable pour signatures, metadata, cache futur
```
## `si_diagnostics`
Gestion structurée de toutes les erreurs utilisateur. Aucune crate ne doit imprimer directement ses erreurs.
```txt
src/
├── lib.rs
├── diagnostic.rs # Diagnostic, message, code, labels, notes, suggestions
├── severity.rs   # Error, Warning, Note
├── label.rs      # span + message local + label principal/secondaire
├── code.rs       # codes stables: E0001 syntax, E0100 unknown var, etc.
├── report.rs     # DiagnosticReport, push, has_errors()
└── render.rs     # rendu humain: fichier, ligne, colonne, extrait, suggestion
```
## `si_lexer`
Transforme le texte source en tokens. Ne connaît ni sémantique, ni AST, ni typechecker.
```txt
src/
├── lib.rs
├── lexer.rs   # scan texte, espaces, commentaires, spans, erreurs
├── token.rs   # Token, TokenKind, ponctuation, opérateurs, identifiants
├── keyword.rs # struct, fn, export, extern, let, mut, const, if, else, ...
├── literal.rs # entiers, flottants, str, cstr, char, bool
└── error.rs   # chaîne non terminée, caractère invalide, nombre invalide
```
Mots-clés V1 :
```txt
struct fn export extern let mut const if else while for in match return break continue true false
```
## `si_ast`
AST brut produit par le parser. Ne contient aucune logique de résolution ou de typechecking.
```txt
src/
├── lib.rs
├── ast.rs     # Ast racine + items globaux
├── item.rs    # StructItem, EnumItem, FunctionItem, ExternFunctionItem, TypeAliasItem, ConstItem
├── stmt.rs    # let, return, while, for, break, continue, expr, assign
├── expr.rs    # call, field, tuple access, index, slice, binop, unop, if, match, struct init, literals
├── ty.rs      # primitives, nom de type, ref, slice, array, tuple, void
├── path.rs    # Position, EntityType::Player, Position::from_xy
├── pattern.rs # patterns de match, V1: variants enum simples
├── visitor.rs # parcours générique AST
└── pretty.rs  # réimpression AST pour debug/snapshots
```
Items autorisés :
```txt
struct
enum
type
const
fn
export fn
extern fn
```

## `si_parser`
Transforme des tokens en AST. Ne résout pas les noms et ne vérifie pas les types.
```txt
src/
├── lib.rs
├── parser.rs         # état, tokens, expect/eat/peek, récupération d'erreur
├── item_parser.rs    # struct, enum, type, const, fn, export fn, extern fn
├── stmt_parser.rs    # let, let mut, return, while, for, break, continue, assign, expr stmt
├── expr_parser.rs    # expressions avec priorité opérateur
├── type_parser.rs    # types syntaxiques
├── pattern_parser.rs # patterns match
├── precedence.rs     # priorité des opérateurs
└── error.rs          # erreurs parser
```
## `si_resolver`
Résout les noms et lie chaque utilisation à une définition.
```txt
src/
├── lib.rs
├── resolver.rs         # orchestration de la résolution
├── scope.rs            # scopes lexicaux, shadowing, visibilité
├── symbol_table.rs     # fonctions, exports, externs, structs, enums, consts, locals
├── def.rs              # Def::Function, ExternFunction, Struct, Enum, Local, Const
├── name_resolution.rs  # identifiants simples
├── field_resolution.rs # champs, méthodes, variants
└── error.rs            # nom inconnu, doublon, champ/variant absent, symbole non appelable
```
## `si_typecheck`
Valide types, mutabilité, Copy/Move, borrow, layout et ABI.
```txt
src/
├── lib.rs
├── typechecker.rs   # orchestre toutes les passes
├── types.rs         # types sémantiques: primitives, structs, enums, refs, slices, arrays, tuples, functions, void
├── type_table.rs    # types connus après résolution
├── infer.rs         # inférence locale simple: let x = 10
├── unify.rs         # comparaison stricte des types
├── coercion.rs      # conversions explicites seulement
├── layout.rs        # taille, alignement, offsets, enum repr, validation ABI
├── abi.rs           # signatures valides pour export fn / extern fn
├── copy_move.rs     # Copy, Move, use-after-move, clone explicite
├── borrow.rs        # &T, &mut T, conflits, durée lexicale, ref qui survit, mutation pendant emprunt
├── mutability.rs    # let vs let mut
├── match_check.rs   # exhaustivité des match sur enums
├── function_check.rs# appels, retours, signatures fn/export/extern
├── struct_check.rs  # champs dupliqués, defaults, init complète, accès champs, méthodes
├── enum_check.rs    # discriminants, variants, valeurs, exhaustivité
└── error.rs         # erreurs typecheck
```

Règles ABI :
```txt
Autorisé: entiers fixes, floats, bool, char, cstr, structs ABI-valides, enums à discriminant explicite, &T, &mut T.
Interdit direct: str, T[], &[T], &mut [T], tuples, types contenant ressources runtime.
```
Une `struct` est layout-stable par défaut, mais pas forcément ABI-valide si elle contient `str`, `T[]` ou une ressource runtime.

## `si_lowering`
Transforme l'AST typé en IR simple pour faciliter la génération bytecode et simplifier la VM.
```txt
src/
├── lib.rs
├── hir.rs          # IR typée/résolue, sans noms non résolus ni types inconnus
├── lowering.rs     # AST typé -> HIR
├── locals.rs       # index des variables locales: x -> local 0
├── control_flow.rs # if, while, for, break, continue, match
└── error.rs        # erreurs lowering
```
## `si_bytecode`
Définit le format bytecode interne. Ne l'exécute jamais.
```txt
src/
├── lib.rs
├── module.rs      # fonctions, exports, externs requis, types, constantes, metadata ABI
├── function.rs    # nom, signature, locals, instructions, constantes
├── instruction.rs # LoadConst, LoadLocal, StoreLocal, AddI32, Call, CallExtern, Return, Jump, GetField...
├── constant.rs    # table des constantes
├── value.rs       # valeurs bytecode
├── signature.rs   # signatures stables fn/export fn/extern fn
├── metadata.rs    # exports, externs, types publics, layouts, version ABI, hash signature
├── builder.rs     # API interne de construction bytecode
├── encoder.rs     # sérialisation future, minimal en V1
└── decoder.rs     # désérialisation future, minimal en V1
```
## `si_vm`
Exécute le bytecode. Ne connaît pas le parser ni le source.
```txt
src/
├── lib.rs
├── vm.rs          # boucle principale d'exécution
├── stack.rs       # pile d'exécution
├── frame.rs       # fonction courante, IP, locals, base stack
├── heap.rs        # str, T[], ressources internes
├── value.rs       # valeurs runtime, valeurs ABI, handles opaques
├── call.rs        # appels internes fn
├── extern_call.rs # appels script -> hôte via extern fn
├── export_call.rs # appels hôte -> script via export fn
├── array.rs       # tableaux dynamiques
├── string.rs      # chaînes str
├── error.rs       # index OOB, div zéro, extern manquant, signature incompatible, null pointer, limite dépassée
└── limits.rs      # stack max, call depth max, heap max, instructions max optionnelles
```
## `lang_runtime`
API Rust haut niveau pour compiler, charger et exécuter des scripts.
```txt
src/
├── lib.rs
├── runtime.rs  # Runtime: VM, externs enregistrés, modules, appels exports
├── compiler.rs # pipeline source -> lexer -> parser -> resolver -> typecheck -> lowering -> bytecode
├── module.rs   # module chargé: bytecode, metadata, état runtime, exports, externs requis
├── config.rs   # cible ABI, debug/release, limites, strict/permissif externs
├── externs.rs  # register_extern(name, signature, fn_ptr)
├── exports.rs  # call_export(name, args)
├── metadata.rs # accès metadata module
├── error.rs    # erreurs haut niveau
└── prelude.rs  # exports pratiques Rust
```
## `lang_ffi`
API C stable. Compilable en `cdylib` et/ou `staticlib`.
```txt
crates/lang_ffi/
├── include/
│   └── language.h
└── src/
    ├── lib.rs       # entrée FFI, exports C, panic boundary
    ├── api.rs       # lang_runtime_create, compile, register_extern, call_export, get_last_error...
    ├── types.rs     # types #[repr(C)]
    ├── handles.rs   # pointeurs opaques, null check, double free, use-after-free API
    ├── error.rs     # codes stables, message, destruction propre, pas de panic
    ├── callbacks.rs # fonctions hôte pour extern fn
    ├── module.rs    # fonctions C liées aux modules
    └── safety.rs    # zone unsafe centralisée autant que possible
```
`include/language.h` expose uniquement :
```txt
pointeurs opaques, entiers fixes, floats, bool ABI-stable, const char*, structs ABI-stables, enums repr(C), codes d'erreur.
```
Interdit dans l'API C :
```txt
String, Vec<T>, Result<T,E>, références Rust, enums Rust non repr(C), panic à travers FFI.
```
Exemples de handles C :
```c
typedef struct LangRuntime LangRuntime;
typedef struct LangModule LangModule;
typedef struct LangError LangError;
```
## `si_cli`
Outil terminal.
```txt
src/
├── main.rs
├── args.rs
├── commands.rs
├── cmd_check.rs    # si check file.si
├── cmd_run.rs      # si run file.si
├── cmd_build.rs    # lang build file.si
├── cmd_metadata.rs # exports, externs, types publics, layouts, version ABI
├── cmd_fmt.rs      # optionnel V1
└── output.rs
```
## `si_tests`
Helpers de tests partagés pour éviter la duplication.
```txt
src/
├── lib.rs
├── fixtures.rs   # charge scripts de test
├── compile.rs    # compiler un script dans les tests
├── run.rs        # exécuter un script VM
├── snapshot.rs   # snapshots
└── assertions.rs # erreur, type, valeur runtime, diagnostic attendu
```
# Dossiers hors crates
## `tests/`
```txt
tests/
├── scripts/
│   ├── valid/
│   └── invalid/
├── snapshots/
├── integration/
│   ├── compile.rs
│   ├── run.rs
│   ├── ffi_c.rs
│   └── embedding_rust.rs
└── abi/
    ├── layout.rs
    ├── exports.rs
    └── externs.rs
```
Scripts valides minimum :
```txt
hello.si, structs.si, enums.si, functions.si, export_fn.si,
extern_fn.si, arrays.si, tuples.si, borrow.si, match.si
```
Scripts invalides minimum :
```txt
syntax_error.si, unknown_variable.si, type_mismatch.si,
use_after_move.si, borrow_conflict.si, immutable_assignment.si,
missing_extern.si, abi_invalid_str.si, non_exhaustive_match.si
```
Tests ABI obligatoires :
```txt
taille structs, alignement, offsets champs, enum discriminant,
host -> export fn, script -> extern fn, null pointer, signature incompatible.
```
## `examples/`
```txt
examples/
├── basic/
│   ├── hello.si
│   ├── structs.si
│   ├── enums.si
│   └── arrays.si
├── embedding_rust/
│   ├── Cargo.toml
│   └── src/main.rs
└── embedding_c/
    ├── main.c
    ├── build.sh
    └── script.si
```
Exemples V1 obligatoires :
```txt
Hello world, struct stable, enum discriminant explicite,
extern fn appelée depuis script, export fn appelée depuis C,
embedding Rust minimal, embedding C minimal.
```
## `tools/`
```txt
tools/
├── fuzz/
│   ├── lexer_fuzz.rs
│   ├── parser_fuzz.rs
│   └── typecheck_fuzz.rs
└── scripts/
    ├── check_all.sh
    ├── test_all.sh
    ├── build_c_example.sh
    └── generate_header.sh
```
Priorité fuzzing :
```txt
1. lexer
2. parser
3. typechecker
```
Objectif : aucune panic sur input utilisateur invalide.
# Représentation des fonctions
## `fn`
- Définie dans le script.
- Appelée par le script.
- Peut utiliser les types internes.
- Non exposée à l'hôte par défaut.
## `export fn`
- Définie dans le script.
- Appelable depuis C/Rust/Zig/etc.
- Signature validée ABI.
- Visible dans les metadata du module.
## `extern fn`
- Déclarée dans le script.
- Sans corps script.
- Implémentée par l'hôte.
- Appelée par le script.
- Signature validée ABI.
- Doit être enregistrée dans le runtime avant exécution/appel.

# Représentation des types
## Layout stable par défaut
```txt
struct Position {
    x: f32,
    y: f32,
}
enum EntityType : i32 {
    Player = 0,
    Enemy = 1,
}
```
Pas de :
```txt
extern struct
extern enum
```
## ABI-valide
Autorisé :
```txt
i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 bool char cstr
struct ABI-valide
enum avec discriminant explicite
&T
&mut T
```
Interdit directement :
```txt
str
T[]
&[T]
&mut [T]
tuples
types contenant ressources runtime
```
# Gestion des erreurs
## Erreurs utilisateur
Produire un `Diagnostic`.
Exemples :
```txt
syntaxe invalide, type incorrect, nom inconnu, borrow invalide, signature ABI invalide
```
## Erreurs internes
Réservées aux bugs/invariants cassés.
Règles :
```txt
Erreur utilisateur -> jamais panic.
Panic autorisée uniquement pour invariant impossible, bug compilateur ou test.
Panic interdite à travers API C.
```
# Sécurité FFI
Règles :
```txt
vérifier tous les pointeurs C entrants
null pointer -> erreur propre
valider les strings C
capturer les panics
handles opaques détruits par fonctions prévues
aucun type Rust interne exposé
unsafe minimal, localisé, commenté, testé
```
Zones unsafe prioritaires :
```txt
lang_ffi
si_vm::extern_call
```
# Dépendances
## Autorisées
```txt
si_diagnostics -> si_core
si_lexer       -> si_core, si_diagnostics
si_ast         -> si_core
si_parser      -> si_core, si_diagnostics, si_lexer, si_ast
si_resolver    -> si_core, si_diagnostics, si_ast
si_typecheck   -> si_core, si_diagnostics, si_ast, si_resolver
si_lowering    -> si_core, si_diagnostics, si_ast, si_typecheck
si_bytecode    -> si_core
si_vm          -> si_core, si_diagnostics, si_bytecode
lang_runtime     -> crates compilateur + si_vm
lang_ffi         -> lang_runtime
si_cli         -> lang_runtime, si_diagnostics
si_tests       -> lang_runtime, si_diagnostics
```
## Interdites
```txt
si_core -> autre crate projet
si_lexer -> si_parser
si_ast -> si_parser
si_ast -> si_typecheck
si_vm -> si_parser
lang_ffi -> si_parser directement
si_bytecode -> si_vm
```
# Ordre d'implémentation conseillé
```txt
1.  si_core
2.  si_diagnostics
3.  si_lexer
4.  si_ast
5.  si_parser
6.  si_resolver
7.  si_typecheck
8.  copy_move.rs
9.  borrow.rs
10. abi.rs
11. layout.rs
12. si_lowering
13. si_bytecode
14. si_vm
15. appels fn
16. appels extern fn
17. appels export fn
18. lang_runtime
19. API Rust
20. lang_ffi
21. API C
22. exemples C/Rust
23. CLI
24. diagnostics propres
25. tests ABI
26. tests intégration
27. fuzzing
28. documentation finale
```
# Documentation racine
```txt
private_docs/RESUME.md      -> résumer rapide du projet/langage
private_docs/LANGUAGE.md    -> spécification du langage
private_docs/ROADMAP.md     -> plan d'implémentation
private_docs/ARCHITECTURE.md-> architecture du code
```
# Critères production-ready
L'architecture est valide si :
```txt
chaque crate a une responsabilité claire
les dépendances vont dans un seul sens
l'API C n'expose aucun type Rust interne
aucun panic ne traverse la FFI
le runtime ne dépend pas du parser
les diagnostics sont structurés
les tests sont séparés par niveau
l'unsafe est limité et documenté
native fn n'existe pas
extern fn représente les fonctions hôte
struct/enum sont layout-stables par défaut
les règles ABI sont centralisées dans abi.rs et layout.rs
```
# Résumé strict
```txt
core        -> types fondamentaux
diagnostics -> erreurs structurées
lexer       -> texte vers tokens
ast         -> AST brut
parser      -> tokens vers AST
resolver    -> noms vers définitions
typecheck   -> types, ABI, move, borrow
lowering    -> AST typé vers HIR
bytecode    -> format exécutable interne
vm          -> exécution
runtime     -> API Rust haut niveau
ffi         -> API C stable
cli         -> outil terminal
tests       -> helpers et intégration
```
Objectif final : V1 petite, rapide, stable, testable et réellement embeddable.
