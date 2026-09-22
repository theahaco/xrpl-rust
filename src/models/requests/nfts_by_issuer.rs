use alloc::borrow::Cow;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::models::Model;

use super::{CommonFields, LedgerIndex, LookupByLedgerRequest, Marker, Request, RequestMethod};

/// The `nfts_by_issuer` method retrieves all of the NFTokens
/// issued by an account
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct NFTsByIssuer<'a> {
    /// The common fields shared by all requests.
    #[serde(flatten)]
    pub common_fields: CommonFields<'a>,
    /// The unique identifier of a ledger.
    #[serde(flatten)]
    pub ledger_lookup: Option<LookupByLedgerRequest<'a>>,
    /// The unique identifier for an account that issues NFTokens
    /// The request returns NFTokens issued by this account.
    pub issuer: Cow<'a, str>,
    pub limit: Option<u32>,
    pub marker: Option<Marker<'a>>,
    pub nft_taxon: Option<u64>,
}

impl Model for NFTsByIssuer<'_> {}

impl<'a> Request<'a> for NFTsByIssuer<'a> {
    fn get_common_fields(&self) -> &CommonFields<'a> {
        &self.common_fields
    }

    fn get_common_fields_mut(&mut self) -> &mut CommonFields<'a> {
        &mut self.common_fields
    }
}

#[bon::bon]
impl<'a> NFTsByIssuer<'a> {
    #[builder]
    pub fn new(
        #[builder(start_fn, into)] issuer: Cow<'a, str>,
        #[builder(into)] id: Option<Cow<'a, str>>,
        #[builder(into)] ledger_hash: Option<Cow<'a, str>>,
        #[builder(into)] ledger_index: Option<LedgerIndex<'a>>,
        limit: Option<u32>,
        #[builder(into)] marker: Option<Marker<'a>>,
        nft_taxon: Option<u64>,
    ) -> Self {
        Self {
            common_fields: CommonFields {
                command: RequestMethod::NFTsByIssuer,
                id,
            },
            ledger_lookup: Some(LookupByLedgerRequest {
                ledger_hash,
                ledger_index,
            }),
            issuer,
            limit,
            marker,
            nft_taxon,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_round_trip() {
        let req = NFTsByIssuer::builder("rIssuer11111111111111111111111111")
            .id("nbi-1")
            .limit(100)
            .marker(Marker::Int(1))
            .nft_taxon(42)
            .build();
        let serialized = serde_json::to_string(&req).unwrap();
        let deserialized: NFTsByIssuer = serde_json::from_str(&serialized).unwrap();
        assert_eq!(req, deserialized);
        assert!(serialized.contains("\"command\":\"nfts_by_issuer\""));
    }
}
