use std::time::SystemTime;
use crate::utils::sign::date_time::{format_date, format_date_time};
use sha2::{Digest, Sha256};
use sha2::digest::FixedOutput;

// `use aws_sigv4::sign::v4::sha256_hex_string;`
/// HashedPayload = Lowercase(HexEncode(Hash(requestPayload)))
#[allow(dead_code)] // Unused when compiling without certain features
pub fn sha256_hex_string(bytes: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize_fixed())
}
pub type SigningParams<'a> = aws_sigv4::sign::v4::SigningParams<'a, ()>;

fn calculate_string_to_sign(
    message_payload: &[u8],
    last_signature: &str,
    time: SystemTime,
    params: &SigningParams<'_>,
) -> Vec<u8> {
    // Event Stream string to sign format is documented here:
    // https://docs.aws.amazon.com/transcribe/latest/dg/how-streaming.html
    let date_time_str = format_date_time(time);
    let date_str = format_date(time);

    let mut sts: Vec<u8> = Vec::new();
    writeln!(sts, "AWS4-HMAC-SHA256-PAYLOAD").unwrap();
    writeln!(sts, "{}", date_time_str).unwrap();
    writeln!(
        sts,
        "{}/{}/{}/aws4_request",
        date_str, params.region, params.name
    )
        .unwrap();
    writeln!(sts, "{}", last_signature).unwrap();

    let date_header = Header::new(":date", HeaderValue::Timestamp(time.into()));
    let mut date_buffer = Vec::new();
    write_headers_to(&[date_header], &mut date_buffer).unwrap();
    writeln!(sts, "{}", sha256_hex_string(&date_buffer)).unwrap();
    write!(sts, "{}", sha256_hex_string(message_payload)).unwrap();
    sts
}