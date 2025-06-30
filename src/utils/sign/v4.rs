use sha2::Sha256;

#[allow(dead_code)] // Unused when compiling without certain features
pub(crate) fn sha256_hex_string(bytes: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize_fixed())
}
