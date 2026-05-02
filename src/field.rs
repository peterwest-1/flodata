use std::marker::PhantomData;

use crate::filter::{CompareOp, FilterExpr, ODataFilter, StringFunction};
use crate::value::{IntoODataValue, ODataOrd, ODataStringLike, ODataValue};

// ── FieldRef (type-erased) ─────────────────────────────────────────

/// A type-erased reference to any field, used for `select` / `orderby`
/// where the value type doesn't matter.
pub trait FieldRef {
    fn field_name(&self) -> &str;
}

// ── Field<T> ───────────────────────────────────────────────────────

/// A typed OData field.
///
/// The type parameter `T` determines which operations are available:
/// - All fields: `eq`, `ne`, `is_null`, `is_not_null`, `in_list`
/// - `Field<String>`: `contains`, `starts_with`, `ends_with`
/// - Numeric / orderable fields: `gt`, `ge`, `lt`, `le`
///
/// ```
/// use flodata::field::Field;
/// use flodata::filter::ODataFilter;
///
/// const PRICE: Field<f64> = Field::new("price");
/// const NAME: Field<String> = Field::new("name");
///
/// let f = PRICE.gt(100.0);
/// assert_eq!(f.into_filter_expr().to_filter_string(), "price gt 100.0");
/// ```
#[derive(Debug)]
pub struct Field<T> {
    name: &'static str,
    _phantom: PhantomData<T>,
}

impl<T> Field<T> {
    /// Create a new typed field. Use OData path syntax for nested
    /// fields, e.g. `"ContentDate/Start"`.
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _phantom: PhantomData,
        }
    }
}

impl<T> Clone for Field<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Field<T> {}

impl<T> FieldRef for Field<T> {
    fn field_name(&self) -> &str {
        self.name
    }
}

// Allow &Field<T> as FieldRef too (for slices of references).
impl<T> FieldRef for &Field<T> {
    fn field_name(&self) -> &str {
        self.name
    }
}

// ── Universal operations (all field types) ─────────────────────────

impl<T: IntoODataValue> Field<T> {
    /// Equality: `field eq value`
    pub fn eq(self, value: impl IntoODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.name.into(),
            op: CompareOp::Eq,
            value: value.into_odata_value(),
        })
    }

    /// Inequality: `field ne value`
    pub fn ne(self, value: impl IntoODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.name.into(),
            op: CompareOp::Ne,
            value: value.into_odata_value(),
        })
    }
}

impl<T> Field<T> {
    /// Null check: `field eq null`
    pub fn is_null(self) -> FieldFilter {
        FieldFilter(FilterExpr::IsNull {
            field: self.name.into(),
        })
    }

    /// Not-null check: `field ne null`
    pub fn is_not_null(self) -> FieldFilter {
        FieldFilter(FilterExpr::IsNotNull {
            field: self.name.into(),
        })
    }

    /// Equality with a pre-built `ODataValue`.
    ///
    /// Useful for datetime/guid values:
    /// ```
    /// use flodata::field::Field;
    /// use flodata::value::ODataValue;
    /// use flodata::filter::ODataFilter;
    ///
    /// const START: Field<String> = Field::new("ContentDate/Start");
    /// let f = START.eq_value(ODataValue::datetime("2024-01-01T00:00:00.000"));
    /// assert_eq!(
    ///     f.into_filter_expr().to_filter_string(),
    ///     "ContentDate/Start eq 2024-01-01T00:00:00.000Z"
    /// );
    /// ```
    pub fn eq_value(self, value: ODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.name.into(),
            op: CompareOp::Eq,
            value,
        })
    }

    /// Greater-than with a pre-built `ODataValue`.
    pub fn gt_value(self, value: ODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.name.into(),
            op: CompareOp::Gt,
            value,
        })
    }

    /// Less-than with a pre-built `ODataValue`.
    pub fn lt_value(self, value: ODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.name.into(),
            op: CompareOp::Lt,
            value,
        })
    }

    /// `field in (v1, v2, v3)`
    pub fn in_list(self, values: Vec<ODataValue>) -> FieldFilter {
        FieldFilter(FilterExpr::In {
            field: self.name.into(),
            values,
        })
    }
}

// ── Ordering operations (numeric + string) ─────────────────────────

impl<T: IntoODataValue + ODataOrd> Field<T> {
    /// Greater than: `field gt value`
    pub fn gt(self, value: impl IntoODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.name.into(),
            op: CompareOp::Gt,
            value: value.into_odata_value(),
        })
    }

    /// Greater or equal: `field ge value`
    pub fn ge(self, value: impl IntoODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.name.into(),
            op: CompareOp::Ge,
            value: value.into_odata_value(),
        })
    }

    /// Less than: `field lt value`
    pub fn lt(self, value: impl IntoODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.name.into(),
            op: CompareOp::Lt,
            value: value.into_odata_value(),
        })
    }

    /// Less or equal: `field le value`
    pub fn le(self, value: impl IntoODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.name.into(),
            op: CompareOp::Le,
            value: value.into_odata_value(),
        })
    }
}

// ── String-only operations ─────────────────────────────────────────

impl<T: ODataStringLike> Field<T> {
    /// `contains(field, 'value')`
    pub fn contains(self, value: &str) -> FieldFilter {
        FieldFilter(FilterExpr::StringFunc {
            func: StringFunction::Contains,
            field: self.name.into(),
            value: value.into(),
        })
    }

    /// `startswith(field, 'value')`
    pub fn starts_with(self, value: &str) -> FieldFilter {
        FieldFilter(FilterExpr::StringFunc {
            func: StringFunction::StartsWith,
            field: self.name.into(),
            value: value.into(),
        })
    }

    /// `endswith(field, 'value')`
    pub fn ends_with(self, value: &str) -> FieldFilter {
        FieldFilter(FilterExpr::StringFunc {
            func: StringFunction::EndsWith,
            field: self.name.into(),
            value: value.into(),
        })
    }
}

// ── FieldFilter wrapper ────────────────────────────────────────────

/// Wrapper returned by field methods. Implements `ODataFilter` so it
/// composes with `.and()` / `.or()` / `.not()` via `FilterCompose`.
#[derive(Debug, Clone)]
pub struct FieldFilter(pub FilterExpr);

impl ODataFilter for FieldFilter {
    fn into_filter_expr(self) -> FilterExpr {
        self.0
    }
}

// ── Untyped field helper ───────────────────────────────────────────

/// Create an untyped field for quick prototyping or fully dynamic queries.
///
/// All comparison methods are available regardless of type.
///
/// ```
/// use flodata::field::col;
/// use flodata::filter::ODataFilter;
///
/// let f = col("name").eq("Widget");
/// assert_eq!(f.into_filter_expr().to_filter_string(), "name eq 'Widget'");
/// ```
pub fn col(name: &'static str) -> UntypedField {
    UntypedField(name)
}

/// An untyped field — all operations are available but not type-checked.
#[derive(Debug, Clone, Copy)]
pub struct UntypedField(&'static str);

impl FieldRef for UntypedField {
    fn field_name(&self) -> &str {
        self.0
    }
}

impl UntypedField {
    pub fn eq(self, value: impl IntoODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.0.into(),
            op: CompareOp::Eq,
            value: value.into_odata_value(),
        })
    }

    pub fn ne(self, value: impl IntoODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.0.into(),
            op: CompareOp::Ne,
            value: value.into_odata_value(),
        })
    }

    pub fn gt(self, value: impl IntoODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.0.into(),
            op: CompareOp::Gt,
            value: value.into_odata_value(),
        })
    }

    pub fn ge(self, value: impl IntoODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.0.into(),
            op: CompareOp::Ge,
            value: value.into_odata_value(),
        })
    }

    pub fn lt(self, value: impl IntoODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.0.into(),
            op: CompareOp::Lt,
            value: value.into_odata_value(),
        })
    }

    pub fn le(self, value: impl IntoODataValue) -> FieldFilter {
        FieldFilter(FilterExpr::Compare {
            field: self.0.into(),
            op: CompareOp::Le,
            value: value.into_odata_value(),
        })
    }

    pub fn contains(self, value: &str) -> FieldFilter {
        FieldFilter(FilterExpr::StringFunc {
            func: StringFunction::Contains,
            field: self.0.into(),
            value: value.into(),
        })
    }

    pub fn starts_with(self, value: &str) -> FieldFilter {
        FieldFilter(FilterExpr::StringFunc {
            func: StringFunction::StartsWith,
            field: self.0.into(),
            value: value.into(),
        })
    }

    pub fn ends_with(self, value: &str) -> FieldFilter {
        FieldFilter(FilterExpr::StringFunc {
            func: StringFunction::EndsWith,
            field: self.0.into(),
            value: value.into(),
        })
    }

    pub fn is_null(self) -> FieldFilter {
        FieldFilter(FilterExpr::IsNull {
            field: self.0.into(),
        })
    }

    pub fn is_not_null(self) -> FieldFilter {
        FieldFilter(FilterExpr::IsNotNull {
            field: self.0.into(),
        })
    }

    pub fn in_list(self, values: Vec<ODataValue>) -> FieldFilter {
        FieldFilter(FilterExpr::In {
            field: self.0.into(),
            values,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{FilterCompose, ODataFilter};

    const PRICE: Field<f64> = Field::new("price");
    const NAME: Field<String> = Field::new("name");
    const ACTIVE: Field<bool> = Field::new("isActive");

    #[test]
    fn typed_eq() {
        let f = PRICE.eq(42.0);
        assert_eq!(f.into_filter_expr().to_filter_string(), "price eq 42.0");
    }

    #[test]
    fn typed_gt() {
        let f = PRICE.gt(100.0);
        assert_eq!(f.into_filter_expr().to_filter_string(), "price gt 100.0");
    }

    #[test]
    fn string_contains() {
        let f = NAME.contains("bike");
        assert_eq!(
            f.into_filter_expr().to_filter_string(),
            "contains(name, 'bike')"
        );
    }

    #[test]
    fn string_starts_with() {
        let f = NAME.starts_with("Pro");
        assert_eq!(
            f.into_filter_expr().to_filter_string(),
            "startswith(name, 'Pro')"
        );
    }

    #[test]
    fn bool_eq() {
        let f = ACTIVE.eq(true);
        assert_eq!(
            f.into_filter_expr().to_filter_string(),
            "isActive eq true"
        );
    }

    #[test]
    fn is_null() {
        let f = NAME.is_null();
        assert_eq!(f.into_filter_expr().to_filter_string(), "name eq null");
    }

    #[test]
    fn compose_typed_fields() {
        let f = PRICE.gt(10.0).and(PRICE.lt(100.0));
        assert_eq!(
            f.to_filter_string(),
            "(price gt 10.0) and (price lt 100.0)"
        );
    }

    #[test]
    fn untyped_col_works() {
        let f = col("status").eq("Active");
        assert_eq!(
            f.into_filter_expr().to_filter_string(),
            "status eq 'Active'"
        );
    }

    #[test]
    fn nested_field_path() {
        const START: Field<String> = Field::new("ContentDate/Start");
        let f = START.gt_value(ODataValue::datetime("2024-01-01T00:00:00.000"));
        assert_eq!(
            f.into_filter_expr().to_filter_string(),
            "ContentDate/Start gt 2024-01-01T00:00:00.000Z"
        );
    }
}
