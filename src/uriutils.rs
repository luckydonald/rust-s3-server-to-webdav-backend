use rocket::Request as RocketRequest;
use http::{Request as HttpRequest, Method};

pub(crate) fn convert_rocket_request_to_http(rocket_request: &RocketRequest) -> HttpRequest<Vec<u8>> {
    // Extract the HTTP method
    let method = match rocket_request.method().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        "OPTIONS" => Method::OPTIONS,
        "HEAD" => Method::HEAD,
        _ => Method::GET, // Default to GET if method is unknown
    };

    // Extract the host and scheme
    let host = rocket_request.host().expect("should have host").to_string();
    // let scheme = rocket_request.segments().scheme.unwrap_or("http").to_string();

    // Construct the full URI
    // let uri = format!("{}://{}/{}", scheme, host, rocket_request.uri().path());
    let uri = rocket_request.uri().to_string();

    // Create a new HTTP request
    let mut http_request = HttpRequest::builder()
        .method(method)
        .uri(host);

    // Copy headers from Rocket request to HTTP request
    for header in rocket_request.headers().iter() {
        let key = header.name().as_str();
        let value = header.value();
        http_request = http_request.header(key, value);
    }

    // If you need to handle the body, you can do so here
    // For example, if the body is in JSON format:
    let body = rocket_request.body().data().await.unwrap();
    let http_request = http_request.body(body);
    http_request;
    
    // Build and return the HTTP request
    // http_request.body(Vec::new()).unwrap() // Replace Vec::new() with actual body if needed
}
