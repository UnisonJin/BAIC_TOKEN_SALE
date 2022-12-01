use cosmwasm_std::Uint128;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<State> = Item::new("config_state");
pub const SALEINFO: Item<SaleInfo> = Item::new("config_sale_info");
pub const COININFO: Map<&str, bool> = Map::new("config_token_info");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub admin: String,
    pub token_address: String,
    pub total_supply: Uint128,
    pub presale_start: u64,
    pub presale_period: u64,
    pub token_ratio: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SaleInfo {
    pub token_sold_amount: Uint128,
    pub earned_juno: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserInfo {
    pub address: String,
    pub bought_token_amount: Uint128,
    pub sent_juno: Uint128,
}

pub type UserInfoKey<'a> = String;

pub fn user_info_key<'a>(address: &'a String) -> UserInfoKey<'a> {
    address.clone()
}

pub struct UserInfoIndicies<'a> {
    pub address: MultiIndex<'a, String, UserInfo, UserInfoKey<'a>>,
}

impl<'a> IndexList<UserInfo> for UserInfoIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UserInfo>> + '_> {
        let v: Vec<&dyn Index<UserInfo>> = vec![&self.address];
        Box::new(v.into_iter())
    }
}

pub fn user_info_storage<'a>() -> IndexedMap<'a, UserInfoKey<'a>, UserInfo, UserInfoIndicies<'a>> {
    let indexes = UserInfoIndicies {
        address: MultiIndex::new(
            |d: &UserInfo| d.address.clone(),
            "user_info",
            "user_info__collection",
        ),
    };
    IndexedMap::new("user_info", indexes)
}
