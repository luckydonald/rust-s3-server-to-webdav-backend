use std::arch::aarch64::vceqz_f32;
// Taken in parts from
// https://github.com/dacut/scratchstack-aws-signature/blob/main/src/signature.rs
// which is licensed under MIT License, Copyright (c) 2021 David Cuthbert.
use std::collections::HashMap;
use std::str::from_utf8;
use std::time::SystemTime;
use rocket::form::ValueField;
use rocket::http::HeaderMap;
use scratchstack_aws_signature::{normalize_uri_path_component, SignatureError};
use {
    chrono::{
        format::{ParseError, ParseResult},
        offset::FixedOffset,
        DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc,
    },
    lazy_static::lazy_static,
    regex::Regex,
    std::{error::Error, str::FromStr},
};
use crate::date_utils::parse_date_str;

/// Algorithm for AWS SigV4
const AWS4_HMAC_SHA256: &str = "AWS4-HMAC-SHA256";

/// String included at the end of the AWS SigV4 credential scope
const AWS4_REQUEST: &str = "aws4_request";

/// Header parameter for the authorization
const AUTHORIZATION: &str = "authorization";

/// Content-Type parameter for specifying the character set
const CHARSET: &str = "charset";

/// Signature field for the access key
const CREDENTIAL: &str = "Credential";

/// Header field for the content type
const CONTENT_TYPE: &str = "content-type";

/// Header parameter for the date
const DATE: &str = "date";

/// Compact ISO8601 format used for the string to sign
const ISO8601_COMPACT_FORMAT: &str = "%Y%m%dT%H%M%SZ";

/// SHA-256 of an empty string.
const SHA256_EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Signature field for the signature itself
const SIGNATURE: &str = "Signature";

/// Authorization header parameter specifying the signed headers
const SIGNEDHEADERS: &str = "SignedHeaders";

/// Query parameter for delivering the access key
const X_AMZ_CREDENTIAL: &str = "X-Amz-Credential";

/// Query parameter for delivering the date
const X_AMZ_DATE: &str = "X-Amz-Date";

/// Header for delivering the alternate date
const X_AMZ_DATE_LOWER: &str = "x-amz-date";

/// Query parameter for delivering the session token
const X_AMZ_SECURITY_TOKEN: &str = "X-Amz-Security-Token";

/// Header for delivering the session token
const X_AMZ_SECURITY_TOKEN_LOWER: &str = "x-amz-security-token";

/// Query parameter for delivering the signature
const X_AMZ_SIGNATURE: &str = "X-Amz-Signature";

/// Query parameter specifying the signed headers
const X_AMZ_SIGNEDHEADERS: &str = "X-Amz-SignedHeaders";

/// Length of a SigV4 signature string.
const SIGV4_SIGNATURE_LEN: usize = 64;


pub(crate) fn get_date(headers: &HeaderMap, query: Vec<ValueField>) -> Result<SystemTime, SignatureError> {
    // It turns out that unrolling this logic is the most straightforward way to return sensible error messages.

    match get_query_param_one(query, X_AMZ_DATE) {
        Ok(date_str) => parse_date_str(
            &date_str,
            SignatureError::MalformedParameter {
                message: "X-Amz-Date is not a valid timestamp".to_string(),
            },
        ),
        Err(e) => match e {
            SignatureError::MissingParameter {
                ..
            } => match get_header_one(headers, X_AMZ_DATE_LOWER) {
                Ok(date_str) => parse_date_str(
                    &date_str,
                    SignatureError::MalformedHeader {
                        message: "X-Amz-Date is not a valid timestamp".to_string(),
                    },
                ),
                Err(e) => match e {
                    SignatureError::MissingHeader {
                        ..
                    } => match get_header_one(headers, DATE) {
                        Ok(date_str) => parse_date_str(
                            &date_str,
                            SignatureError::MalformedHeader {
                                message: "Date is not a valid timestamp".to_string(),
                            },
                        ),
                        Err(e) => Err(e),
                    },
                    _ => Err(e),
                },
            },
            _ => Err(e),
        },
    }
}



/// Retrieve a header value, requiring exactly one value be present.
///
/// # Errors
/// If the header is missing, a `SignatureError::MissingHeader` error is returned.
///
/// If the header contains multiple values, a `SignatureError::MultipleHeaderValues` error is returned.
///
/// If the header value is not valid UTF-8, a `SignatureError::MalformedHeader` error is returned.
pub(crate) fn get_header_one(headers: &HeaderMap, header: &str) -> Result<String, SignatureError> {
    let mut iter = headers.get(header);
    match iter.next() {
        None => Err(SignatureError::MissingHeader {
            header: header.to_string(),
        }),
        Some(value) => match iter.next() {
            None => match from_utf8(value.as_bytes()) {
                Ok(ref s) => Ok(s.to_string()),
                Err(_) => Err(SignatureError::MalformedHeader {
                    message: format!("{} cannot does not contain valid UTF-8", header),
                }),
            },
            Some(_) => Err(SignatureError::MultipleHeaderValues {
                header: header.to_string(),
            }),
        },
    }
}


/// Normalize the query parameters by normalizing the keys and values of each parameter and return a `HashMap` mapping
/// each key to a _vector_ of values (since it is valid for a query parameters to appear multiple times).
///
/// The order of the values matches the order that they appeared in the query string -- this is important for SigV4
/// validation.
///
/// Keys and values are normalized according to the rules of [`normalize_uri_path_component`].
///
/// # Errors
/// If any key or value cannot be normalized as a URI path component, a `SignatureError::InvalidURIPath` error is
/// returned.
///
/// # Example
/// ```rust
/// # use scratchstack_aws_signature::normalize_query_parameters;
/// let result = normalize_query_parameters("a=1&b=2&a=3&c=4").unwrap();
///
/// assert_eq!(result.get("a").unwrap(), &vec!["1", "3"]);
/// assert_eq!(result.get("b").unwrap(), &vec!["2"]);
/// assert_eq!(result.get("c").unwrap(), &vec!["4"]);
/// ```
pub fn get_query_parameters(query: Vec<ValueField>) -> Result<HashMap<String, Vec<String>>, SignatureError> {
    if query.is_empty() {
        return Ok(HashMap::new());
    }

    let mut result = HashMap::<String, Vec<String>>::new();

    for component in query {
        // Normalize the key and value.
        let norm_key = normalize_uri_path_component(component.name.as_name().as_str())?;
        let norm_value = normalize_uri_path_component(component.value)?;

        // If we already have a value for this key, append to it; otherwise, create a new vector containing the value.
        if let Some(result_value) = result.get_mut(&norm_key) {
            result_value.push(norm_value);
        } else {
            result.insert(norm_key, vec![norm_value]);
        }
    }

    Ok(result)
}

/// Retrieve a query parameter, requiring exactly one value be present.
pub(crate) fn get_query_param_one(query: Vec<ValueField>, parameter: &str) -> Result<String, SignatureError> {
    match get_query_parameters(query)?.get(parameter) {
        None => Err(SignatureError::MissingParameter {
            parameter: parameter.to_string(),
        }),
        Some(values) => match values.len() {
            0 => Err(SignatureError::MissingParameter {
                parameter: parameter.to_string(),
            }),
            1 => Ok(values[0].to_string()),
            _ => Err(SignatureError::MultipleParameterValues {
                parameter: parameter.to_string(),
            }),
        },
    }
}
