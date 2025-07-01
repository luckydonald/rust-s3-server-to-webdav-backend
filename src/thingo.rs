use std::{
    io,
    pin::Pin,
    str::{FromStr, Utf8Error},
    task::{ready, Context, Poll},
};

use memchr::{memchr, memmem::find, memrchr};
use rocket::{
    data::{DataStream, FromData, Outcome, ToByteUnit},
    futures::StreamExt,
    http::{ContentType, Header, HeaderMap, Status},
    tokio::io::{AsyncBufRead, AsyncRead, ReadBuf},
    Data, Request,
};
use thiserror::Error;
use tokio_util::{
    bytes::{BufMut, Bytes, BytesMut},
    codec::{Decoder, FramedRead},
};

type Result<T, E = Error> = std::result::Result<T, E>;

// list of multiple strings
const HEADER_BLACKLIST: [&str; 23] = [
    "cache-control",
    "content-type",
    "content-length",
    "expect",
    "max-forwards",
    "pragma",
    "range",
    "te",
    "if-match",
    "if-none-match",
    "if-modified-since",
    "if-unmodified-since",
    "if-range",
    "accept",
    "authorization",
    "proxy-authorization",
    "from",
    "referer",
    "user-agent",
    "X-Amz-User-Agent",
    "x-amzn-trace-id",
    "aws-sdk-invocation-id",
    "aws-sdk-retry",
];

/// Error returned by `MultipartReader`
#[derive(Debug, Error)]
pub enum Error {
    /// An underlying IO error
    #[error(transparent)]
    Io(#[from] io::Error),
    /// A header was not utf8 encoded
    #[error(transparent)]
    Encoding(#[from] Utf8Error),
    /// An error from `serde_json`
    ///
    /// Only available on `json` feature
    #[cfg(feature = "json")]
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The content-type of a multipart stream did not specify a boundary
    #[error("The content type of a multipart stream must specify a boundary")]
    BoundaryNotSpecified,
}


const CHUNK_SIZE: usize = 1024;

/// A data guard for `multipart/*` data. Provides async reading of the
/// individual multipart sections.
///
/// # Example
///
/// ```rust,no_run
/// # use rocket::{post, tokio::io::AsyncReadExt};
/// # use rocket_multipart::MultipartReader;
/// #[post("/mixed", data = "<mixed>")]
/// async fn multipart_data(mut mixed: MultipartReader<'_>) -> String {
///     while let Some(mut a) = mixed.next().await.unwrap() {
///         if let Some(ct) = a.headers().get_one("Content-Type") {
///             // Check content_type
///         }
///         let mut buf = vec![];
///         a.read_to_end(&mut buf).await.unwrap();
///         // Use section's body
///     }
/// #   String::new()
/// }
/// ```
///
/// # Limits
///
/// Like most data guards, `MultipartReader` provides a configurable limit. It
/// uses the `file/multipart` limit or a default limit of 1 MiB. This is the
/// limit for the entire stream, not individual sections.
pub struct MultipartReader<'r> {
    stream: FramedRead<DataStream<'r>, MultipartDecoder<'r>>,
    buffer: MultipartFrame,
    content_type: &'r ContentType,
}

impl<'r> MultipartReader<'r> {
    /// Gets the next section from this multipart reader. The returned section
    /// mutably borrows from `self`, so it must be dropped before another secton
    /// can be read.
    pub async fn next(&mut self) -> Result<Option<MultipartReadSection<'r, '_>>> {
        while self.buffer != MultipartFrame::Boundary {
            match self.stream.next().await {
                Some(Ok(MultipartFrame::End)) | None => {
                    self.buffer = MultipartFrame::End;
                    return Ok(None);
                }
                Some(Ok(val)) => self.buffer = val,
                Some(Err(e)) => return Err(e),
            }
        }

        let mut headers = HeaderMap::new();
        loop {
            match self.stream.next().await {
                Some(Ok(MultipartFrame::End)) | None => {
                    self.buffer = MultipartFrame::End;
                    if headers.is_empty() {
                        return Ok(None);
                    } else {
                        break;
                    }
                }
                Some(Ok(MultipartFrame::Header(header))) => headers.add(header),
                Some(Ok(MultipartFrame::Boundary)) => {
                    self.buffer = MultipartFrame::Boundary;
                    break;
                }
                Some(Ok(val @ MultipartFrame::Data(_))) => {
                    self.buffer = val;
                    break;
                }
                // Some(Ok(val)) => self.buffer = val,
                Some(Err(e)) => return Err(e),
            }
        }
        Ok(Some(MultipartReadSection {
            headers,
            reader: self,
        }))
    }

    /// The content type of the multipart stream as a whole. The primary type is always `multipart`
    pub fn content_type(&self) -> &'r ContentType {
        self.content_type
    }
}

pub fn parse_request_headers(
    headers: &HeaderMap,
) -> Result<HeaderMap, Error> {
    let mut parsed_headers = HeaderMap::new();
    for header in headers.iter() {
        if HEADER_BLACKLIST.contains(&header.name().as_str()) {
            continue;
        }
        // Check if the header is valid UTF-8
        let value = header.value().to_str().map_err(Error::Encoding)?;
        parsed_headers.insert(header.name().clone(), Header::new(header.name(), value));
    }
    Ok(parsed_headers)
}

#[rocket::async_trait]
impl<'r> FromData<'r> for MultipartReader<'r> {
    type Error = Error;

    fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        // Outcome::Forward((data, Status::BadRequest))
        let limit = req
            .rocket()
            .config()
            .limits
            .get("auth/aws_sigv4/body")
            .unwrap_or(5.gibibyte() + 4458);
        if let Some(boundary) = content_type.param("boundary") {
            let read = data.open(limit);

            /*
            #Create a Canonical Request
            CanonicalRequest =
              HTTPRequestMethod + '\n' +
              CanonicalURI + '\n' +
              CanonicalQueryString + '\n' +
              CanonicalHeaders + '\n' +
              SignedHeaders + '\n' +
              HexEncode(Hash(RequestPayload))
             */
            let canonical_request = format!(
                "{}\n{}\n{}\n{}\n{}\n{}",
                req.method(),
                req.uri().path(),
                req.uri().query().unwrap_or(""),
                req.headers()
                    .iter()
                    .map(|h| format!("{}:{}", h.name(), h.value()))
                    .collect::<Vec<_>>()
                    .join("\n"),
                req.headers()
                    .iter()
                    .map(|h| h.name())
                    .collect::<Vec<_>>()
                    .join(";"),
                "hex_encoded_payload_placeholder" // Placeholder for actual payload hash
            );

            Outcome::Success(Self {
                stream: FramedRead::new(
                    data.open(limit),
                    MultipartDecoder {
                        state: MultipartDecoderState::BeforeFirstBoundary,
                        boundary,
                    },
                ),
                buffer: MultipartFrame::Data(BytesMut::new()),
                content_type,
            })
        } else {
            Outcome::Error((Status::BadRequest, Error::BoundaryNotSpecified))
        }
    }
}