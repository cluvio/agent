/// Convert to URL-safe base64 string without padding.
pub fn encode<T: AsRef<[u8]>>(bytes: T) -> String {
    base64::encode_config(bytes.as_ref(), base64::URL_SAFE_NO_PAD)
}

/// Convert from URL-safe base64 string without padding.
pub fn decode(s: &str) -> Option<Vec<u8>> {
    base64::decode_config(s, base64::URL_SAFE_NO_PAD).ok()
}

