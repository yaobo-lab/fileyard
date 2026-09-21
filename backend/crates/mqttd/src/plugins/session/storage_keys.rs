use super::storage_info::StoredKey;
use bytes::Bytes;
use std::convert::From as _;

#[inline]
pub(super) fn make_map_stored_key<T: AsRef<[u8]>>(id: T) -> StoredKey {
    let mut key = Vec::from("map-");
    key.extend_from_slice(id.as_ref());
    Bytes::from(key)
}

#[inline]
pub(super) fn map_stored_key_to_id_bytes(stored_key: &[u8]) -> &[u8] {
    if stored_key.starts_with(b"map-") {
        stored_key[4..].as_ref()
    } else {
        stored_key
    }
}

#[inline]
pub(super) fn make_list_stored_key<T: AsRef<[u8]>>(id: T) -> StoredKey {
    let mut key = Vec::from("list-");
    key.extend_from_slice(id.as_ref());
    Bytes::from(key)
}

#[inline]
pub(super) fn list_stored_key_to_id_bytes(stored_key: &[u8]) -> &[u8] {
    if stored_key.starts_with(b"list-") {
        stored_key[5..].as_ref()
    } else {
        stored_key
    }
}
