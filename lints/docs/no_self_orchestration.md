# `oneway::no_self_orchestration`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

In an inherent `impl Foo` block, a public method must not call another public method on `self`. Private helper methods on `self` remain free to call — they're internal organization, not API composition.

## Rationale

Stores, repositories, clients, and service-shaped types tend to start small and grow over time. Each new feature feels like it deserves "just one more method" on the same type:

```rust
impl JudgeStore {
    pub fn list_dynamic(&self) -> ...      { ... }
    pub fn put_active_dynamic(&self, ...)  { ... }
    pub fn rebuild_judges(&self, ...) -> ... {   // ← composed from list_dynamic + ...
        let db = self.list_dynamic()?;
        let (merged, report) = merge(yaml, &db);
        ...
    }
}
```

Each method "looks reasonable" in isolation, but the type slowly accumulates into a god-object that holds half the application's behavior. The signal you can catch *early* is the moment a public method composes the type's own public API: `rebuild_judges` calls `self.list_dynamic()`. That's workflow logic — it belongs in a dedicated use-case struct that depends on the store, not on the store itself.

Private helpers (`fn load_memories(&self)`, `fn insert_memory(&self, ...)`) are the explicit escape hatch — they're how you share SQL across pub methods without exposing the composition surface.

## ❌ Bad — public method composing public API

```rust
impl MemoryStore {
    pub async fn recall(&self, query: &str) -> Vec<Memory> { ... }
    pub async fn messages(&self) -> Vec<StoredMessage> { ... }

    pub async fn assemble_context(&self, msg: &str, budget: TokenCount) -> AssembledContext {
        let recalled = self.recall(msg, 10).await?;        // ← self.<pub method>
        let stored = self.messages().await?;                // ← self.<pub method>
        // ... fitting / packing logic
    }
}
```

## ✅ Good — extract the workflow to a use-case struct

```rust
pub struct AssembleContext<'a> {
    pub budget: TokenCount,
    pub new_user_message: &'a str,
    pub store: &'a MemoryStore,
}

impl AssembleContext<'_> {
    pub async fn run(self) -> AssembledContext {
        let recalled = self.store.recall(self.new_user_message, 10).await?;
        let stored = self.store.messages().await?;
        // ... fitting / packing logic
    }
}
```

## ✅ Good — pub method using a private helper

```rust
impl MemoryStore {
    pub async fn recall(&self, query: &str) -> Vec<Memory> {
        let embedding = self.embed(query).await?;
        let memories = self.load_memories().await?;   // ← private helper, OK
        // ... ranking
    }

    async fn load_memories(&self) -> Vec<Memory> { ... }   // private
    async fn embed(&self, text: &str) -> Vec<f32> { ... }  // private
}
```

## What this rule does NOT do

- It does not fire on `Self::other()` path calls — those are constructor delegations, not method composition.
- It does not fire on `self.field.method()` — calling a method on a field (e.g. `self.pool.execute(...)`) is using a dependency, not composing your own API.
- It does not fire on trait `impl Trait for Foo` blocks — trait methods have signatures fixed by the trait and can legitimately delegate to inherent methods.
- It does not fire on `pub` methods calling private (`fn`, no `pub`) methods on `self`.
