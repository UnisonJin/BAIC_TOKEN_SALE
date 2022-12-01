use crate::msg::{QueryMsg, TimeInfo, UserInfoResponse, UserInfosResponse};
use crate::state::{user_info_key, user_info_storage, SaleInfo, State, UserInfo, CONFIG, SALEINFO};
use cosmwasm_std::{entry_point, to_binary, Binary, Decimal, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

// Query limits
const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 30;

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetStateInfo {} => to_binary(&query_state_info(deps)?),
        QueryMsg::GetUserInfo { address } => to_binary(&query_user_info(deps, address)?),
        QueryMsg::GetSaleInfo {} => to_binary(&query_sale_info(deps)?),
        QueryMsg::GetUserInfos { start_after, limit } => {
            to_binary(&query_get_user_infos(deps, start_after, limit)?)
        }
    }
}

pub fn query_state_info(deps: Deps) -> StdResult<State> {
    let state = CONFIG.load(deps.storage)?;
    Ok(state)
}

pub fn query_user_info(deps: Deps, address: String) -> StdResult<UserInfoResponse> {
    let user_info_key = user_info_key(&address);
    let user_info = user_info_storage().may_load(deps.storage, user_info_key)?;
    deps.api.addr_validate(&address)?;
    match user_info {
        Some(user_info) => Ok(UserInfoResponse { user_info }),
        None => {
            let user_info = UserInfo {
                address,
                bought_token_amount: Uint128::zero(),
                sent_juno: Uint128::zero(),
            };
            Ok(UserInfoResponse { user_info })
        }
    }
}

pub fn query_sale_info(deps: Deps) -> StdResult<SaleInfo> {
    let sale_info = SALEINFO.load(deps.storage)?;
    Ok(sale_info)
}

pub fn query_get_user_infos(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<UserInfosResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let user_info = user_info_storage()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;
    Ok(UserInfosResponse { user_info })
}
