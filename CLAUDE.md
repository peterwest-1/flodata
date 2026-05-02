# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`flodata` (Fluent OData) is a single-crate Rust library — a type-safe, composable OData v4 query builder. Zero runtime dependencies (stdlib only). MSRV is implied by `edition = "2021"`. See `README.md` for the full user-facing API tour.

## Common commands

```sh
cargo build
cargo test                          # runs unit tests in src/* + tests/integration.rs + doctests
cargo test --test integration       # integration tests only
cargo test <name>                   # run a single test by substring (e.g. `cargo test datetime_appends_z`)
cargo test --doc                    # doctests only — README examples in lib.rs and module docs
cargo clippy --all-targets
cargo fmt
```

There are no other build scripts, no feature flags, no workspace, no CI config in-tree.

## Architecture

The crate is built around an **expression tree that defers serialization until `.to_query_string()` / `.to_url()`**. Everything composes through traits rather than string concatenation. The five modules are tightly coupled — a change to `FilterExpr` or `ODataValue` will ripple through all of them.

### Module roles and dependency direction

```
builder ──┬──> expand ──┐
          └──> field ───┼──> filter ──> value
                        └─────────────> value
```

- **`value`** (`src/value.rs`) — leaf layer. `ODataValue` enum + the OData literal-formatting rules (string quoting, `Z`-suffixing for datetime, float-must-have-decimal). Defines marker traits `ODataOrd` and `ODataStringLike` plus the `IntoODataValue` conversion trait. **Adding a new primitive requires touching this module + every place that pattern-matches on the enum.**
- **`filter`** (`src/filter.rs`) — the AST. `FilterExpr` enum is the single source of truth for what an OData `$filter` expression can be. `ODataFilter` is the trait *anything filter-shaped* must implement (one method: `into_filter_expr`). `FilterCompose` is **blanket-implemented** for every `T: ODataFilter`, which is what makes `.and()`, `.or()`, `.not()` available everywhere — do not add an explicit `impl FilterCompose for ...`. `RawFilter` is the escape hatch for OData syntax the AST doesn't model.
- **`field`** (`src/field.rs`) — the typed DSL surface. `Field<T>` uses `PhantomData<T>` for compile-time type checking; the type bounds (`IntoODataValue`, `ODataOrd`, `ODataStringLike`) gate which methods are visible. `UntypedField` (returned by `col()`) is the parallel untyped surface — it duplicates the method set deliberately because it must offer all operations regardless of type. `FieldRef` is the type-erased trait used for `select` / `orderby` where the value type is irrelevant.
- **`expand`** (`src/expand.rs`) — `$expand` clauses, which are recursive (`ExpandClause` contains `Vec<ExpandClause>`) and carry their own `$select`/`$filter`/`$orderby`/`$top`/`$skip`. `IntoExpand` mirrors `ODataFilter`'s pattern for letting users define reusable expand types. `OrderByClause` and `Order` live here because expands need them; the top-level builder reuses them.
- **`builder`** (`src/builder.rs`) — `ODataQuery`, the public entry point (re-exported at crate root in `lib.rs`). Holds an `Option<FilterExpr>`; **multiple `.filter()` calls are AND-folded into the existing tree**, not replaced.

### Two key extension points

When a user wants custom logic, the codebase pushes them toward implementing one of these traits rather than mutating the enums:

1. `impl ODataFilter for MyType` — domain filters (e.g. `ActiveProducts`) that compose with everything else via the blanket `FilterCompose`.
2. `impl IntoExpand for MyType` — reusable expand patterns.

Adding a new comparison/string operator means extending `FilterExpr` *and* the `Field<T>` impl blocks (and `UntypedField` to mirror). Adding a new value primitive means extending `ODataValue` *and* the `to_odata_string` match *and* `IntoODataValue` impls.

### Doctests are part of the test suite

Module-level and item-level rustdoc examples in `lib.rs`, `field.rs`, `filter.rs`, `value.rs`, `expand.rs`, `builder.rs` are executed by `cargo test`. Update them when you change a public signature or output format — a serialization tweak (e.g. how floats render) will silently break doctests if you only run unit tests.
