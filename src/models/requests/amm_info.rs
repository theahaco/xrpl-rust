use alloc::borrow::Cow;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::models::{Currency, Model};

use super::{CommonFields, Request};

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct AMMInfo<'a> {
    /// The common fields shared by all requests.
    #[serde(flatten)]
    pub common_fields: CommonFields<'a>,
    pub amm_account: Option<Cow<'a, str>>,
    pub asset: Option<Currency<'a>>,
    pub asset2: Option<Currency<'a>>,
}

impl Model for AMMInfo<'_> {}

impl<'a> Request<'a> for AMMInfo<'a> {
    fn get_common_fields(&self) -> &CommonFields<'a> {
        &self.common_fields
    }

    fn get_common_fields_mut(&mut self) -> &mut CommonFields<'a> {
        &mut self.common_fields
    }
}

#[bon::bon]
impl<'a> AMMInfo<'a> {
    #[builder]
    pub fn new(
        #[builder(into)] id: Option<Cow<'a, str>>,
        #[builder(into)] amm_account: Option<Cow<'a, str>>,
        #[builder(into)] asset: Option<Currency<'a>>,
        #[builder(into)] asset2: Option<Currency<'a>>,
    ) -> Self {
        Self {
            common_fields: CommonFields {
                command: super::RequestMethod::AMMInfo,
                id,
            },
            amm_account,
            asset,
            asset2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::currency::{IssuedCurrency, XRP};

    #[test]
    fn test_serde_round_trip() {
        let req = AMMInfo::builder()
            .id("amm-1")
            .amm_account("rAMM1111111111111111111111111111111")
            .asset(Currency::XRP(XRP::new()))
            .asset2(Currency::IssuedCurrency(IssuedCurrency::new(
                "USD".into(),
                "rIssuer11111111111111111111111111".into(),
            )))
            .build();
        let serialized = serde_json::to_string(&req).unwrap();
        let deserialized: AMMInfo = serde_json::from_str(&serialized).unwrap();
        assert_eq!(req, deserialized);
        assert!(serialized.contains("\"command\":\"amm_info\""));
    }
}
