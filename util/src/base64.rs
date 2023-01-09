use base64::Engine;

/// Convert to URL-safe base64 string without padding.
pub fn encode<T: AsRef<[u8]>>(bytes: T) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes.as_ref())
}

/// Convert from URL-safe base64 string without padding.
pub fn decode(s: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).ok()
}

