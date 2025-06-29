use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Data, Request, Response};
use ubyte::ToByteUnit;

/// Fairing for timing requests.
pub struct HmacChecker;

/// Value stored in request-local state.
#[derive(Copy, Clone)]
struct HmacCheckResult(bool);

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
        // Peek at the first 512 bytes of the request body, if that's already enough.
        if data.peek_complete() {
            payload = <&[u8; 0]>::try_from(data.peek(512).await).unwrap();
        } else { 
            // If the body is not complete, we can only peek at the first 512 bytes.
            let stream = data.open(5.gibibytes());
            payload = stream.into_bytes()
                .await
                .unwrap_or_else(|_| b"")
            ;
            
        }
        
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