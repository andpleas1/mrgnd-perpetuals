use cosmwasm_std::{
    Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response,
    Reply, ReplyOn, StdError, StdResult, SubMsg, to_binary, Uint128,
    WasmMsg, SubMsgExecutionResponse,
};

use margined_perp::margined_vamm::{Direction, ExecuteMsg};
use margined_perp::margined_engine::Side;
use crate::{
    contract::{SWAP_INCREASE_REPLY_ID, SWAP_DECREASE_REPLY_ID, SWAP_REVERSE_REPLY_ID},
    state::{
        Config, read_config, store_config,
        Position, read_position, store_position,
        store_tmp_position, read_tmp_position, remove_tmp_position,
    },
};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;
    if info.sender != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    let new_config = Config {
        owner: deps.api.addr_validate(&owner).unwrap(),
        ..config
    };

    store_config(deps.storage, &new_config)?;

    Ok(Response::default())
}


// Opens a position
pub fn open_position(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    vamm: String,
    trader: String,
    side: Side,
    quote_asset_amount: Uint128,
    leverage: Uint128,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    // create a response, so that we can assign relevant submessages to
    // be executed
    let mut response = Response::new();
    response = response.add_attributes(vec![("action", "open_position")]);

    // validate address inputs
    let vamm = deps.api.addr_validate(&vamm)?;
    let trader = deps.api.addr_validate(&trader)?;

    // calc the input amount wrt to leverage and decimals
    let input_amount = quote_asset_amount
                .checked_mul(leverage)?
                .checked_div(config.decimals)?;

    // read the position for the trader from vamm
    let current_position = read_position(deps.storage, &vamm, &trader)?;
    let mut position = Position::default();

    // so if the position returned is None then its new
    if current_position.is_none() {
        let direction: Direction = side_to_direction(side.clone());

        // update the default position
        position.vamm = vamm.clone();
        position.trader = trader.clone();
        position.direction = direction.clone();
        position.timestamp = env.block.time;

    } else {
        position = current_position.unwrap();
    }
    
    let mut is_increase: bool = true;
    if !(position.direction == Direction::AddToAmm && side.clone() == Side::BUY) &&
            !(position.direction == Direction::RemoveFromAmm && side.clone() == Side::SELL) {
        is_increase = false;
    }

    if is_increase {
        // then we are opening a new position or adding to an existing
        let msg = swap_input(
            &vamm,
            side.clone(),
            input_amount,
            SWAP_INCREASE_REPLY_ID
        ).unwrap();

        // increase the margin, notional etc...
        position.margin = position.margin.checked_add(quote_asset_amount)?;
        position.notional = position.notional.checked_add(input_amount)?;

        // Add the submessage to the response
        response = response.add_submessage(msg);

    } else {
        // if old position is greater then we don't need to reverse just need to 
        // reduce
        // else if it is equal or great there is more complicado stuff to do
        if position.notional > input_amount {
            println!("{}, {}", position.notional, input_amount);
            // then we are opening a new position or adding to an existing
            let msg = swap_input(
                &vamm,
                side.clone(),
                input_amount,
                SWAP_INCREASE_REPLY_ID
            ).unwrap();

            // increase the margin, notional etc...
            position.margin = position.margin.checked_sub(quote_asset_amount)?;
            position.notional = position.notional.checked_sub(input_amount)?;

            // Add the submessage to the response
            response = response.add_submessage(msg);
        } else {
            println!("{}, {}", position.notional, input_amount);
            // then we are opening a new position or adding to an existing
            let msg = swap_input(
                &vamm,
                side.clone(),
                input_amount,
                SWAP_INCREASE_REPLY_ID
            ).unwrap();

            // increase the margin, notional etc...
            position.margin = position.margin.checked_sub(quote_asset_amount)?;
            position.notional = position.notional.checked_sub(input_amount)?;

            // Add the submessage to the response
            response = response.add_submessage(msg);
        }
    }

    store_tmp_position(deps.storage, &position)?;



    Ok(response)
}

// Updates position after successful execution of the swap
pub fn update_position(
    deps: DepsMut,
    _env: Env,
    input: Uint128,
    output: Uint128,
) -> StdResult<Response> {
    let tmp_position = read_tmp_position(deps.storage)?;
    if tmp_position.is_none() {
        return Err(StdError::generic_err("no temporary position"));
    }

    // TODO update the position with what actually happened in the
    // swap, probably later this requires to check if long, short,buy, sell
    // but for now lets just implement the long case
    let mut position: Position = tmp_position.unwrap();
    position.size = position.size.checked_add(output)?;

    // store the updated position
    store_position(deps.storage, &position)?;

    // remove the tmp position
    remove_tmp_position(deps.storage);

    Ok(Response::new())
}

fn swap_input(
    vamm: &Addr, 
    side: Side, 
    input_amount: Uint128, 
    id: u64,
) -> StdResult<SubMsg> {
    let direction: Direction = side_to_direction(side);

    let swap_msg = WasmMsg::Execute {
        contract_addr: vamm.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::SwapInput {
            direction: direction,
            quote_asset_amount: input_amount,
        })?,
    };

    let execute_submsg = SubMsg {
        msg: CosmosMsg::Wasm(swap_msg),
        gas_limit: None, // probably should set a limit in the config
        id: id,
        reply_on: ReplyOn::Always,
    };

    Ok(execute_submsg)
}

// takes the side (buy|sell) and returns the direction (long|short)
fn side_to_direction(
    side: Side,
) -> Direction {
    let direction: Direction = match side {
            Side::BUY => Direction::AddToAmm,
            Side::SELL => Direction::RemoveFromAmm,
    };

    return direction
}

// /// Unit tests
// #[test]
// fn test_get_input_and_output_price() {}
