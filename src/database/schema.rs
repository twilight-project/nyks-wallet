// This file is for manually-written schema. For auto-generated schema,
// diesel would typically place it in schema.rs

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
diesel::table! {
    zk_accounts (id) {
        id -> Nullable<Integer>,
        wallet_id -> Text,
        account_index -> BigInt,
        qq_address -> Text,
        balance -> BigInt,
        account -> Text,
        scalar -> Text,
        io_type_value -> Integer,
        on_chain -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
diesel::table! {
    encrypted_wallets (id) {
        id -> Nullable<Integer>,
        wallet_id -> Text,
        encrypted_data -> Binary,
        salt -> Binary,
        nonce -> Binary,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
diesel::table! {
    order_wallets (id) {
        id -> Nullable<Integer>,
        wallet_id -> Text,
        chain_id -> Text,
        seed_encrypted -> Binary,
        seed_salt -> Binary,
        seed_nonce -> Binary,
        relayer_api_endpoint -> Text,
        zkos_server_endpoint -> Text,
        relayer_program_json_path -> Text,
        is_active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
diesel::table! {
    utxo_details (id) {
        id -> Nullable<Integer>,
        wallet_id -> Text,
        account_index -> BigInt,
        utxo_data -> Text, // JSON serialized UtxoDetailResponse
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
diesel::table! {
    request_ids (id) {
        id -> Nullable<Integer>,
        wallet_id -> Text,
        account_index -> BigInt,
        request_id -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
diesel::allow_tables_to_appear_in_same_query!(
    zk_accounts,
    encrypted_wallets,
    order_wallets,
    utxo_details,
    request_ids,
);
