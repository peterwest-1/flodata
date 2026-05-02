//! # Flodata — Fluent OData
//!
//! A type-safe, composable OData v4 query builder for Rust.
//!
//! ```
//! use flodata::ODataQuery;
//! use flodata::field::Field;
//! use flodata::filter::{ODataFilter, FilterCompose};
//! use flodata::expand::{expand, Order};
//!
//! // Define typed fields
//! const PRICE: Field<f64> = Field::new("price");
//! const NAME: Field<String> = Field::new("name");
//!
//! let query = ODataQuery::new()
//!     .base_url("https://api.example.com/odata")
//!     .collection("Products")
//!     .filter(PRICE.gt(10.0).and(NAME.contains("bike")))
//!     .select_fields(&[&NAME, &PRICE])
//!     .expand(expand("Supplier").select_str(&["Name", "Country"]))
//!     .order_by_field(&PRICE, Order::Desc)
//!     .top(20)
//!     .count(true);
//!
//! println!("{}", query.to_query_string());
//! ```

pub mod builder;
pub mod expand;
pub mod field;
pub mod filter;
pub mod value;

// Re-export the main entry point at crate root.
pub use builder::ODataQuery;
