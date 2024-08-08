// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    node_status_api::models::{AxumErrorResponse, AxumResult},
    support::http::static_routes,
    v2::AxumAppState,
};
use axum::{extract, Router};
use nym_api_requests::models::CirculatingSupplyResponse;
use nym_validator_client::nyxd::Coin;

pub(crate) fn circulating_supply_routes() -> Router<AxumAppState> {
    Router::new()
        .route(
            &static_routes::v1::circulating_supply(),
            axum::routing::get(get_full_circulating_supply),
        )
        .route(
            &static_routes::v1::circulating_supply::circulating_supply_value(),
            axum::routing::get(get_circulating_supply),
        )
        .route(
            &static_routes::v1::circulating_supply::total_supply_value(),
            axum::routing::get(get_total_supply),
        )
}

// TODO dz consider "substates" axum pattern
async fn get_full_circulating_supply(
    extract::State(state): extract::State<AxumAppState>,
) -> AxumResult<axum::Json<CirculatingSupplyResponse>> {
    match state
        .circulating_supply_cache()
        .get_circulating_supply()
        .await
    {
        Some(value) => Ok(value.into()),
        None => Err(AxumErrorResponse::internal_msg("unavailable")),
    }
}

async fn get_total_supply(
    extract::State(state): extract::State<AxumAppState>,
) -> AxumResult<axum::Json<f64>> {
    let full_circulating_supply = match state
        .circulating_supply_cache()
        .get_circulating_supply()
        .await
    {
        Some(res) => res,
        None => return Err(AxumErrorResponse::internal_msg("unavailable")),
    };

    Ok(unym_coin_to_float_unym(full_circulating_supply.total_supply.into()).into())
}

async fn get_circulating_supply(
    extract::State(state): extract::State<AxumAppState>,
) -> AxumResult<axum::Json<f64>> {
    let full_circulating_supply = match state
        .circulating_supply_cache()
        .get_circulating_supply()
        .await
    {
        Some(res) => res,
        None => return Err(AxumErrorResponse::internal_msg("unavailable")),
    };

    Ok(unym_coin_to_float_unym(full_circulating_supply.circulating_supply.into()).into())
}

// TODO: this is not the best place to put it, it should be more centralised,
// but for a quick fix, that's good enough for now...
// (for proper solution we should be managing `NymNetworkDetails` via rocket and grabbing display exponent
// value from the mix denom here.
const UNYM_RATIO: f64 = 1000000.;

fn unym_coin_to_float_unym(coin: Coin) -> f64 {
    // our total supply can't exceed 1B so an overflow here is impossible
    // (if it happened, then we SHOULD crash)
    coin.amount as f64 / UNYM_RATIO
}
