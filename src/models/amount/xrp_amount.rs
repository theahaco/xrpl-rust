use crate::models::{Model, XRPLModelException, XRPLModelResult};
use alloc::{
    borrow::Cow,
    string::{String, ToString},
};
use bigdecimal::BigDecimal;
use core::str::FromStr;
use core::{
    convert::{TryFrom, TryInto},
    fmt::Display,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The total XRP supply, in drops. Every valid drop amount is in `0..=MAX_DROPS`.
///
/// 100 billion XRP at 1,000,000 drops each. The ledger cannot represent more, so
/// anything above this is a malformed amount rather than an unaffordable one.
pub const MAX_DROPS: u64 = 100_000_000_000_000_000;

/// Represents an amount of XRP in Drops.
#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub struct XRPAmount<'a>(pub Cow<'a, str>);

impl<'a> Model for XRPAmount<'a> {
    fn get_errors(&self) -> XRPLModelResult<()> {
        // Drops are an unsigned 64-bit integer on the wire. Parsing them as `u32`
        // capped the library at 4,294.967295 XRP — roughly 0.000004% of the
        // representable range — and surfaced as a bare `ParseIntError` naming no
        // field.
        let drops: u64 = self
            .0
            .parse()
            .map_err(|_| XRPLModelException::InvalidValue {
                field: "XRPAmount".into(),
                expected: "an integer number of drops".into(),
                found: self.0.to_string(),
            })?;

        if drops > MAX_DROPS {
            return Err(XRPLModelException::InvalidValue {
                field: "XRPAmount".into(),
                expected: alloc::format!("at most {MAX_DROPS} drops"),
                found: self.0.to_string(),
            });
        }

        Ok(())
    }
}

impl Default for XRPAmount<'_> {
    fn default() -> Self {
        Self("0".into())
    }
}

impl Display for XRPAmount<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// implement Deserializing from Cow<str>, &str, String, Decimal, f64, u32, and Value
impl<'de, 'a> Deserialize<'de> for XRPAmount<'a> {
    fn deserialize<D>(deserializer: D) -> XRPLModelResult<XRPAmount<'a>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let amount_string = Value::deserialize(deserializer)?;
        XRPAmount::try_from(amount_string).map_err(serde::de::Error::custom)
    }
}

impl<'a> From<Cow<'a, str>> for XRPAmount<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        Self(value)
    }
}

impl<'a> From<&'a str> for XRPAmount<'a> {
    fn from(value: &'a str) -> Self {
        Self(value.into())
    }
}

impl<'a> From<String> for XRPAmount<'a> {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl<'a> From<BigDecimal> for XRPAmount<'a> {
    fn from(value: BigDecimal) -> Self {
        Self(value.to_string().into())
    }
}

impl<'a> From<f64> for XRPAmount<'a> {
    fn from(value: f64) -> Self {
        Self(value.to_string().into())
    }
}

impl<'a> From<u32> for XRPAmount<'a> {
    fn from(value: u32) -> Self {
        Self(value.to_string().into())
    }
}

impl<'a> From<u64> for XRPAmount<'a> {
    fn from(value: u64) -> Self {
        Self(value.to_string().into())
    }
}

impl<'a> TryFrom<Value> for XRPAmount<'a> {
    type Error = XRPLModelException;

    fn try_from(value: Value) -> XRPLModelResult<Self, Self::Error> {
        // Reject non-string and non-number JSON types (objects, arrays, null, booleans)
        if !value.is_string() && !value.is_number() {
            return Err(XRPLModelException::InvalidValue {
                field: "XRPAmount".into(),
                expected: "string or number".into(),
                found: match &value {
                    Value::Object(_) => "object".into(),
                    Value::Array(_) => "array".into(),
                    Value::Null => "null".into(),
                    Value::Bool(_) => "boolean".into(),
                    _ => "unknown".into(),
                },
            });
        }

        match serde_json::to_string(&value) {
            Ok(amount_string) => {
                let amount_string = amount_string.clone().replace("\"", "");
                Ok(Self(amount_string.into()))
            }
            Err(serde_error) => Err(serde_error.into()),
        }
    }
}

impl<'a> TryInto<f64> for XRPAmount<'a> {
    type Error = XRPLModelException;

    fn try_into(self) -> XRPLModelResult<f64, Self::Error> {
        Ok(self.0.parse::<f64>()?)
    }
}

impl<'a> TryInto<u32> for XRPAmount<'a> {
    type Error = XRPLModelException;

    fn try_into(self) -> XRPLModelResult<u32, Self::Error> {
        Ok(self.0.parse::<u32>()?)
    }
}

impl<'a> TryInto<u64> for XRPAmount<'a> {
    type Error = XRPLModelException;

    fn try_into(self) -> XRPLModelResult<u64, Self::Error> {
        Ok(self.0.parse::<u64>()?)
    }
}

impl<'a> TryInto<BigDecimal> for XRPAmount<'a> {
    type Error = XRPLModelException;

    fn try_into(self) -> XRPLModelResult<BigDecimal, Self::Error> {
        Ok(BigDecimal::from_str(&self.0)?)
    }
}

impl<'a> TryInto<Cow<'a, str>> for XRPAmount<'a> {
    type Error = XRPLModelException;

    fn try_into(self) -> XRPLModelResult<Cow<'a, str>, Self::Error> {
        Ok(self.0)
    }
}

impl<'a> XRPAmount<'a> {
    /// Compare two XRP amounts numerically.
    ///
    /// Returns an error if either side is not a valid numeric XRP amount. Use this
    /// when callers need to distinguish malformed input from an ordering result.
    pub fn checked_cmp(&self, other: &Self) -> XRPLModelResult<core::cmp::Ordering> {
        let self_decimal: BigDecimal = <Self as Clone>::clone(self).try_into()?;
        let other_decimal: BigDecimal = <Self as Clone>::clone(other).try_into()?;
        Ok(self_decimal.cmp(&other_decimal))
    }
}

// There is deliberately no `Ord`/`PartialOrd` here. An `XRPAmount` wraps a string
// the library does not control — amounts arrive from the network — so an
// infallible comparison has to panic on malformed input, and `amounts.sort()` on
// a parsed response could abort the process. `checked_cmp` is the honest API. A
// validated `Drops(u64)` newtype is the place for an infallible ordering if one
// is ever needed.

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{format, vec};
    use core::cmp::Ordering;

    #[test]
    fn test_checked_cmp_valid_amounts() {
        let amount1 = XRPAmount("100".into());
        let amount2 = XRPAmount("200".into());
        let amount3 = XRPAmount("100".into());

        assert_eq!(amount1.checked_cmp(&amount2).unwrap(), Ordering::Less);
        assert_eq!(amount2.checked_cmp(&amount1).unwrap(), Ordering::Greater);
        assert_eq!(amount1.checked_cmp(&amount3).unwrap(), Ordering::Equal);
    }

    #[test]
    fn test_checked_cmp_is_numeric_not_lexicographic() {
        // "9" sorts after "10" as a string and before it as a number.
        let nine = XRPAmount("9".into());
        let ten = XRPAmount("10".into());

        assert_eq!(nine.checked_cmp(&ten).unwrap(), Ordering::Less);
    }

    #[test]
    fn test_checked_cmp_zero() {
        let zero = XRPAmount("0".into());
        let positive = XRPAmount("100".into());

        assert_eq!(zero.checked_cmp(&positive).unwrap(), Ordering::Less);
        assert_eq!(positive.checked_cmp(&zero).unwrap(), Ordering::Greater);
    }

    #[test]
    fn test_checked_cmp_invalid_vs_valid_returns_error() {
        let valid = XRPAmount("100".into());
        let invalid = XRPAmount("not-a-number".into());

        assert!(valid.checked_cmp(&invalid).is_err());
        assert!(invalid.checked_cmp(&valid).is_err());
    }

    #[test]
    fn test_checked_cmp_both_invalid_returns_error() {
        let invalid1 = XRPAmount("not-a-number".into());
        let invalid2 = XRPAmount("also-invalid".into());

        assert!(invalid1.checked_cmp(&invalid2).is_err());
    }

    #[test]
    fn test_malformed_amount_cannot_panic_a_comparison() {
        // There is no `Ord`, so a malformed amount arriving from the network can
        // no longer abort a sort. The fallible comparison reports it instead.
        let valid = XRPAmount("100".into());
        let malformed = XRPAmount("xyz".into());

        assert!(valid.checked_cmp(&malformed).is_err());
    }

    #[test]
    fn test_sorting_valid_amounts() {
        let mut amounts = vec![
            XRPAmount("50".into()),
            XRPAmount("100".into()),
            XRPAmount("25".into()),
        ];

        // Sorting is the caller's decision now, and it has to say what happens to
        // a malformed amount. Here: treat the whole sort as fallible.
        amounts.sort_by(|a, b| a.checked_cmp(b).expect("test amounts are valid"));

        assert_eq!(amounts[0].0.as_ref(), "25");
        assert_eq!(amounts[1].0.as_ref(), "50");
        assert_eq!(amounts[2].0.as_ref(), "100");
    }

    #[test]
    fn test_validates_drops_beyond_the_u32_ceiling() {
        // u32::MAX drops is 4,294.967295 XRP. Everything above it used to fail
        // validation with a bare ParseIntError naming no field.
        for drops in [
            "4294967295",
            "5000000000",
            "100000000000",
            "100000000000000000",
        ] {
            assert!(
                XRPAmount(drops.into()).get_errors().is_ok(),
                "{drops} drops should validate"
            );
        }
    }

    #[test]
    fn test_rejects_more_than_the_total_supply() {
        let over = XRPAmount((MAX_DROPS + 1).to_string().into());
        let error = over.get_errors().unwrap_err();
        let message = format!("{error}");

        assert!(message.contains("XRPAmount"), "error should name the field");
        assert!(
            message.contains("100000000000000000"),
            "error should name the bound: {message}"
        );
    }

    #[test]
    fn test_rejects_a_non_integer_naming_the_field() {
        let error = XRPAmount("not-a-number".into()).get_errors().unwrap_err();
        let message = format!("{error}");

        assert!(message.contains("XRPAmount"), "error should name the field");
        assert!(
            message.contains("not-a-number"),
            "error should quote the value"
        );
    }

    #[test]
    fn test_u64_round_trip() {
        let amount = XRPAmount::from(100_000_000_000u64);
        assert_eq!(amount.0.as_ref(), "100000000000");

        let drops: u64 = amount.try_into().unwrap();
        assert_eq!(drops, 100_000_000_000u64);
    }

    #[test]
    fn test_try_from_value_rejects_object() {
        let obj_value = serde_json::json!({"key": "value"});
        let result = XRPAmount::try_from(obj_value);
        assert!(result.is_err(), "Object should be rejected");
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("object"));
    }

    #[test]
    fn test_try_from_value_rejects_array() {
        let array_value = serde_json::json!([1, 2, 3]);
        let result = XRPAmount::try_from(array_value);
        assert!(result.is_err(), "Array should be rejected");
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("array"));
    }

    #[test]
    fn test_try_from_value_rejects_null() {
        let null_value = serde_json::Value::Null;
        let result = XRPAmount::try_from(null_value);
        assert!(result.is_err(), "Null should be rejected");
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("null"));
    }

    #[test]
    fn test_try_from_value_rejects_boolean() {
        let bool_value = serde_json::json!(true);
        let result = XRPAmount::try_from(bool_value);
        assert!(result.is_err(), "Boolean should be rejected");
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("boolean"));
    }

    #[test]
    fn test_try_from_value_accepts_string() {
        let string_value = serde_json::json!("100");
        let result = XRPAmount::try_from(string_value);
        assert!(result.is_ok(), "String should be accepted");
        assert_eq!(result.unwrap().0.as_ref(), "100");
    }

    #[test]
    fn test_try_from_value_accepts_number() {
        let number_value = serde_json::json!(100);
        let result = XRPAmount::try_from(number_value);
        assert!(result.is_ok(), "Number should be accepted");
    }
}
