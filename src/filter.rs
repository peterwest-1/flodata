use crate::value::ODataValue;

// ── Expression tree ────────────────────────────────────────────────

/// Comparison operators.
#[derive(Debug, Clone, PartialEq)]
pub enum CompareOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

impl CompareOp {
    fn as_str(&self) -> &'static str {
        match self {
            CompareOp::Eq => "eq",
            CompareOp::Ne => "ne",
            CompareOp::Gt => "gt",
            CompareOp::Ge => "ge",
            CompareOp::Lt => "lt",
            CompareOp::Le => "le",
        }
    }
}

/// Logical operators.
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOp {
    And,
    Or,
}

impl LogicalOp {
    fn as_str(&self) -> &'static str {
        match self {
            LogicalOp::And => "and",
            LogicalOp::Or => "or",
        }
    }
}

/// A string function call (contains, startswith, endswith, etc).
#[derive(Debug, Clone, PartialEq)]
pub enum StringFunction {
    Contains,
    StartsWith,
    EndsWith,
}

impl StringFunction {
    fn as_str(&self) -> &'static str {
        match self {
            StringFunction::Contains => "contains",
            StringFunction::StartsWith => "startswith",
            StringFunction::EndsWith => "endswith",
        }
    }
}

/// The core expression tree for OData `$filter`.
///
/// Serialization is deferred until `.to_string()` is called, allowing
/// programmatic inspection, transformation, and composition.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterExpr {
    /// Field comparison: `price gt 100`
    Compare {
        field: String,
        op: CompareOp,
        value: ODataValue,
    },

    /// Logical combination: `(left) and (right)`
    Logical {
        left: Box<FilterExpr>,
        op: LogicalOp,
        right: Box<FilterExpr>,
    },

    /// Negation: `not (expr)`
    Not(Box<FilterExpr>),

    /// Null check: `field eq null`
    IsNull { field: String },

    /// Not-null check: `field ne null`
    IsNotNull { field: String },

    /// String function: `contains(name, 'bike')`
    StringFunc {
        func: StringFunction,
        field: String,
        value: String,
    },

    /// `field in ('a', 'b', 'c')`
    In {
        field: String,
        values: Vec<ODataValue>,
    },

    /// Raw OData expression — the escape hatch.
    Raw(String),
}

impl FilterExpr {
    /// Serialize this expression to an OData `$filter` string.
    pub fn to_filter_string(&self) -> String {
        match self {
            FilterExpr::Compare { field, op, value } => {
                format!("{} {} {}", field, op.as_str(), value.to_odata_string())
            }

            FilterExpr::Logical { left, op, right } => {
                format!(
                    "({}) {} ({})",
                    left.to_filter_string(),
                    op.as_str(),
                    right.to_filter_string()
                )
            }

            FilterExpr::Not(expr) => {
                format!("not ({})", expr.to_filter_string())
            }

            FilterExpr::IsNull { field } => {
                format!("{} eq null", field)
            }

            FilterExpr::IsNotNull { field } => {
                format!("{} ne null", field)
            }

            FilterExpr::StringFunc { func, field, value } => {
                format!(
                    "{}({}, '{}')",
                    func.as_str(),
                    field,
                    value.replace('\'', "''")
                )
            }

            FilterExpr::In { field, values } => {
                let vals: Vec<String> = values.iter().map(|v| v.to_odata_string()).collect();
                format!("{} in ({})", field, vals.join(", "))
            }

            FilterExpr::Raw(s) => s.clone(),
        }
    }
}

impl std::fmt::Display for FilterExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_filter_string())
    }
}

// ── ODataFilter trait ──────────────────────────────────────────────

/// The core filter trait. Implement this on any type to make it usable
/// with the query builder and composable with other filters.
///
/// ```
/// use flodata::filter::{ODataFilter, FilterExpr};
///
/// struct ActiveStatusFilter;
///
/// impl ODataFilter for ActiveStatusFilter {
///     fn into_filter_expr(self) -> FilterExpr {
///         FilterExpr::Compare {
///             field: "isActive".into(),
///             op: flodata::filter::CompareOp::Eq,
///             value: flodata::value::ODataValue::Boolean(true),
///         }
///     }
/// }
/// ```
pub trait ODataFilter {
    /// Convert this filter into an expression tree node.
    fn into_filter_expr(self) -> FilterExpr;
}

// FilterExpr is trivially a filter.
impl ODataFilter for FilterExpr {
    fn into_filter_expr(self) -> FilterExpr {
        self
    }
}

// ── Composition ────────────────────────────────────────────────────

/// Extension trait providing `.and()` and `.or()` on any `ODataFilter`.
///
/// This is blanket-implemented, so every type that implements
/// `ODataFilter` gets these combinators for free.
pub trait FilterCompose: ODataFilter + Sized {
    /// Combine with another filter using AND.
    fn and<F: ODataFilter>(self, other: F) -> FilterExpr {
        FilterExpr::Logical {
            left: Box::new(self.into_filter_expr()),
            op: LogicalOp::And,
            right: Box::new(other.into_filter_expr()),
        }
    }

    /// Combine with another filter using OR.
    fn or<F: ODataFilter>(self, other: F) -> FilterExpr {
        FilterExpr::Logical {
            left: Box::new(self.into_filter_expr()),
            op: LogicalOp::Or,
            right: Box::new(other.into_filter_expr()),
        }
    }

    /// Negate this filter.
    fn not(self) -> FilterExpr {
        FilterExpr::Not(Box::new(self.into_filter_expr()))
    }
}

// Blanket implementation.
impl<T: ODataFilter> FilterCompose for T {}

// ── Raw filter (escape hatch) ──────────────────────────────────────

/// A raw OData filter string. Use this when the typed DSL doesn't
/// cover a specific API quirk.
///
/// ```
/// use flodata::filter::{RawFilter, ODataFilter};
///
/// let f = RawFilter::new("Attributes/OData.CSC.StringAttribute/any(att:att/Name eq 'productType')");
/// assert_eq!(
///     f.into_filter_expr().to_filter_string(),
///     "Attributes/OData.CSC.StringAttribute/any(att:att/Name eq 'productType')"
/// );
/// ```
pub struct RawFilter(String);

impl RawFilter {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl ODataFilter for RawFilter {
    fn into_filter_expr(self) -> FilterExpr {
        FilterExpr::Raw(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_serialization() {
        let expr = FilterExpr::Compare {
            field: "price".into(),
            op: CompareOp::Gt,
            value: ODataValue::Float(100.0),
        };
        assert_eq!(expr.to_filter_string(), "price gt 100.0");
    }

    #[test]
    fn logical_and() {
        let left = FilterExpr::Compare {
            field: "price".into(),
            op: CompareOp::Gt,
            value: ODataValue::Float(10.0),
        };
        let right = FilterExpr::Compare {
            field: "price".into(),
            op: CompareOp::Lt,
            value: ODataValue::Float(100.0),
        };
        let combined = FilterExpr::Logical {
            left: Box::new(left),
            op: LogicalOp::And,
            right: Box::new(right),
        };
        assert_eq!(
            combined.to_filter_string(),
            "(price gt 10.0) and (price lt 100.0)"
        );
    }

    #[test]
    fn not_filter() {
        let expr = FilterExpr::Compare {
            field: "isActive".into(),
            op: CompareOp::Eq,
            value: ODataValue::Boolean(true),
        };
        let negated = FilterExpr::Not(Box::new(expr));
        assert_eq!(negated.to_filter_string(), "not (isActive eq true)");
    }

    #[test]
    fn string_function() {
        let expr = FilterExpr::StringFunc {
            func: StringFunction::Contains,
            field: "name".into(),
            value: "bike".into(),
        };
        assert_eq!(expr.to_filter_string(), "contains(name, 'bike')");
    }

    #[test]
    fn in_operator() {
        let expr = FilterExpr::In {
            field: "status".into(),
            values: vec![
                ODataValue::String("Active".into()),
                ODataValue::String("Pending".into()),
            ],
        };
        assert_eq!(
            expr.to_filter_string(),
            "status in ('Active', 'Pending')"
        );
    }

    #[test]
    fn raw_filter_passthrough() {
        let f = RawFilter::new("custom/path eq 'value'");
        assert_eq!(f.into_filter_expr().to_filter_string(), "custom/path eq 'value'");
    }

    #[test]
    fn compose_trait_and() {
        let left = FilterExpr::Compare {
            field: "a".into(),
            op: CompareOp::Eq,
            value: ODataValue::Int(1),
        };
        let right = FilterExpr::Compare {
            field: "b".into(),
            op: CompareOp::Eq,
            value: ODataValue::Int(2),
        };
        let result = left.and(right);
        assert_eq!(result.to_filter_string(), "(a eq 1) and (b eq 2)");
    }

    #[test]
    fn compose_custom_filter() {
        struct ActiveFilter;
        impl ODataFilter for ActiveFilter {
            fn into_filter_expr(self) -> FilterExpr {
                FilterExpr::Compare {
                    field: "isActive".into(),
                    op: CompareOp::Eq,
                    value: ODataValue::Boolean(true),
                }
            }
        }

        let result = ActiveFilter.and(FilterExpr::Compare {
            field: "price".into(),
            op: CompareOp::Gt,
            value: ODataValue::Float(50.0),
        });
        assert_eq!(
            result.to_filter_string(),
            "(isActive eq true) and (price gt 50.0)"
        );
    }
}
