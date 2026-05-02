# Flodata — Fluent OData

A type-safe, composable OData v4 query builder for Rust.

```rust
use flodata::ODataQuery;
use flodata::field::Field;
use flodata::filter::{ODataFilter, FilterCompose};
use flodata::expand::{expand, Order};

const PRICE: Field<f64> = Field::new("price");
const NAME: Field<String> = Field::new("name");

let url = ODataQuery::new()
    .base_url("https://api.example.com/odata")
    .collection("Products")
    .filter(PRICE.gt(10.0).and(NAME.contains("bike")))
    .select_fields(&[&NAME, &PRICE])
    .expand(expand("Supplier").select_str(&["Name", "Country"]))
    .order_by_field(&PRICE, Order::Desc)
    .top(20)
    .count(true)
    .to_url()
    .unwrap();
```

## Design principles

- **Typed-first, not typed-only** — typed fields are the default, untyped `col()` and `RawFilter` are the escape hatches
- **Everything is a filter** — built-in expressions, custom domain filters, and raw strings all implement `ODataFilter` and compose the same way
- **Expression tree, not string concatenation** — filters build an AST that serializes at the end, enabling inspection and transformation
- **Transport-agnostic** — outputs a `String`; bring your own HTTP client
- **OData v4** — no v2/v3 baggage

## Usage

### 1. Define typed fields

Group fields under a module or struct impl. The type parameter controls which operations are available.

```rust
use flodata::field::Field;

mod product {
    use super::*;

    pub const ID: Field<i32>       = Field::new("id");
    pub const NAME: Field<String>  = Field::new("name");
    pub const PRICE: Field<f64>    = Field::new("price");
    pub const ACTIVE: Field<bool>  = Field::new("isActive");
    pub const RATING: Field<i32>   = Field::new("rating");
}
```

Type constraints are enforced at compile time:

```rust
product::PRICE.gt(100.0);          // ✅ f64 compared to f64
product::NAME.contains("widget");  // ✅ contains() on String field
// product::PRICE.contains("x");   // ❌ won't compile — contains() is String-only
// product::ACTIVE.gt(true);       // ❌ won't compile — gt() requires ODataOrd
```

### 2. Build filters

Filters are created from field methods and composed with `.and()` / `.or()` / `.not()`:

```rust
use flodata::filter::{FilterCompose, ODataFilter};

// Simple comparison
let f = product::PRICE.gt(100.0);

// Composed
let f = product::PRICE.gt(10.0)
    .and(product::PRICE.lt(100.0))
    .and(product::NAME.contains("bike"));

// Negation
let f = product::ACTIVE.eq(false).not();
```

### 3. Custom domain filters

Any type that implements `ODataFilter` plugs into the system. Define reusable business logic as types:

```rust
use flodata::filter::{ODataFilter, FilterExpr, FilterCompose};

struct ActiveProducts;

impl ODataFilter for ActiveProducts {
    fn into_filter_expr(self) -> FilterExpr {
        product::ACTIVE.eq(true).into_filter_expr()
    }
}

struct PricedBetween(f64, f64);

impl ODataFilter for PricedBetween {
    fn into_filter_expr(self) -> FilterExpr {
        product::PRICE.ge(self.0)
            .and(product::PRICE.le(self.1))
            .into_filter_expr()
    }
}

// Use them like any other filter — they compose freely
let f = ActiveProducts
    .and(PricedBetween(10.0, 99.99))
    .or(product::RATING.ge(5));
```

### 4. Build queries

```rust
use flodata::ODataQuery;
use flodata::expand::{expand, Order};

let query = ODataQuery::new()
    .base_url("https://api.example.com/odata")
    .collection("Products")
    .filter(ActiveProducts)
    .filter(PricedBetween(10.0, 99.99))   // ANDed with previous
    .select_fields(&[&product::NAME, &product::PRICE])
    .order_by_field(&product::PRICE, Order::Desc)
    .top(20)
    .skip(40)
    .count(true);

// Query string only
let qs = query.to_query_string();
// → "$filter=(isActive eq true) and ((price ge 10.0) and (price le 99.99))&$select=name,price&$orderby=price desc&$top=20&$skip=40&$count=true"

// Full URL
let url = query.to_url().unwrap();
// → "https://api.example.com/odata/Products?$filter=..."
```

### 5. Expand (with nested queries)

Expands support their own `$select`, `$filter`, `$orderby`, `$top`, `$skip`, and even nested `$expand`:

```rust
let query = ODataQuery::new()
    .collection("Products")
    .expand(
        expand("Orders")
            .select_str(&["Id", "Amount", "Date"])
            .filter(col("Amount").gt(50))
            .order_by("Date", Order::Desc)
            .top(10)
    )
    .expand(
        expand("Supplier").select_str(&["Name", "Country"])
    );

// → "$expand=Orders($select=Id,Amount,Date;$filter=Amount gt 50;$orderby=Date desc;$top=10),Supplier($select=Name,Country)"
```

Deeply nested:

```rust
let query = ODataQuery::new()
    .expand(
        expand("Orders")
            .expand(expand("Items").select_str(&["ProductName", "Qty"]))
            .select_str(&["Id"])
    );
// → "$expand=Orders($select=Id;$expand=Items($select=ProductName,Qty))"
```

Reusable expand patterns via `IntoExpand`:

```rust
use flodata::expand::{IntoExpand, ExpandClause};

struct ProductWithTopOrders;

impl IntoExpand for ProductWithTopOrders {
    fn into_expand(self) -> ExpandClause {
        expand("Orders")
            .select_str(&["Id", "Amount"])
            .order_by("Amount", Order::Desc)
            .top(5)
    }
}

let query = ODataQuery::new().expand(ProductWithTopOrders);
```

### 6. Nested fields (paths)

OData often uses slash-separated paths. Define them as you would any field:

```rust
mod copernicus {
    use flodata::field::Field;

    pub const CONTENT_DATE_START: Field<String> = Field::new("ContentDate/Start");
    pub const CONTENT_DATE_END: Field<String>   = Field::new("ContentDate/End");
    pub const NAME: Field<String>                = Field::new("Name");
    pub const ONLINE: Field<bool>                = Field::new("Online");
}
```

### 7. DateTime and special values

OData has specific literal formats that don't map directly to Rust primitives. Use the `ODataValue` constructors:

```rust
use flodata::value::ODataValue;

// DateTime (bare ISO8601, trailing Z appended automatically)
let f = copernicus::CONTENT_DATE_START
    .gt_value(ODataValue::datetime("2019-05-15T00:00:00.000"));
// → "ContentDate/Start gt 2019-05-15T00:00:00.000Z"

// GUID
let f = col("ProductId").eq_value(ODataValue::guid("01234567-89ab-cdef-0123-456789abcdef"));

// Null
let f = product::NAME.is_null();
// → "name eq null"

// In-list
let f = product::RATING.in_list(vec![
    ODataValue::Int(4),
    ODataValue::Int(5),
]);
// → "rating in (4, 5)"
```

### 8. Untyped fields (quick prototyping)

When you don't want to define field types, use `col()`:

```rust
use flodata::field::col;

let f = col("price").gt(100).and(col("name").contains("bike"));
```

All operations are available on untyped fields — no compile-time type checking, but full functionality.

### 9. Raw filter (escape hatch)

When the OData API uses features the DSL doesn't cover, pass a raw string:

```rust
use flodata::filter::RawFilter;

let f = RawFilter::new(
    "Attributes/OData.CSC.StringAttribute/any(att:att/Name eq 'productType')"
);

// Composes with everything else
let f = product::ACTIVE.eq(true).and(RawFilter::new(
    "geo.distance(Location, geography'POINT(0 0)') lt 100"
));
```

### 10. Real-world example: Copernicus Data Space

```rust
use flodata::ODataQuery;
use flodata::value::ODataValue;
use flodata::filter::{FilterCompose, ODataFilter, RawFilter};

let query = ODataQuery::new()
    .base_url("https://catalogue.dataspace.copernicus.eu/odata/v1")
    .collection("Products")
    .filter(
        copernicus::NAME.contains("S2A")
            .and(copernicus::CONTENT_DATE_START
                .gt_value(ODataValue::datetime("2019-05-15T00:00:00.000")))
            .and(copernicus::CONTENT_DATE_END
                .lt_value(ODataValue::datetime("2019-06-15T00:00:00.000")))
            .and(copernicus::ONLINE.eq(true))
            .and(RawFilter::new(
                "Attributes/OData.CSC.StringAttribute/any(att:att/Name eq 'productType' and att/OData.CSC.StringAttribute/Value eq 'S2MSI1C')"
            ))
    )
    .top(10);

let url = query.to_url().unwrap();
```

## Module structure

| Module | Purpose |
|--------|---------|
| `flodata::field` | `Field<T>`, `col()`, `FieldRef` trait |
| `flodata::filter` | `FilterExpr`, `ODataFilter` trait, `FilterCompose`, `RawFilter` |
| `flodata::value` | `ODataValue`, `IntoODataValue`, marker traits |
| `flodata::expand` | `ExpandClause`, `IntoExpand`, `Order` |
| `flodata::builder` | `ODataQuery` |

## When to use what

| Situation | Approach |
|-----------|----------|
| You control the domain model | Typed `Field<T>` constants |
| Repeated business logic | Custom `ODataFilter` impl |
| Quick one-off query | `col("fieldName")` |
| API quirk or unsupported syntax | `RawFilter::new("...")` |
| Reusable expand patterns | Custom `IntoExpand` impl |
| DateTime / GUID / special values | `ODataValue::datetime()`, `ODataValue::guid()`, etc. |

## License

MIT
