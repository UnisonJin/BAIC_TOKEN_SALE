use cosmwasm_std::{
    entry_point, to_binary, BankMsg, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128, WasmMsg,
};

use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{user_info_key, user_info_storage, SaleInfo, State, UserInfo, CONFIG, SALEINFO};

const CONTRACT_NAME: &str = "BANANA_SALE";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const JUNO: &str = "ujuno";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    deps.api.addr_validate(&msg.admin)?;
    deps.api.addr_validate(&msg.token_address)?;

    let crr_time = env.block.time.seconds();
    if crr_time > msg.presale_start {
        return Err(ContractError::WrongConfig {});
    }

    //presale start, end and claim period check
    let state = State {
        admin: msg.admin,
        token_address: msg.token_address,
        total_supply: msg.total_supply,
        presale_start: msg.presale_start,
        presale_period: msg.presale_period,
        token_ratio: msg.token_ratio,
    };
    CONFIG.save(deps.storage, &state)?;

    SALEINFO.save(
        deps.storage,
        &SaleInfo {
            token_sold_amount: Uint128::zero(),
            earned_juno: Uint128::zero(),
        },
    )?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::BuyToken {} => execute_buy_token(deps, env, info),
        ExecuteMsg::ChangeAdmin { address } => execute_change_admin(deps, env, info, address),
        ExecuteMsg::UpdateConfig { state } => execute_update_config(deps, env, info, state),
        ExecuteMsg::WithdrawTokenByAdmin {} => execute_withdraw_token_by_admin(deps, env, info),
    }
}

fn execute_buy_token(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;
    let sender = info.sender.to_string();

    //presale start validation check
    let crr_time = env.block.time.seconds();
    if crr_time < state.presale_start {
        return Err(ContractError::PresaleNotStarted {});
    };
    if crr_time > state.presale_start + state.presale_period {
        return Err(ContractError::PresaleEnded {});
    }

    let received_coin = get_coin_info(&info)?;

    //token_amount checking
    let token_amount = received_coin.amount * state.token_ratio;
    let sale_info = SALEINFO.load(deps.storage)?;

    if token_amount + sale_info.token_sold_amount > state.total_supply {
        return Err(ContractError::NoEnoughTokens {});
    }

    //sale info update
    SALEINFO.update(deps.storage, |mut sale_info| -> StdResult<_> {
        sale_info.earned_juno = sale_info.earned_juno + received_coin.amount;
        sale_info.token_sold_amount = sale_info.token_sold_amount + token_amount;
        Ok(sale_info)
    })?;

    let user_info_key = user_info_key(&sender);

    //user info update
    let user_info = user_info_storage().may_load(deps.storage, user_info_key.clone())?;
    match user_info {
        Some(mut user_info) => {
            user_info.sent_juno = user_info.sent_juno + received_coin.amount;
            user_info.bought_token_amount = user_info.bought_token_amount + token_amount;
            user_info_storage().save(deps.storage, user_info_key, &user_info)?;
        }
        None => {
            user_info_storage().save(
                deps.storage,
                user_info_key,
                &UserInfo {
                    address: sender.clone(),
                    bought_token_amount: token_amount,
                    sent_juno: received_coin.amount,
                },
            )?;
        }
    }

    //messages handling
    let mut messages: Vec<CosmosMsg> = Vec::new();
    let token_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: sender.clone(),
        amount: token_amount,
    };

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.token_address,
        msg: to_binary(&token_transfer_msg)?,
        funds: vec![],
    }));

    let coin_send_msg = BankMsg::Send {
        to_address: state.admin,
        amount: vec![Coin {
            denom: JUNO.to_string(),
            amount: received_coin.amount,
        }],
    };
    messages.push(CosmosMsg::Bank(coin_send_msg));

    Ok(Response::new()
        .add_attribute("action", "buy_token")
        .add_attribute("denom", received_coin.denom)
        .add_attribute("amount", received_coin.amount.to_string())
        .add_attribute("buyer", sender)
        .add_messages(messages))
}

//Mint token to this contract
fn execute_change_admin(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    authcheck(deps.as_ref(), &info)?;

    CONFIG.update(deps.storage, |mut state| -> StdResult<_> {
        state.admin = address.clone();
        Ok(state)
    })?;

    Ok(Response::new()
        .add_attribute("action", "change the admin")
        .add_attribute("address", address))
}

fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    state: State,
) -> Result<Response, ContractError> {
    authcheck(deps.as_ref(), &info)?;

    CONFIG.save(deps.storage, &state)?;
    Ok(Response::new().add_attribute("action", "update configuration"))
}

fn execute_withdraw_token_by_admin(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    authcheck(deps.as_ref(), &info)?;

    let state = CONFIG.load(deps.storage)?;
    let sale_info = SALEINFO.load(deps.storage)?;
    let crr_time = env.block.time.seconds();
    let presale_end = state.presale_start + state.presale_period;

    if crr_time < presale_end {
        return Err(ContractError::PresaleNotEnded {});
    }

    let cw20_transfer_msg = WasmMsg::Execute {
        contract_addr: "token_address".to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: state.admin,
            amount: state.total_supply - sale_info.token_sold_amount,
        })?,
        funds: vec![],
    };

    let msg: CosmosMsg = CosmosMsg::Wasm(cw20_transfer_msg);

    Ok(Response::new()
        .add_attribute("action", "withdraw token by admin")
        .add_message(msg))
}

fn authcheck(deps: Deps, info: &MessageInfo) -> Result<(), ContractError> {
    let state = CONFIG.load(deps.storage)?;
    if info.sender != state.admin {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

fn get_coin_info(info: &MessageInfo) -> Result<Coin, ContractError> {
    if info.funds.len() != 1 {
        return Err(ContractError::SeveralCoinsSent {});
    } else {
        let denom = info.funds[0].denom.clone();
        if denom.as_str() != JUNO {
            return Err(ContractError::InvalidCoin {});
        }
        Ok(info.funds[0].clone())
    }
}
