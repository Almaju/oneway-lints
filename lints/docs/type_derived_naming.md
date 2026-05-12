# `oneway::type_derived_naming`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

The implementation enforces this for function parameters and `let` bindings with an explicit type ascription. For inferred types, the rule is not enforced today — add `: Type` to opt the binding in.

**Exemptions:** primitives (`i32`, `bool`, `String`, …) and a handful of stdlib containers whose idiomatic short names are too entrenched to flag (`Option`, `Result`, `Vec`, `Box`, `Rc`, `Arc`, `HashMap`, `Path`, …). Use whatever name you like for those.

Every binding's name must be the `snake_case` version of its type. This applies to both `let` bindings and function parameters — wherever you give a value a name, that name should echo the type.

## Rationale

At every use site, the reader can map the variable back to its type without scrolling up to the declaration — `user_id.something()` is unambiguous in a way that `id.something()` is not. It also kills bikeshedding ("`id`, `uid`, `user_id`, `userid`?" — only one answer) and removes a whole category of stylistic review feedback. When two bindings of the same type need to coexist, add a descriptive prefix (`sender_account_id` / `receiver_account_id`) — the rule bends only where the type can't disambiguate on its own.

## ❌ Bad — short, type-unrelated names

```rust
let id = UserId(42);
let db = Database::connect();
let u = User::find(id);
```

## ✅ Good

```rust
let user_id = UserId(42);
let database = Database::connect();
let user = User::find(user_id);
```

## ❌ Bad — function parameter doesn't echo its type

```rust
fn find_user(id: UserId, db: &Database) -> Option<User> {
    db.query(id)
}
```

## ✅ Good

```rust
fn find_user(user_id: UserId, database: &Database) -> Option<User> {
    database.query(user_id)
}
```

## ❌ Bad — two of the same type without disambiguation

```rust
let src = AccountId(1);
let dst = AccountId(2);
```

## ✅ Good

```rust
let sender_account_id = AccountId(1);
let receiver_account_id = AccountId(2);
```

## Generics

For generic-typed parameters, the rule resolves the generic's bounds:

- **No effective bounds** (filtering `Sized`, `?Sized`, `Send`, `Sync`, `Unpin`): no constraint — the generic carries no semantic info.
- **One bound** (`<M: Migrator>`): binding must match the trait — `migrator: M`. The generic's spelling can stay short (`M`, `T`).
- **Two or more bounds** (`<M: Migrator + Connector>`): the generic itself must be given a descriptive identifier reflecting its role (`Service`, `Worker`, …), and the binding must match it: `<Service: Migrator + Connector>(service: Service)`.

### ❌ Bad — single-letter binding for a bounded generic

```rust
fn run<M: Migrator>(m: M) {
    m.migrate()
}
```

### ✅ Good

```rust
fn run<M: Migrator>(migrator: M) {
    migrator.migrate()
}
```

### ❌ Bad — placeholder generic name with multiple bounds

```rust
fn deploy<M: Migrator + Connector + Orchestrator>(m: M) {
    m.migrate()?;
    m.connect()?;
    m.orchestrate()
}
```

### ✅ Good — name the generic by its role

```rust
fn deploy<Service: Migrator + Connector + Orchestrator>(service: Service) {
    service.migrate()?;
    service.connect()?;
    service.orchestrate()
}
```
