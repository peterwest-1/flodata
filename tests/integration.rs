use flodata::expand::{expand, Order};
use flodata::field::col;
use flodata::filter::{FilterCompose, FilterExpr, ODataFilter, RawFilter};
use flodata::value::ODataValue;
use flodata::ODataQuery;

// ── Entity field definitions ───────────────────────────────────────

mod product {
    use flodata::field::Field;

    pub const ID: Field<i32> = Field::new("id");
    pub const NAME: Field<String> = Field::new("name");
    pub const PRICE: Field<f64> = Field::new("price");
    pub const RATING: Field<i32> = Field::new("rating");
    pub const IS_ACTIVE: Field<bool> = Field::new("isActive");
    pub const CATEGORY: Field<String> = Field::new("category");
}

mod copernicus {
    use flodata::field::Field;

    pub const NAME: Field<String> = Field::new("Name");
    pub const CONTENT_DATE_START: Field<String> = Field::new("ContentDate/Start");
    pub const CONTENT_DATE_END: Field<String> = Field::new("ContentDate/End");
    pub const ONLINE: Field<bool> = Field::new("Online");
}

// ── Custom domain filters ──────────────────────────────────────────

struct ActiveProductFilter;

impl ODataFilter for ActiveProductFilter {
    fn into_filter_expr(self) -> FilterExpr {
        product::IS_ACTIVE.eq(true).into_filter_expr()
    }
}

struct PricedBetween(f64, f64);

impl ODataFilter for PricedBetween {
    fn into_filter_expr(self) -> FilterExpr {
        product::PRICE
            .ge(self.0)
            .and(product::PRICE.le(self.1))
            .into_filter_expr()
    }
}

struct HighRated(i32);

impl ODataFilter for HighRated {
    fn into_filter_expr(self) -> FilterExpr {
        product::RATING.ge(self.0).into_filter_expr()
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[test]
fn simple_typed_query() {
    let qs = ODataQuery::new()
        .filter(product::PRICE.gt(100.0))
        .select_fields(&[&product::ID, &product::NAME, &product::PRICE])
        .top(10)
        .to_query_string();

    assert_eq!(
        qs,
        "$filter=price gt 100.0&$select=id,name,price&$top=10"
    );
}

#[test]
fn custom_filter_composable() {
    let qs = ODataQuery::new()
        .filter(ActiveProductFilter)
        .filter(PricedBetween(10.0, 99.99))
        .to_query_string();

    assert_eq!(
        qs,
        "$filter=(isActive eq true) and ((price ge 10.0) and (price le 99.99))"
    );
}

#[test]
fn compose_custom_with_typed_fields() {
    let f = ActiveProductFilter
        .and(PricedBetween(10.0, 50.0))
        .or(product::CATEGORY.eq("Sale"));

    let qs = ODataQuery::new().filter(f).to_query_string();

    assert_eq!(
        qs,
        "$filter=((isActive eq true) and ((price ge 10.0) and (price le 50.0))) or (category eq 'Sale')"
    );
}

#[test]
fn negation() {
    let f = ActiveProductFilter.not();
    let qs = ODataQuery::new().filter(f).to_query_string();

    assert_eq!(qs, "$filter=not (isActive eq true)");
}

#[test]
fn three_custom_filters_composed() {
    let f = ActiveProductFilter
        .and(PricedBetween(10.0, 99.99))
        .and(HighRated(4));
    let qs = ODataQuery::new().filter(f).to_query_string();

    assert_eq!(
        qs,
        "$filter=((isActive eq true) and ((price ge 10.0) and (price le 99.99))) and (rating ge 4)"
    );
}

#[test]
fn untyped_col_alongside_typed_fields() {
    let qs = ODataQuery::new()
        .filter(product::PRICE.gt(50.0).and(col("legacyField").eq("yes")))
        .to_query_string();

    assert_eq!(
        qs,
        "$filter=(price gt 50.0) and (legacyField eq 'yes')"
    );
}

#[test]
fn string_operations() {
    let qs = ODataQuery::new()
        .filter(
            product::NAME
                .contains("bike")
                .and(product::CATEGORY.starts_with("Sport")),
        )
        .to_query_string();

    assert_eq!(
        qs,
        "$filter=(contains(name, 'bike')) and (startswith(category, 'Sport'))"
    );
}

#[test]
fn null_checks() {
    let qs = ODataQuery::new()
        .filter(product::CATEGORY.is_not_null())
        .to_query_string();

    assert_eq!(qs, "$filter=category ne null");
}

#[test]
fn in_list() {
    let qs = ODataQuery::new()
        .filter(product::CATEGORY.in_list(vec![
            ODataValue::String("Electronics".into()),
            ODataValue::String("Books".into()),
            ODataValue::String("Sports".into()),
        ]))
        .to_query_string();

    assert_eq!(
        qs,
        "$filter=category in ('Electronics', 'Books', 'Sports')"
    );
}

#[test]
fn datetime_filter() {
    let qs = ODataQuery::new()
        .filter(
            copernicus::CONTENT_DATE_START
                .gt_value(ODataValue::datetime("2019-05-15T00:00:00.000")),
        )
        .to_query_string();

    assert_eq!(
        qs,
        "$filter=ContentDate/Start gt 2019-05-15T00:00:00.000Z"
    );
}

#[test]
fn nested_expand_with_filter_and_select() {
    let qs = ODataQuery::new()
        .collection("Products")
        .expand(
            expand("Orders")
                .select_str(&["Id", "Amount", "Date"])
                .filter(col("Amount").gt(50))
                .order_by("Date", Order::Desc)
                .top(10),
        )
        .to_query_string();

    assert_eq!(
        qs,
        "$expand=Orders($select=Id,Amount,Date;$filter=Amount gt 50;$orderby=Date desc;$top=10)"
    );
}

#[test]
fn deeply_nested_expand() {
    let qs = ODataQuery::new()
        .expand(
            expand("Orders")
                .expand(expand("Items").select_str(&["ProductName", "Qty"]))
                .select_str(&["Id"]),
        )
        .to_query_string();

    assert_eq!(
        qs,
        "$expand=Orders($select=Id;$expand=Items($select=ProductName,Qty))"
    );
}

#[test]
fn multiple_expands() {
    let qs = ODataQuery::new()
        .expand("Supplier")
        .expand(expand("Category").select_str(&["Name"]))
        .to_query_string();

    assert_eq!(
        qs,
        "$expand=Supplier,Category($select=Name)"
    );
}

#[test]
fn full_url_generation() {
    let url = ODataQuery::new()
        .base_url("https://api.example.com/odata")
        .collection("Products")
        .filter(product::IS_ACTIVE.eq(true))
        .select_fields(&[&product::NAME, &product::PRICE])
        .order_by_field(&product::PRICE, Order::Desc)
        .top(20)
        .skip(40)
        .count(true)
        .to_url()
        .unwrap();

    assert_eq!(
        url,
        "https://api.example.com/odata/Products?$filter=isActive eq true&$select=name,price&$orderby=price desc&$top=20&$skip=40&$count=true"
    );
}

#[test]
fn entity_key_url() {
    let url = ODataQuery::new()
        .base_url("https://api.example.com/odata")
        .collection("Products")
        .key("(42)")
        .expand("Supplier")
        .to_url()
        .unwrap();

    assert_eq!(
        url,
        "https://api.example.com/odata/Products(42)?$expand=Supplier"
    );
}

#[test]
fn raw_filter_escape_hatch() {
    let qs = ODataQuery::new()
        .filter(RawFilter::new(
            "Attributes/OData.CSC.StringAttribute/any(att:att/Name eq 'productType')",
        ))
        .to_query_string();

    assert_eq!(
        qs,
        "$filter=Attributes/OData.CSC.StringAttribute/any(att:att/Name eq 'productType')"
    );
}

#[test]
fn raw_filter_composed_with_typed() {
    let qs = ODataQuery::new()
        .filter(
            product::IS_ACTIVE.eq(true).and(RawFilter::new(
                "geo.distance(Location, geography'POINT(0 0)') lt 100",
            )),
        )
        .to_query_string();

    assert_eq!(
        qs,
        "$filter=(isActive eq true) and (geo.distance(Location, geography'POINT(0 0)') lt 100)"
    );
}

#[test]
fn copernicus_real_world_query() {
    let qs = ODataQuery::new()
        .filter(
            copernicus::NAME
                .contains("S2A")
                .and(
                    copernicus::CONTENT_DATE_START
                        .gt_value(ODataValue::datetime("2019-05-15T00:00:00.000")),
                )
                .and(
                    copernicus::CONTENT_DATE_END
                        .lt_value(ODataValue::datetime("2019-06-15T00:00:00.000")),
                )
                .and(copernicus::ONLINE.eq(true)),
        )
        .top(10)
        .to_query_string();

    assert!(qs.contains("contains(Name, 'S2A')"));
    assert!(qs.contains("ContentDate/Start gt 2019-05-15T00:00:00.000Z"));
    assert!(qs.contains("ContentDate/End lt 2019-06-15T00:00:00.000Z"));
    assert!(qs.contains("Online eq true"));
    assert!(qs.contains("$top=10"));
}

#[test]
fn search_and_format() {
    let qs = ODataQuery::new()
        .search("blue OR green")
        .format("json")
        .to_query_string();

    assert_eq!(qs, "$search=blue OR green&$format=json");
}

#[test]
fn custom_parameters() {
    let qs = ODataQuery::new()
        .filter(product::IS_ACTIVE.eq(true))
        .custom("apiVersion", "2.0")
        .custom("tenant", "acme")
        .to_query_string();

    assert_eq!(
        qs,
        "$filter=isActive eq true&apiVersion=2.0&tenant=acme"
    );
}

#[test]
fn display_trait() {
    let query = ODataQuery::new()
        .base_url("https://api.example.com/odata")
        .collection("Products")
        .top(5);

    assert_eq!(
        format!("{}", query),
        "https://api.example.com/odata/Products?$top=5"
    );
}

#[test]
fn display_trait_no_url() {
    let query = ODataQuery::new().top(5);
    assert_eq!(format!("{}", query), "$top=5");
}

// ── Custom IntoExpand example ──────────────────────────────────────

use flodata::expand::IntoExpand;

struct ProductWithTopOrders;

impl IntoExpand for ProductWithTopOrders {
    fn into_expand(self) -> flodata::expand::ExpandClause {
        expand("Orders")
            .select_str(&["Id", "Amount"])
            .order_by("Amount", Order::Desc)
            .top(5)
    }
}

#[test]
fn custom_expand_pattern() {
    let qs = ODataQuery::new()
        .expand(ProductWithTopOrders)
        .to_query_string();

    assert_eq!(
        qs,
        "$expand=Orders($select=Id,Amount;$orderby=Amount desc;$top=5)"
    );
}
