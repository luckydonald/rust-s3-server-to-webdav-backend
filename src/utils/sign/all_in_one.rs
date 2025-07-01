use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Sha256, Digest};

type HmacSha256 = Hmac<Sha256>;

fn get_signature_key(key: &str, datestamp: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sign(format!("AWS4{}", key).as_bytes(), datestamp.as_bytes());
    let k_region = hmac_sign(&k_date, region.as_bytes());
    let k_service = hmac_sign(&k_region, service.as_bytes());
    hmac_sign(&k_service, b"aws4_request")
}

fn hmac_sign(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

pub fn sigv4_authorization_header(
    method: &str,
    service: &str,
    host: &str,
    region: &str,
    endpoint: &str,
    date: Option<DateTime<Utc>>,
    access_key: &str,
    secret_key: &str,
) -> String {
    let t = date.unwrap_or_else(Utc::now);
    let amzdate = t.format("%Y%m%dT%H%M%SZ").to_string();
    let datestamp = t.format("%Y%m%d").to_string();

    // Canonical request
    let canonical_uri = endpoint;
    let canonical_querystring = "";
    let canonical_headers = format!("host:{}\n", host);
    let signed_headers = "host";
    let payload_hash = hex::encode(Sha256::digest(b""));
    let canonical_request = format!(
        "{method}\n{uri}\n{query}\n{headers}\n{signed}\n{payload}",
        method = method,
        uri = canonical_uri,
        query = canonical_querystring,
        headers = canonical_headers,
        signed = signed_headers,
        payload = payload_hash
    );

    // String to sign
    let algorithm = "AWS4-HMAC-SHA256";
    let credential_scope = format!("{}/{}/{}/aws4_request", datestamp, region, service);
    let string_to_sign = format!(
        "{alg}\n{amzdate}\n{scope}\n{hash}",
        alg = algorithm,
        amzdate = amzdate,
        scope = credential_scope,
        hash = hex::encode(Sha256::digest(canonical_request.as_bytes()))
    );

    // Derive signing key
    let signing_key = get_signature_key(secret_key, &datestamp, region, service);
    let mut mac = HmacSha256::new_from_slice(&signing_key).unwrap();
    mac.update(string_to_sign.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    // Authorization header
    let mut authorization_header = format!(
        "{} Credential={}/{}, SignedHeaders={}, Signature={}",
        algorithm,
        access_key,
        credential_scope,
        signed_headers,
        signature
    );
    authorization_header
}