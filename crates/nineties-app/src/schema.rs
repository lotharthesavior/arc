// @generated automatically by Diesel CLI.
// Manually fixed: PRIMARY KEY columns should be Integer, not Nullable<Integer>

diesel::table! {
    events (id) {
        id -> Integer,
        event_id -> Text,
        aggregate_type -> Text,
        aggregate_id -> Text,
        sequence -> BigInt,
        event_type -> Text,
        payload -> Text,
        timestamp -> BigInt,
        actor_id -> Text,
        actor_session_id -> Nullable<Text>,
        source_ip -> Nullable<Text>,
        user_agent -> Nullable<Text>,
        timestamp_utc_us -> BigInt,
        causation_id -> Nullable<Text>,
        correlation_id -> Text,
    }
}
