/// OData primitive value representation.
///
/// Handles the OData-specific literal formatting rules:
/// - Strings are single-quoted: `'hello'`
/// - Datetimes are bare ISO8601: `2024-01-01T00:00:00.000Z`
/// - GUIDs are bare: `01234567-89ab-cdef-0123-456789abcdef`
/// - Booleans are lowercase: `true` / `false`
/// - Null is `null`
#[derive(Debug, Clone, PartialEq)]
pub enum ODataValue {
    String(String),
    Int(i64),
    Float(f64),
    Boolean(bool),
    DateTime(String),
    Date(String),
    TimeOfDay(String),
    Guid(String),
    Null,
}

impl ODataValue {
    /// Create a datetime literal. Input should be ISO8601.
    /// A trailing `Z` is appended if not present.
    ///
    /// ```
    /// use flodata::value::ODataValue;
    ///
    /// let v = ODataValue::datetime("2024-01-15T09:30:00.000");
    /// assert_eq!(v.to_odata_string(), "2024-01-15T09:30:00.000Z");
    /// ```
    pub fn datetime(s: &str) -> Self {
        let s = if s.ends_with('Z') || s.ends_with('z') {
            s.to_string()
        } else {
            format!("{s}Z")
        };
        ODataValue::DateTime(s)
    }

    /// Create a date literal (no time component).
    pub fn date(s: &str) -> Self {
        ODataValue::Date(s.to_string())
    }

    /// Create a time-of-day literal.
    pub fn time_of_day(s: &str) -> Self {
        ODataValue::TimeOfDay(s.to_string())
    }

    /// Create a GUID literal.
    pub fn guid(s: &str) -> Self {
        ODataValue::Guid(s.to_string())
    }

    /// Serialize this value to its OData literal representation.
    pub fn to_odata_string(&self) -> String {
        match self {
            ODataValue::String(s) => format!("'{}'", s.replace('\'', "''")),
            ODataValue::Int(n) => n.to_string(),
            ODataValue::Float(n) => format_float(*n),
            ODataValue::Boolean(b) => b.to_string(),
            ODataValue::DateTime(s) => s.clone(),
            ODataValue::Date(s) => s.clone(),
            ODataValue::TimeOfDay(s) => s.clone(),
            ODataValue::Guid(s) => s.clone(),
            ODataValue::Null => "null".to_string(),
        }
    }
}

/// Format a float, ensuring it always has a decimal point.
fn format_float(n: f64) -> String {
    let s = n.to_string();
    if s.contains('.') {
        s
    } else {
        format!("{s}.0")
    }
}

// ── Conversion traits ──────────────────────────────────────────────

/// Types that can be converted into an OData literal value.
pub trait IntoODataValue {
    fn into_odata_value(self) -> ODataValue;
}

impl IntoODataValue for &str {
    fn into_odata_value(self) -> ODataValue {
        ODataValue::String(self.to_string())
    }
}

impl IntoODataValue for String {
    fn into_odata_value(self) -> ODataValue {
        ODataValue::String(self)
    }
}

impl IntoODataValue for i32 {
    fn into_odata_value(self) -> ODataValue {
        ODataValue::Int(self as i64)
    }
}

impl IntoODataValue for i64 {
    fn into_odata_value(self) -> ODataValue {
        ODataValue::Int(self)
    }
}

impl IntoODataValue for f32 {
    fn into_odata_value(self) -> ODataValue {
        ODataValue::Float(self as f64)
    }
}

impl IntoODataValue for f64 {
    fn into_odata_value(self) -> ODataValue {
        ODataValue::Float(self)
    }
}

impl IntoODataValue for bool {
    fn into_odata_value(self) -> ODataValue {
        ODataValue::Boolean(self)
    }
}

impl IntoODataValue for ODataValue {
    fn into_odata_value(self) -> ODataValue {
        self
    }
}

// ── Marker traits for field type constraints ────────────────────────

/// Marker: types that support ordering operators (gt, lt, gte, lte).
pub trait ODataOrd {}

impl ODataOrd for f32 {}
impl ODataOrd for f64 {}
impl ODataOrd for i32 {}
impl ODataOrd for i64 {}

// String is Ord in OData (lexicographic)
impl ODataOrd for String {}

/// Marker: string-like types that support contains/startswith/endswith.
pub trait ODataStringLike {}

impl ODataStringLike for String {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_value_single_quotes() {
        let v = "hello".into_odata_value();
        assert_eq!(v.to_odata_string(), "'hello'");
    }

    #[test]
    fn string_value_escapes_quotes() {
        let v = "it's".into_odata_value();
        assert_eq!(v.to_odata_string(), "'it''s'");
    }

    #[test]
    fn datetime_appends_z() {
        let v = ODataValue::datetime("2024-01-15T00:00:00.000");
        assert_eq!(v.to_odata_string(), "2024-01-15T00:00:00.000Z");
    }

    #[test]
    fn datetime_preserves_z() {
        let v = ODataValue::datetime("2024-01-15T00:00:00.000Z");
        assert_eq!(v.to_odata_string(), "2024-01-15T00:00:00.000Z");
    }

    #[test]
    fn float_always_has_decimal() {
        let v = (100.0_f64).into_odata_value();
        assert!(v.to_odata_string().contains('.'));
    }

    #[test]
    fn bool_lowercase() {
        assert_eq!(true.into_odata_value().to_odata_string(), "true");
        assert_eq!(false.into_odata_value().to_odata_string(), "false");
    }
}
