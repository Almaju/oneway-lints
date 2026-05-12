# `oneway::no_loop`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

Don't use `loop`, `while`, or `for` with manual iteration. Use iterators and functional combinators instead.

## Rationale

Imperative loops mix *what* (the transformation) with *how* (mutable accumulators, index arithmetic, early-exit flags). Combinator pipelines (`.iter().filter().map().sum()`) state the transformation as a sequence of named steps, make the intermediate types visible at each stage, and compose naturally with other pipelines. They also remove a class of bugs — off-by-one indexing, forgotten increments, mutation captured by the wrong reference — that loops keep alive.

## ❌ Bad

```rust
let mut total = 0;
for item in &items {
    if item.is_active() {
        total += item.price();
    }
}
```

## ✅ Good

```rust
let total: u64 = items
    .iter()
    .filter(|item| item.is_active())
    .map(|item| item.price())
    .sum();
```

## ❌ Bad

```rust
let mut result = Vec::new();
let mut i = 0;
while i < items.len() {
    result.push(items[i].transform());
    i += 1;
}
```

## ✅ Good

```rust
let result: Vec<_> = items.iter().map(|item| item.transform()).collect();
```
