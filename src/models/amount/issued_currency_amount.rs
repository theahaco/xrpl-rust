use crate::core::addresscodec::is_valid_classic_address;
use crate::models::{Model, XRPLModelException, XRPLModelResult};
use crate::utils::{is_iso_code, is_iso_hex};
use alloc::borrow::Cow;
use alloc::string::ToString;
use bigdecimal::num_bigint::Sign;
use bigdecimal::BigDecimal;
use core::convert::TryInto;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct IssuedCurrencyAmount<'a> {
    pub currency: Cow<'a, str>,
    pub issuer: Cow<'a, str>,
    pub value: Cow<'a, str>,
}

impl<'a> Model for IssuedCurrencyAmount<'a> {
    fn get_errors(&self) -> XRPLModelResult<()> {
        // A currency is either a 3-character ISO code or a 40-character hex code.
        if !is_iso_code(&self.currency) && !is_iso_hex(&self.currency) {
            return Err(XRPLModelException::InvalidValue {
                field: "IssuedCurrencyAmount.currency".into(),
                expected: "a 3-character ISO code or a 40-character hex code".into(),
                found: self.currency.to_string(),
            });
        }

        if !is_valid_classic_address(&self.issuer) {
            return Err(XRPLModelException::InvalidValue {
                field: "IssuedCurrencyAmount.issuer".into(),
                expected: "a classic address".into(),
                found: self.issuer.to_string(),
            });
        }

        // Issued amounts are 15-significant-digit decimals, not floats. The crate
        // already depends on `bigdecimal`; parsing as `f64` both lost precision
        // and accepted things the ledger does not.
        let value =
            BigDecimal::from_str(&self.value).map_err(|_| XRPLModelException::InvalidValue {
                field: "IssuedCurrencyAmount.value".into(),
                expected: "a decimal number".into(),
                found: self.value.to_string(),
            })?;

        if value.sign() == Sign::Minus {
            return Err(XRPLModelException::InvalidValue {
                field: "IssuedCurrencyAmount.value".into(),
                expected: "a non-negative amount".into(),
                found: self.value.to_string(),
            });
        }

        Ok(())
    }
}

impl<'a> IssuedCurrencyAmount<'a> {
    /// Compare two issued amounts numerically.
    ///
    /// Two amounts are only comparable when they name the same currency and the
    /// same issuer: one USD is not one EUR, and one issuer's USD is not another's.
    /// Returns an error when they differ, or when either value is not a number.
    pub fn checked_cmp(&self, other: &Self) -> XRPLModelResult<core::cmp::Ordering> {
        if self.currency != other.currency || self.issuer != other.issuer {
            return Err(XRPLModelException::InvalidValue {
                field: "IssuedCurrencyAmount".into(),
                expected: alloc::format!("{}/{}", self.currency, self.issuer),
                found: alloc::format!("{}/{}", other.currency, other.issuer),
            });
        }

        let this = BigDecimal::from_str(&self.value)?;
        let that = BigDecimal::from_str(&other.value)?;

        Ok(this.cmp(&that))
    }

    pub fn new(currency: Cow<'a, str>, issuer: Cow<'a, str>, value: Cow<'a, str>) -> Self {
        Self {
            currency,
            issuer,
            value,
        }
    }
}

impl<'a> TryInto<BigDecimal> for IssuedCurrencyAmount<'a> {
    type Error = XRPLModelException;

    fn try_into(self) -> XRPLModelResult<BigDecimal, Self::Error> {
        Ok(BigDecimal::from_str(&self.value)?)
    }
}

// There is deliberately no `Ord`/`PartialOrd` here. The previous impl compared
// only `value`, lexicographically, while the derived `PartialEq` compared all
// three fields — so `Ord` and `Eq` disagreed, which `Ord`'s contract forbids and
// which silently loses data: inserting a EUR balance into a `BTreeSet` already
// holding a USD balance with the same numeral was a no-op. `checked_cmp` refuses
// to compare amounts that do not name the same currency and issuer.

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use core::cmp::Ordering;

    const ISSUER: &str = "r9cZA1mLK5R5Am25ArfXFmqgNwjZgnfk59";
    const OTHER_ISSUER: &str = "r4DymtkgUAh2wqRxVfdd3Xtswzim6eC6c5";

    fn usd(value: &str) -> IssuedCurrencyAmount<'static> {
        IssuedCurrencyAmount::new("USD".into(), ISSUER.into(), value.to_string().into())
    }

    #[test]
    fn test_accepts_an_iso_code_and_a_hex_code() {
        assert!(usd("100").get_errors().is_ok());

        let hex = IssuedCurrencyAmount::new(
            "0000000000000000000000005553440000000000".into(),
            ISSUER.into(),
            "100".into(),
        );
        assert!(hex.get_errors().is_ok());
    }

    #[test]
    fn test_rejects_a_malformed_currency_code() {
        let amount = IssuedCurrencyAmount::new("TOOLONGCODE".into(), ISSUER.into(), "100".into());

        let message = format!("{}", amount.get_errors().unwrap_err());
        assert!(message.contains("currency"), "{message}");
    }

    #[test]
    fn test_rejects_a_malformed_issuer() {
        let amount = IssuedCurrencyAmount::new("USD".into(), "not-an-address".into(), "100".into());

        let message = format!("{}", amount.get_errors().unwrap_err());
        assert!(message.contains("issuer"), "{message}");
    }

    #[test]
    fn test_rejects_a_negative_value() {
        let message = format!("{}", usd("-5").get_errors().unwrap_err());
        assert!(message.contains("value"), "{message}");
        assert!(message.contains("non-negative"), "{message}");
    }

    #[test]
    fn test_rejects_a_non_numeric_value() {
        let message = format!("{}", usd("not-a-number").get_errors().unwrap_err());
        assert!(message.contains("value"), "{message}");
    }

    #[test]
    fn test_checked_cmp_is_numeric_not_lexicographic() {
        // The deleted `Ord` compared the value strings, so "9" ranked above "10".
        assert_eq!(usd("9").checked_cmp(&usd("10")).unwrap(), Ordering::Less);
        assert_eq!(usd("10").checked_cmp(&usd("9")).unwrap(), Ordering::Greater);
        assert_eq!(
            usd("10").checked_cmp(&usd("10.0")).unwrap(),
            Ordering::Equal
        );
    }

    #[test]
    fn test_checked_cmp_refuses_different_currencies_and_issuers() {
        let eur = IssuedCurrencyAmount::new("EUR".into(), ISSUER.into(), "100".into());
        let other_usd = IssuedCurrencyAmount::new("USD".into(), OTHER_ISSUER.into(), "100".into());

        // The deleted `Ord` called both of these `Equal` while `PartialEq` called
        // them unequal — the contract violation that made `BTreeSet` lose entries.
        assert!(usd("100").checked_cmp(&eur).is_err());
        assert!(usd("100").checked_cmp(&other_usd).is_err());
        assert_ne!(usd("100"), eur);
        assert_ne!(usd("100"), other_usd);
    }
}
