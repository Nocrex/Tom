// @generated automatically by Diesel CLI.

diesel::table! {
    playerreports (report, steamid) {
        report -> Int4,
        steamid -> Int8,
        last_seen -> Timestamp,
        attribute -> Int2,
        verified -> Bool,
    }
}

diesel::table! {
    reporters (id) {
        id -> Int8,
        steamid -> Nullable<Int8>,
    }
}

diesel::table! {
    reports (id) {
        id -> Int4,
        reporter -> Int8,
        time -> Timestamp,
        points -> Int2,
        threadurl -> Text,
        message -> Text,
    }
}

diesel::joinable!(playerreports -> reports (report));
diesel::joinable!(reports -> reporters (reporter));

diesel::allow_tables_to_appear_in_same_query!(playerreports, reporters, reports,);
