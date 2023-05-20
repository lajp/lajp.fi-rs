diesel::table! {
    visits (id) {
        id -> Int4,
        visitor -> Text,
        path -> Text,
        instance -> Timestamp,
    }
}
