use std::ops::{Deref, DerefMut};
use aws_sigv4::sign::v4::generate_signing_key as generate_signing_key_v4;
#[cfg(feature = "sig4a")]
use aws_sigv4::sign::v4a::generate_signing_key as generate_signing_key_v4a;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Data, Request, Response};
use rocket::data::ToByteUnit;
use rocket::http::HeaderMap;
use crate::date_utils::parse_date_str;
use crate::environment::Config;
use crate::header_utils::{get_date, get_header_one};

/// Fairing for timing requests.
pub struct HmacChecker;

/// Value stored in request-local state.
struct HmacCheckResult(str);

#[rocket::async_trait]
impl Fairing for HmacChecker {
    fn info(&self) -> Info {
        Info {
            name: "HMAC Checker",
            kind: Kind::Request,
        }
    }

    /// Stores the start time of the request in request-local state.
    async fn on_request(&self, request: &mut Request<'_>, data: &mut Data<'_>) {
        // Store a `HmacCheckResult` instead of directly storing a `SystemTime`
        // to ensure that this usage doesn't conflict with anything else
        // that might store a `SystemTime` in request-local cache.
        let mut payload = b"";
        let date = get_date(
            request.headers(),
            request.query_fields().collect::<Vec<_>>(),
        ).unwrap();
        // Because rocket is a dumbfuck, we can only parse the first 512 bytes AT ALL.
        // Amazon specs that the body can be up to 5GB, so I guess fuck you, rocket.
        // Also of cause that 512 is not configurable,
        // and to put oil into the fire that constant is private.
        let body = data.peek(usize::MAX).await;

        let env = Config::from_env().expect("Failed to load config from environment");
        let v4_expected = generate_signing_key_v4(
            env.aws_secret_key.as_str(),
            date,
            env.aws_region.as_str(),
            env.aws_service.as_str(),
        );
        #[cfg(feature = "sig4a")]
        let v4a_expected = generate_signing_key_v4a(
            env.aws_secret_key.as_str(),
            time,
            env.aws_region.as_str(),
            "aws4_request",
        );

        calculate_string_to_sign()
        
        request.local_cache(|| HmacCheckResult(true));
    }

    /// Adds a header to the response indicating how long the server took to
    /// process the request.
    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let start_time = req.local_cache(|| HmacCheckResult(false));
        if let Some(Ok(duration)) = start_time.0.map(|st| st.elapsed()) {
            let ms = duration.as_secs() * 1000 + duration.subsec_millis() as u64;
            res.set_raw_header("X-Response-Time", format!("{} ms", ms));
        }
    }
}