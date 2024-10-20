// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::helpers::{
    CachedImmutableEpochItem, CachedImmutableItems, IssuedCoinIndicesSignatures,
    IssuedExpirationDateSignatures,
};
use crate::ecash::keys::KeyPair;
use nym_config::defaults::BloomfilterParameters;
use nym_crypto::asymmetric::identity;
use nym_ecash_double_spending::DoubleSpendingFilter;
use std::sync::Arc;
use time::Date;
use tokio::sync::RwLock;

pub(crate) struct TicketDoubleSpendingFilter {
    built_on: Date,
    params_id: i64,

    today_filter: DoubleSpendingFilter,
    global_filter: DoubleSpendingFilter,
}

impl TicketDoubleSpendingFilter {
    pub(crate) fn new(
        built_on: Date,
        params_id: i64,
        global_filter: DoubleSpendingFilter,
        today_filter: DoubleSpendingFilter,
    ) -> TicketDoubleSpendingFilter {
        TicketDoubleSpendingFilter {
            built_on,
            params_id,
            today_filter,
            global_filter,
        }
    }

    pub(crate) fn built_on(&self) -> Date {
        self.built_on
    }

    pub(crate) fn params(&self) -> BloomfilterParameters {
        self.today_filter.params()
    }

    pub(crate) fn params_id(&self) -> i64 {
        self.params_id
    }

    pub(crate) fn check(&self, sn: &Vec<u8>) -> bool {
        self.global_filter.check(sn)
    }

    /// Returns boolean to indicate if the entry was already present
    pub(crate) fn insert_both(&mut self, sn: &Vec<u8>) -> bool {
        self.today_filter.set(sn);
        self.insert_global_only(sn)
    }

    /// Returns boolean to indicate if the entry was already present
    pub(crate) fn insert_global_only(&mut self, sn: &Vec<u8>) -> bool {
        let existed = self.global_filter.check(sn);
        self.global_filter.set(sn);
        existed
    }

    pub(crate) fn export_today_bitmap(&self) -> Vec<u8> {
        self.today_filter.dump_bitmap()
    }

    pub(crate) fn advance_day(&mut self, date: Date, new_global: DoubleSpendingFilter) {
        self.built_on = date;
        self.global_filter = new_global;
        self.today_filter.reset();
    }
}

pub(crate) struct LocalEcashState {
    pub(crate) ecash_keypair: KeyPair,
    pub(crate) identity_keypair: identity::KeyPair,

    pub(crate) explicitly_disabled: bool,

    /// Specifies whether this api is a signer in given epoch
    pub(crate) active_signer: CachedImmutableEpochItem<bool>,

    pub(crate) partial_coin_index_signatures: CachedImmutableEpochItem<IssuedCoinIndicesSignatures>,
    pub(crate) partial_expiration_date_signatures:
        CachedImmutableItems<Date, IssuedExpirationDateSignatures>,

    // the actual, up to date, bloomfilter
    pub(crate) double_spending_filter: Arc<RwLock<TicketDoubleSpendingFilter>>,
}

impl LocalEcashState {
    pub(crate) fn new(
        ecash_keypair: KeyPair,
        identity_keypair: identity::KeyPair,
        double_spending_filter: TicketDoubleSpendingFilter,
        explicitly_disabled: bool,
    ) -> Self {
        LocalEcashState {
            ecash_keypair,
            identity_keypair,
            explicitly_disabled,
            active_signer: Default::default(),
            partial_coin_index_signatures: Default::default(),
            partial_expiration_date_signatures: Default::default(),
            double_spending_filter: Arc::new(RwLock::new(double_spending_filter)),
        }
    }
}
