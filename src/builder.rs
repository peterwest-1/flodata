use crate::expand::{ExpandClause, IntoExpand, Order, OrderByClause};
use crate::field::FieldRef;
use crate::filter::{FilterExpr, LogicalOp, ODataFilter};

/// The main OData query builder.
///
/// Build queries fluently, then call `.to_query_string()` to serialize.
///
/// ```
/// use flodata::ODataQuery;
/// use flodata::field::col;
/// use flodata::filter::{ODataFilter, FilterCompose};
///
/// let qs = ODataQuery::new()
///     .collection("Products")
///     .filter(col("price").gt(100))
///     .select(&["id", "name", "price"])
///     .top(10)
///     .to_query_string();
///
/// assert!(qs.contains("$filter=price gt 100"));
/// assert!(qs.contains("$select=id,name,price"));
/// assert!(qs.contains("$top=10"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ODataQuery {
    base_url: Option<String>,
    collection: Option<String>,
    key: Option<String>,
    filter: Option<FilterExpr>,
    select: Vec<String>,
    order_by: Vec<OrderByClause>,
    expand: Vec<ExpandClause>,
    top: Option<usize>,
    skip: Option<usize>,
    count: Option<bool>,
    search: Option<String>,
    format: Option<String>,
    custom: Vec<(String, String)>,
}

impl ODataQuery {
    /// Create a new empty query builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base URL (e.g. `"https://api.example.com/odata"`).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the entity collection (e.g. `"Products"`).
    pub fn collection(mut self, name: impl Into<String>) -> Self {
        self.collection = Some(name.into());
        self
    }

    /// Set the entity key for single-entity queries (e.g. `"(42)"`).
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    // ── Filter ─────────────────────────────────────────────────────

    /// Add a filter. Multiple calls are ANDed together.
    pub fn filter(mut self, filter: impl ODataFilter) -> Self {
        let new_expr = filter.into_filter_expr();
        self.filter = Some(match self.filter.take() {
            Some(existing) => FilterExpr::Logical {
                left: Box::new(existing),
                op: LogicalOp::And,
                right: Box::new(new_expr),
            },
            None => new_expr,
        });
        self
    }

    // ── Select ─────────────────────────────────────────────────────

    /// Select fields by string name.
    pub fn select(mut self, fields: &[&str]) -> Self {
        self.select = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Select fields using typed `Field<T>` references.
    pub fn select_fields(mut self, fields: &[&dyn FieldRef]) -> Self {
        self.select = fields.iter().map(|f| f.field_name().to_string()).collect();
        self
    }

    // ── OrderBy ────────────────────────────────────────────────────

    /// Add an order-by clause by string name.
    pub fn order_by(mut self, field: &str, direction: Order) -> Self {
        self.order_by.push(OrderByClause {
            field: field.to_string(),
            direction,
        });
        self
    }

    /// Add an order-by clause using a typed field.
    pub fn order_by_field(mut self, field: &dyn FieldRef, direction: Order) -> Self {
        self.order_by.push(OrderByClause {
            field: field.field_name().to_string(),
            direction,
        });
        self
    }

    // ── Expand ─────────────────────────────────────────────────────

    /// Add an expand clause.
    pub fn expand(mut self, expand: impl IntoExpand) -> Self {
        self.expand.push(expand.into_expand());
        self
    }

    // ── Paging ─────────────────────────────────────────────────────

    /// Set `$top`.
    pub fn top(mut self, n: usize) -> Self {
        self.top = Some(n);
        self
    }

    /// Set `$skip`.
    pub fn skip(mut self, n: usize) -> Self {
        self.skip = Some(n);
        self
    }

    // ── Count ──────────────────────────────────────────────────────

    /// Set `$count=true` or `$count=false`.
    pub fn count(mut self, enabled: bool) -> Self {
        self.count = Some(enabled);
        self
    }

    // ── Search ─────────────────────────────────────────────────────

    /// Set `$search`.
    pub fn search(mut self, term: impl Into<String>) -> Self {
        self.search = Some(term.into());
        self
    }

    // ── Format ─────────────────────────────────────────────────────

    /// Set `$format` (e.g. `"json"`, `"xml"`).
    pub fn format(mut self, fmt: impl Into<String>) -> Self {
        self.format = Some(fmt.into());
        self
    }

    // ── Custom parameters ──────────────────────────────────────────

    /// Add a custom query parameter (not prefixed with `$`).
    pub fn custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.push((key.into(), value.into()));
        self
    }

    // ── Serialization ──────────────────────────────────────────────

    /// Build the query string (the `?...` part, without `?`).
    pub fn to_query_string(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(ref filter) = self.filter {
            parts.push(format!("$filter={}", filter.to_filter_string()));
        }

        if !self.select.is_empty() {
            parts.push(format!("$select={}", self.select.join(",")));
        }

        if !self.order_by.is_empty() {
            let frags: Vec<String> = self.order_by.iter().map(|o| o.to_string_fragment()).collect();
            parts.push(format!("$orderby={}", frags.join(",")));
        }

        if !self.expand.is_empty() {
            let frags: Vec<String> = self.expand.iter().map(|e| e.to_expand_string()).collect();
            parts.push(format!("$expand={}", frags.join(",")));
        }

        if let Some(top) = self.top {
            parts.push(format!("$top={}", top));
        }

        if let Some(skip) = self.skip {
            parts.push(format!("$skip={}", skip));
        }

        if let Some(count) = self.count {
            parts.push(format!("$count={}", count));
        }

        if let Some(ref search) = self.search {
            parts.push(format!("$search={}", search));
        }

        if let Some(ref format) = self.format {
            parts.push(format!("$format={}", format));
        }

        for (key, value) in &self.custom {
            parts.push(format!("{}={}", key, value));
        }

        parts.join("&")
    }

    /// Build the full URL (base + collection + key + query string).
    ///
    /// Returns `None` if no base URL is set.
    pub fn to_url(&self) -> Option<String> {
        let base = self.base_url.as_ref()?;
        let mut url = base.trim_end_matches('/').to_string();

        if let Some(ref coll) = self.collection {
            url.push('/');
            url.push_str(coll);
        }

        if let Some(ref key) = self.key {
            url.push_str(key);
        }

        let qs = self.to_query_string();
        if !qs.is_empty() {
            url.push('?');
            url.push_str(&qs);
        }

        Some(url)
    }
}

impl std::fmt::Display for ODataQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_url() {
            Some(url) => write!(f, "{}", url),
            None => write!(f, "{}", self.to_query_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expand::{expand, Order};
    use crate::field::{col, Field};
    use crate::filter::{FilterCompose, RawFilter};

    #[test]
    fn empty_query() {
        let qs = ODataQuery::new().to_query_string();
        assert_eq!(qs, "");
    }

    #[test]
    fn filter_only() {
        let qs = ODataQuery::new()
            .filter(col("price").gt(100))
            .to_query_string();
        assert_eq!(qs, "$filter=price gt 100");
    }

    #[test]
    fn multiple_filters_and() {
        let qs = ODataQuery::new()
            .filter(col("price").gt(100))
            .filter(col("active").eq(true))
            .to_query_string();
        assert_eq!(
            qs,
            "$filter=(price gt 100) and (active eq true)"
        );
    }

    #[test]
    fn select_and_top() {
        let qs = ODataQuery::new()
            .select(&["id", "name"])
            .top(5)
            .to_query_string();
        assert_eq!(qs, "$select=id,name&$top=5");
    }

    #[test]
    fn order_by_multiple() {
        let qs = ODataQuery::new()
            .order_by("name", Order::Asc)
            .order_by("price", Order::Desc)
            .to_query_string();
        assert_eq!(qs, "$orderby=name,price desc");
    }

    #[test]
    fn expand_simple() {
        let qs = ODataQuery::new()
            .expand("Supplier")
            .to_query_string();
        assert_eq!(qs, "$expand=Supplier");
    }

    #[test]
    fn expand_with_options() {
        let qs = ODataQuery::new()
            .expand(
                expand("Orders")
                    .select_str(&["Id", "Amount"])
                    .top(5),
            )
            .to_query_string();
        assert_eq!(qs, "$expand=Orders($select=Id,Amount;$top=5)");
    }

    #[test]
    fn full_url() {
        let url = ODataQuery::new()
            .base_url("https://api.example.com/odata")
            .collection("Products")
            .filter(col("price").gt(100))
            .top(10)
            .to_url()
            .unwrap();
        assert_eq!(
            url,
            "https://api.example.com/odata/Products?$filter=price gt 100&$top=10"
        );
    }

    #[test]
    fn full_url_with_key() {
        let url = ODataQuery::new()
            .base_url("https://api.example.com/odata")
            .collection("Products")
            .key("(42)")
            .to_url()
            .unwrap();
        assert_eq!(url, "https://api.example.com/odata/Products(42)");
    }

    #[test]
    fn count_and_search() {
        let qs = ODataQuery::new()
            .count(true)
            .search("bike")
            .to_query_string();
        assert_eq!(qs, "$count=true&$search=bike");
    }

    #[test]
    fn custom_parameter() {
        let qs = ODataQuery::new()
            .custom("apiVersion", "2.0")
            .to_query_string();
        assert_eq!(qs, "apiVersion=2.0");
    }

    #[test]
    fn raw_filter_in_query() {
        let qs = ODataQuery::new()
            .filter(RawFilter::new("Attributes/OData.CSC.StringAttribute/any(att:att/Name eq 'productType')"))
            .to_query_string();
        assert_eq!(
            qs,
            "$filter=Attributes/OData.CSC.StringAttribute/any(att:att/Name eq 'productType')"
        );
    }

    #[test]
    fn typed_fields_in_query() {
        const PRICE: Field<f64> = Field::new("price");
        const NAME: Field<String> = Field::new("name");

        let qs = ODataQuery::new()
            .filter(PRICE.gt(100.0).and(NAME.contains("bike")))
            .select_fields(&[&NAME, &PRICE])
            .order_by_field(&PRICE, Order::Desc)
            .top(5)
            .to_query_string();

        assert_eq!(
            qs,
            "$filter=(price gt 100.0) and (contains(name, 'bike'))&$select=name,price&$orderby=price desc&$top=5"
        );
    }
}
