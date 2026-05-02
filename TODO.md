# TODO

Findings from the library code review. Grouped by priority.

## P0 — correctness bugs that will hit real users

- [ ] **URL-encode query values in `to_url()`** — `src/builder.rs:224-244`
  Currently raw concatenation. Spaces, `&`, `#`, `+`, single quotes, non-ASCII bytes break the URL. A `RawFilter` containing `&` silently truncates the query. Percent-encode the value side of each `key=value` pair; keep `to_query_string()` raw if desired and document the contract.

- [ ] **Tighten `eq`/`ne`/`in_list` (and `gt`/`ge`/`lt`/`le`) type bounds** — `src/field.rs:72-195`
  `Field::<i32>.eq("hello")` and `Field::<f64>.gt("x")` currently compile. The `T: IntoODataValue` bound on the impl block is decorative — `T` is never used in the body. Introduce a relation trait `ODataComparableWith<T>` and bind value type to field type. Otherwise drop the "compile-time type safety" claim from `README.md`.

- [ ] **Fix float formatting for non-finite values** — `src/value.rs:73-80`
  `f64::NAN.to_string()` returns `"NaN"`; the `.0` fallback then emits `NaN.0`. Same for `inf.0` / `-inf.0`. OData v4 wants `NaN`, `INF`, `-INF`. Special-case before the generic decimal-point path.

## P1 — correctness bugs, lower frequency

- [ ] **Empty `in_list` produces server-rejected `field in ()`** — `src/filter.rs:147-150`, `src/field.rs:149-154`
  Short-circuit empty input to a literal-false expression, or panic. Most likely user intent is "match nothing."

- [ ] **`ODataValue::datetime` mangles offset-bearing input** — `src/value.rs:32-39`
  `"…+02:00"` becomes `"…+02:00Z"`. `"hello"` becomes `"helloZ"`. Detect existing `Z` / `±HH:MM` and skip the append, or split into `datetime_utc(naive)` and `datetime_offset(s)`.

- [ ] **`$search` value emitted bare, no quoting** — `src/builder.rs:206-208`
  Multi-word phrases in OData v4 must be double-quoted. `.search("blue green")` is parsed by servers as `blue AND green`. Add `search_phrase` helper and/or document the contract.

## P2 — API smells / footguns

- [ ] **Inconsistent multi-call semantics across builder methods** — `src/builder.rs:69-122`
  `filter()` AND-folds, `select()` replaces, `expand()` appends, `order_by()` appends. Pick one convention or rename the replacing variants (`set_select` vs `add_select`). `ExpandClause::select_str` has the same issue.

- [ ] **Reconsider `String: ODataOrd`** — `src/value.rs:148`
  Lexicographic ordering is OData-legal, but combined with the type-safety hole it lets `name.gt(42)` compile. Resolved by fixing the eq/ne/gt bounds; otherwise gate behind a feature flag or document.

- [ ] **Skip allocation in `.replace('\'', "''")` when no quote present** — `src/value.rs:59`, `src/filter.rs:143`
  Gate behind `if s.contains('\'')`. Micro-opt; only matters in hot loops.

- [ ] **Recursive `to_filter_string` / `to_expand_string` can stack-overflow** — `src/filter.rs:111-154`, `src/expand.rs`
  `fold(seed, .and)` over ~30k items overflows the default 8 MB stack. Convert to iterative walk, or document the depth ceiling.

- [ ] **`Field::new("")` and `collection("")` silently emit broken queries** — `src/field.rs:42`, `src/builder.rs:55`
  Add `debug_assert!(!name.is_empty())` on non-`const` paths. The const `Field::new` can't validate at runtime; rely on docs.

- [ ] **Document multi-call AND semantics on filter methods** — `src/builder.rs:69`, `src/expand.rs:85`
  Behavior is fine, just undocumented.

- [ ] **Consider sealing `FilterCompose`** — `src/filter.rs:227`
  Future additions to the trait become breaking changes for anyone who has named a method the same. Low priority; pre-1.0 hygiene.

## P3 — documentation drift

- [ ] **Promote README expected-output comments to assertions** — `README.md:133, 158, 170, 217, 231`
  The `// → "..."` lines aren't tested anywhere. Move at least the headline ones into doctests so they can't silently drift.
