/*
 * Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE ecash_credentials
(
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    voucher_info        TEXT    NOT NULL,
    wallet              TEXT    NOT NULL UNIQUE,
    secret_key          TEXT    NOT NULL,
    epoch_id            TEXT    NOT NULL,
    consumed            BOOLEAN NOT NULL
);