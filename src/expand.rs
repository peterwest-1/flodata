use crate::field::FieldRef;
use crate::filter::{FilterExpr, ODataFilter};

// ── OrderBy (used in expand and top-level) ─────────────────────────

/// Sort direction.
#[derive(Debug, Clone, PartialEq)]
pub enum Order {
    Asc,
    Desc,
}

/// A single order-by clause.
#[derive(Debug, Clone)]
pub struct OrderByClause {
    pub field: String,
    pub direction: Order,
}

impl OrderByClause {
    pub fn to_string_fragment(&self) -> String {
        match self.direction {
            Order::Asc => self.field.clone(),
            Order::Desc => format!("{} desc", self.field),
        }
    }
}

// ── Expand clause ──────────────────────────────────────────────────

/// An OData `$expand` clause, optionally with nested query options.
///
/// ```
/// use flodata::expand::ExpandClause;
/// use flodata::field::{Field, col};
/// use flodata::filter::{ODataFilter, FilterCompose};
///
/// let expand = ExpandClause::new("Orders")
///     .select_str(&["Id", "Amount"])
///     .filter(col("Amount").gt(100));
///
/// assert_eq!(
///     expand.to_expand_string(),
///     "Orders($select=Id,Amount;$filter=Amount gt 100)"
/// );
/// ```
#[derive(Debug, Clone)]
pub struct ExpandClause {
    pub navigation: String,
    pub select: Vec<String>,
    pub filter: Option<FilterExpr>,
    pub order_by: Vec<OrderByClause>,
    pub top: Option<usize>,
    pub skip: Option<usize>,
    pub nested_expand: Vec<ExpandClause>,
}

impl ExpandClause {
    /// Create an expand for a navigation property.
    pub fn new(navigation: impl Into<String>) -> Self {
        Self {
            navigation: navigation.into(),
            select: Vec::new(),
            filter: None,
            order_by: Vec::new(),
            top: None,
            skip: None,
            nested_expand: Vec::new(),
        }
    }

    /// Add a `$select` inside this expand (string-based).
    pub fn select_str(mut self, fields: &[&str]) -> Self {
        self.select = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add a `$select` inside this expand (typed fields).
    pub fn select_fields(mut self, fields: &[&dyn FieldRef]) -> Self {
        self.select = fields.iter().map(|f| f.field_name().to_string()).collect();
        self
    }

    /// Add a `$filter` inside this expand.
    pub fn filter(mut self, filter: impl ODataFilter) -> Self {
        let new_expr = filter.into_filter_expr();
        self.filter = Some(match self.filter.take() {
            Some(existing) => FilterExpr::Logical {
                left: Box::new(existing),
                op: crate::filter::LogicalOp::And,
                right: Box::new(new_expr),
            },
            None => new_expr,
        });
        self
    }

    /// Add an `$orderby` inside this expand (string-based).
    pub fn order_by(mut self, field: &str, direction: Order) -> Self {
        self.order_by.push(OrderByClause {
            field: field.to_string(),
            direction,
        });
        self
    }

    /// Add an `$orderby` inside this expand (typed field).
    pub fn order_by_field(mut self, field: &dyn FieldRef, direction: Order) -> Self {
        self.order_by.push(OrderByClause {
            field: field.field_name().to_string(),
            direction,
        });
        self
    }

    /// Set `$top` inside this expand.
    pub fn top(mut self, n: usize) -> Self {
        self.top = Some(n);
        self
    }

    /// Set `$skip` inside this expand.
    pub fn skip(mut self, n: usize) -> Self {
        self.skip = Some(n);
        self
    }

    /// Add a nested `$expand` inside this expand.
    pub fn expand(mut self, nested: ExpandClause) -> Self {
        self.nested_expand.push(nested);
        self
    }

    /// Serialize to the OData expand string.
    pub fn to_expand_string(&self) -> String {
        let mut options: Vec<String> = Vec::new();

        if !self.select.is_empty() {
            options.push(format!("$select={}", self.select.join(",")));
        }

        if let Some(ref filter) = self.filter {
            options.push(format!("$filter={}", filter.to_filter_string()));
        }

        if !self.order_by.is_empty() {
            let parts: Vec<String> = self.order_by.iter().map(|o| o.to_string_fragment()).collect();
            options.push(format!("$orderby={}", parts.join(",")));
        }

        if let Some(top) = self.top {
            options.push(format!("$top={}", top));
        }

        if let Some(skip) = self.skip {
            options.push(format!("$skip={}", skip));
        }

        if !self.nested_expand.is_empty() {
            let parts: Vec<String> = self
                .nested_expand
                .iter()
                .map(|e| e.to_expand_string())
                .collect();
            options.push(format!("$expand={}", parts.join(",")));
        }

        if options.is_empty() {
            self.navigation.clone()
        } else {
            format!("{}({})", self.navigation, options.join(";"))
        }
    }
}

/// Convenience: create an expand clause.
pub fn expand(navigation: &str) -> ExpandClause {
    ExpandClause::new(navigation)
}

// ── IntoExpand trait ───────────────────────────────────────────────

/// Trait for types that can be converted into an expand clause.
/// Lets users define reusable expand patterns.
pub trait IntoExpand {
    fn into_expand(self) -> ExpandClause;
}

impl IntoExpand for ExpandClause {
    fn into_expand(self) -> ExpandClause {
        self
    }
}

impl IntoExpand for &str {
    fn into_expand(self) -> ExpandClause {
        ExpandClause::new(self)
    }
}

impl IntoExpand for String {
    fn into_expand(self) -> ExpandClause {
        ExpandClause::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::col;

    #[test]
    fn simple_expand() {
        let e = expand("Supplier");
        assert_eq!(e.to_expand_string(), "Supplier");
    }

    #[test]
    fn expand_with_select() {
        let e = expand("Supplier").select_str(&["Name", "Country"]);
        assert_eq!(e.to_expand_string(), "Supplier($select=Name,Country)");
    }

    #[test]
    fn expand_with_filter() {
        let e = expand("Orders").filter(col("Amount").gt(100));
        assert_eq!(
            e.to_expand_string(),
            "Orders($filter=Amount gt 100)"
        );
    }

    #[test]
    fn expand_with_multiple_options() {
        let e = expand("Orders")
            .select_str(&["Id", "Amount"])
            .filter(col("Amount").gt(100))
            .order_by("Amount", Order::Desc)
            .top(5);
        assert_eq!(
            e.to_expand_string(),
            "Orders($select=Id,Amount;$filter=Amount gt 100;$orderby=Amount desc;$top=5)"
        );
    }

    #[test]
    fn nested_expand() {
        let inner = expand("Items").select_str(&["Name"]);
        let outer = expand("Orders").expand(inner);
        assert_eq!(
            outer.to_expand_string(),
            "Orders($expand=Items($select=Name))"
        );
    }
}
