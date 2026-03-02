// @generated automatically by Diesel CLI.
// Manually fixed: PRIMARY KEY columns should be Integer, not Nullable<Integer>

diesel::table! {
    events (id) {
        id -> Integer,
        event_id -> Text,
        aggregate_type -> Text,
        aggregate_id -> Text,
        sequence -> Integer,
        event_type -> Text,
        payload -> Text,
        metadata -> Nullable<Text>,
        timestamp -> Integer,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        name -> Text,
        email -> Text,
        password -> Text,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(events, users,);
