# LANGUAGE.md

# Spécification du langage

Ce document définit la syntaxe et les règles sémantiques du langage.

Le langage est conçu pour être :
- simple à parser
- statiquement typé
- rapide
- facilement embarquable dans n'importe quel langage hôte
- compatible ABI stable par défaut
- utilisable sans conversions cachées, sans boxing et sans surcoût inutile

## 1. Principe central

Le langage est **ABI-stable par défaut**.

Cela signifie que les types et fonctions du langage doivent avoir une représentation stable, prévisible et directement utilisable par un hôte natif.

Il n'existe pas deux mondes séparés entre :
- les types script
- les types natifs
- les fonctions script
- les fonctions hôte

Le langage doit être pensé comme un langage de script fortement typé, mais dont les valeurs peuvent être manipulées efficacement par l'hôte.

## 2. Mots-clés de fonction

Le langage distingue seulement trois formes de fonctions.

```txt
fn
export fn
extern fn
```

### 2.1 `fn`

Une fonction normale écrite dans le script.

```txt
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

Une `fn` est appelable depuis le script.

Par défaut, sa signature utilise les mêmes règles de type et de layout que le reste du langage.

### 2.2 `export fn`

Une fonction écrite dans le script et explicitement exposée à l'hôte.

```txt
export fn update_position(pos: &mut Position, dt: f32) {
    pos.x += dt
    pos.y += dt
}
```

Une `export fn` sert à dire :

> cette fonction fait partie de l'interface publique du module.

L'hôte peut la récupérer par son nom ou par les métadonnées du module.

### 2.3 `extern fn`

Une fonction déclarée dans le script, mais fournie par l'hôte.

```txt
extern fn draw_position(pos: &Position)
```

`extern fn` signifie seulement :

> cette fonction existe, mais son corps est fourni ailleurs.

Le langage ne considère pas les fonctions de l'hôte comme un cas spécial : elles suivent les mêmes règles de signature que les fonctions normales.

## 4. Déclarations globales autorisées

Un fichier source contient uniquement des déclarations globales.

```txt
struct
enum
type
const
fn
export fn
extern fn
```

Aucun code exécutable n'est autorisé directement au niveau global.

Valide :

```txt
const SPEED: f32 = 1.0

fn main() {
    // code ici
}
```

Interdit :

```txt
let x = 10
print_i32(x)
```

## 5. Types primitifs

Types entiers :

```txt
i8 i16 i32 i64
u8 u16 u32 u64
```

Types flottants :

```txt
f32 f64
```

Autres types primitifs :

```txt
bool
char
void
str
cstr
```

### 5.1 `bool`

`bool` vaut `true` ou `false`.

Il est stocké de manière stable et peut traverser une frontière d'appel.

### 5.2 `char`

`char` représente un scalaire Unicode.

Il est stocké sur 32 bits.

```txt
let c: char = 'a'
```

### 5.3 `void`

`void` représente l'absence de valeur de retour.

Ces deux formes sont équivalentes :

```txt
fn log() -> void {
}

fn log() {
}
```

## 6. Chaînes

Le langage distingue deux types de chaînes.

```txt
str
cstr
```

### 6.1 `str`

`str` est une chaîne possédée par le runtime.

```txt
let name = "player"
```

Elle peut nécessiter de la mémoire runtime.

Elle n'est pas un simple pointeur.

### 6.2 `cstr`

`cstr` est une chaîne constante terminée par `\0`.

```txt
let path = c"assets/player.png"
```

Elle est utilisée pour passer une chaîne brute à une fonction externe sans allocation cachée.

### 6.3 Règle importante

Il n'y a pas de conversion implicite entre `str` et `cstr`.

Interdit :

```txt
extern fn log(msg: cstr)

fn main() {
    log("hello")
}
```

Valide :

```txt
extern fn log(msg: cstr)

fn main() {
    log(c"hello")
}
```

## 7. Structures

### 7.1 Syntaxe

```txt
struct Position {
    x: f32,
    y: f32,
}
```

Les structures ont un layout stable par défaut.

L'ordre des champs est conservé.

Les champs sont typés statiquement.

### 7.2 Initialisation

```txt
let pos = Position { x: 10.0, y: 20.0 }
```

L'ordre des champs n'a pas d'importance à l'initialisation.

```txt
let pos = Position { y: 20.0, x: 10.0 }
```

### 7.3 Valeurs par défaut

Les champs peuvent avoir une valeur par défaut.

```txt
struct Position {
    x: f32 = 0.0,
    y: f32 = 0.0,
}
```

Dans ce cas, les champs avec valeur par défaut peuvent être omis.

```txt
let pos = Position {}
let pos2 = Position { x: 5.0 }
```

### 7.4 Méthodes

Une structure peut contenir des fonctions.

```txt
struct Position {
    x: f32,
    y: f32,

    fn from_xy(x: f32, y: f32) -> Position {
        Position { x: x, y: y }
    }

    fn move_by(&mut self, dx: f32, dy: f32) {
        self.x += dx
        self.y += dy
    }

    fn length(&self) -> f32 {
        sqrt_f32(self.x * self.x + self.y * self.y)
    }
}
```

Les méthodes ne changent jamais le layout de la structure.

## 8. Enums

### 8.1 Syntaxe simple

```txt
enum EntityType {
    Player,
    Enemy,
    Npc,
}
```

Par défaut, un enum utilise un discriminant stable.

### 8.2 Type de discriminant explicite

```txt
enum EntityType : i32 {
    Player = 0,
    Enemy = 1,
    Npc = 2,
}
```

Le type de discriminant est recommandé pour les enums exposés à l'hôte.

Types autorisés :

```txt
i8 i16 i32 i64
u8 u16 u32 u64
```

### 8.3 Utilisation

```txt
let kind = EntityType::Player
```

## 9. Alias de type

```txt
type Entity = (Position, Velocity)
```

Un alias ne crée pas un nouveau type.

Il donne seulement un nom à un type existant.

## 10. Variables

### 10.1 Immutabilité par défaut

```txt
let x = 10
```

`x` ne peut pas être réassigné.

### 10.2 Mutabilité explicite

```txt
let mut x = 10
x = 20
```

### 10.3 Type explicite optionnel

```txt
let x: i32 = 10
let y = 20
```

Le type peut être inféré si possible.

## 11. Constantes

Les constantes sont globales et immutables.

```txt
const MAX_ENTITY_COUNT: u32 = 1000
```

Le type est obligatoire.

## 12. Mutabilité

La mutabilité appartient à la variable ou à la référence.

```txt
let pos = Position { x: 0.0, y: 0.0 }
pos.x = 1.0 // interdit

let mut pos = Position { x: 0.0, y: 0.0 }
pos.x = 1.0 // autorisé
```

## 13. Références

Le langage supporte deux types de références.

```txt
&T
&mut T
```

### 13.1 Référence immutable

```txt
let pos = Position { x: 0.0, y: 0.0 }
let ref_pos = &pos
```

Permet de lire sans copier.

### 13.2 Référence mutable

```txt
let mut pos = Position { x: 0.0, y: 0.0 }
let ref_pos = &mut pos
```

Permet de modifier sans copier.

### 13.3 Règles de borrow

Règles V1 :

1. Plusieurs `&T` peuvent exister en même temps.
2. Une seule `&mut T` peut exister à la fois.
3. `&T` et `&mut T` ne peuvent pas exister en même temps sur la même valeur.
4. Une référence ne peut jamais survivre à la valeur référencée.
5. Une référence reçue depuis l'hôte ne peut pas être stockée après le retour de la fonction.

## 14. Copy et Move

### 14.1 Types Copy

Sont `Copy` :

- entiers ;
- flottants ;
- `bool` ;
- `char` ;
- `cstr` ;
- enums ;
- structs contenant uniquement des champs `Copy` ;
- tuples contenant uniquement des types `Copy`.

Exemple :

```txt
let a = 10
let b = a
print_i32(a) // ok
```

### 14.2 Types Move

Sont `Move` :

- `str` ;
- tableaux dynamiques ;
- structs contenant au moins un champ non-Copy ;
- tuples contenant au moins un élément non-Copy.

Exemple :

```txt
let a = "hello"
let b = a
// print(a) // interdit : a a été déplacé
print(b)
```

### 14.3 Clone explicite

```txt
let a = "hello"
let b = clone(a)
print(a)
print(b)
```

`clone` est explicite.

Il n'y a pas de copie profonde cachée.

## 15. Tableaux dynamiques

### 15.1 Type

```txt
T[]
```

Exemple :

```txt
let mut values: i32[] = []
```

### 15.2 Méthodes de base

```txt
values.push(10)
values.length()
values.clear()
```

`push` et `clear` nécessitent une variable mutable.

## 16. Slices

Types :

```txt
&[T]
&mut [T]
```

Exemples :

```txt
let slice = values[0..4]
let all = values[..]
let mut_slice = &mut values[..]
```

Une slice est une vue sur un tableau existant.

Elle ne possède pas les données.

## 17. Tuples

### 17.1 Type

```txt
(Position, Velocity)
```

### 17.2 Création

```txt
let entity = (pos, vel)
```

### 17.3 Accès

```txt
entity.0
entity.1
```

### 17.4 Déstructuration

```txt
let (position, velocity) = entity
```

## 18. Fonctions

### 18.1 Fonction avec retour

```txt
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

La dernière expression est retournée implicitement.

### 18.2 Fonction sans retour

```txt
fn log_position(pos: &Position) {
    print_f32(pos.x)
}
```

Équivaut à :

```txt
fn log_position(pos: &Position) -> void {
    print_f32(pos.x)
}
```

### 18.3 Retour explicite

```txt
fn abs_i32(v: i32) -> i32 {
    if v < 0 {
        return -v
    }

    v
}
```

## 19. Fonctions exportées

Une fonction exportée est une fonction publique du module.

```txt
export fn update(pos: &mut Position, dt: f32) {
    pos.x += dt
    pos.y += dt
}
```

L'hôte peut :

- voir cette fonction dans les métadonnées du module ;
- récupérer son pointeur ou son handle ;
- l'appeler avec des valeurs compatibles ;
- modifier les données passées par référence mutable.

## 20. Fonctions externes

Une fonction externe est déclarée dans le script mais fournie par l'hôte.

```txt
extern fn draw_cube(pos: &Position, size: f32)
```

Elle n'a pas de corps.

L'hôte doit l'enregistrer avant l'exécution si le script l'utilise.

Exemple :

```txt
extern fn draw_cube(pos: &Position, size: f32)

fn main() {
    let pos = Position { x: 0.0, y: 0.0 }
    draw_cube(&pos, 1.0)
}
```

## 21. Règles d'interface hôte

L'hôte doit pouvoir :

- créer un runtime ;
- charger un module ;
- compiler ou interpréter un script ;
- enregistrer des `extern fn` ;
- lire les métadonnées du module ;
- récupérer les `export fn` ;
- appeler une `export fn` ;
- transmettre des références vers ses propres données ;
- détruire proprement le runtime.

## 22. Métadonnées de module

Un module compilé doit exposer :

- la liste des `export fn` ;
- leur signature ;
- la liste des `extern fn` requises ;
- les structs utilisées dans l'interface publique ;
- les enums utilisées dans l'interface publique ;
- la taille et l'alignement des types publics ;
- les diagnostics de compilation.

Ces métadonnées permettent à l'hôte de vérifier un module avant de l'exécuter.

## 23. Conditions

```txt
if x > 0 {
    print(c"positive")
} else {
    print(c"negative")
}
```

La condition doit être un `bool`.

### 23.1 `if` comme expression

```txt
let value = if condition { 10 } else { 20 }
```

Les deux branches doivent retourner le même type.

## 24. Boucles

### 24.1 `while`

```txt
let mut i = 0
while i < 10 {
    i += 1
}
```

### 24.2 `for` avec range

```txt
for i in 0..10 {
    print_i32(i)
}
```

### 24.3 `for` sur tableau

```txt
for value in &values {
    print_i32(*value)
}
```

### 24.4 `break` et `continue`

```txt
while true {
    break
}
```

## 25. Match

```txt
let score = match kind {
    EntityType::Player => 10,
    EntityType::Enemy => 5,
    EntityType::Npc => 1,
}
```

Le `match` sur enum doit être exhaustif.

## 26. Opérateurs

### 26.1 Arithmétique

```txt
+ - * / %
```

Les deux opérandes doivent avoir le même type.

### 26.2 Comparaison

```txt
== != < <= > >=
```

### 26.3 Logique

```txt
&& || !
```

### 26.4 Assignation

```txt
= += -= *= /= %=
```

La cible doit être mutable.

## 27. Casts

Il n'y a aucune conversion implicite.

Syntaxe :

```txt
(Type)value
```

Exemples :

```txt
let a: i32 = 10
let b = (f32)a
```

Autorisés :

- entier vers entier ;
- entier vers flottant ;
- flottant vers entier ;
- flottant vers flottant ;
- enum vers son discriminant.

Interdits :

- cast arbitraire entre structs ;
- cast arbitraire entre références incompatibles ;
- cast caché entre `str` et `cstr`.

## 28. Scope

Un bloc crée un scope.

```txt
{
    let x = 10
}

// x inaccessible ici
```

Le shadowing est autorisé.

```txt
let x = 10
let x = 20
```

## 29. Statements et expressions

- `let` est un statement.
- `return` est un statement.
- `while` est un statement.
- `for` est un statement.
- `if` peut être une expression.
- `match` peut être une expression.
- un appel de fonction est une expression.
- une assignation est un statement.

Les points-virgules sont optionnels.

```txt
let x = 10;
let y = 20
```

Les deux sont valides.

## 30. Erreurs runtime

Le langage ne possède pas d'exceptions en V1.

Les erreurs runtime sont fatales.

Exemples :

- index hors limites ;
- division entière par zéro ;
- fonction externe manquante ;
- signature externe incompatible ;
- pointeur nul reçu via une référence ;
- échec d'allocation.

Les erreurs récupérables doivent passer par des codes de retour explicites.

Exemple :

```txt
enum Status : i32 {
    Ok = 0,
    Error = 1,
}
```

## 31. Built-ins V1

### 31.1 Affichage

```txt
print(s: cstr)
println(s: cstr)
print_i32(v: i32)
println_i32(v: i32)
print_f32(v: f32)
println_f32(v: f32)
print_bool(v: bool)
println_bool(v: bool)
```

### 31.2 Math

```txt
sqrt_f32(x: f32) -> f32
sqrt_f64(x: f64) -> f64
sin_f32(x: f32) -> f32
cos_f32(x: f32) -> f32
abs_i32(x: i32) -> i32
abs_f32(x: f32) -> f32
min_i32(a: i32, b: i32) -> i32
max_i32(a: i32, b: i32) -> i32
min_f32(a: f32, b: f32) -> f32
max_f32(a: f32, b: f32) -> f32
```

### 31.3 Clone

```txt
clone(value: T) -> T
```

`clone` est un intrinsèque du typechecker.

Il ne rend pas les generics disponibles dans le langage utilisateur.

## 32. Fonctionnalités interdites en V1

Pour garder le langage simple, ces fonctionnalités sont interdites :

- classes ;
- héritage ;
- interfaces ;
- traits ;
- generics utilisateur ;
- macros ;
- surcharge de fonction ;
- surcharge d'opérateur ;
- closures ;
- fonctions imbriquées ;
- exceptions ;
- variables globales mutables ;
- pointeurs nus exposés directement ;
- conversions implicites ;
- GC obligatoire.

## 33. Exemple complet

```txt
struct Position {
    x: f32,
    y: f32,

    fn from_xy(x: f32, y: f32) -> Position {
        Position { x: x, y: y }
    }

    fn move_by(&mut self, dx: f32, dy: f32) {
        self.x += dx
        self.y += dy
    }
}

struct Velocity {
    x: f32,
    y: f32,
}

enum EntityType : i32 {
    Player = 0,
    Enemy = 1,
}

type Entity = (Position, Velocity)

extern fn draw_position(pos: &Position)

export fn update_position(pos: &mut Position, vel: &Velocity, dt: f32) {
    pos.x += vel.x * dt
    pos.y += vel.y * dt
}

fn create_entity() -> Entity {
    (
        Position::from_xy(0.0, 0.0),
        Velocity { x: 1.0, y: 1.0 },
    )
}

fn main() {
    let (mut pos, vel) = create_entity()

    update_position(&mut pos, &vel, 0.016)
    draw_position(&pos)
}
```
