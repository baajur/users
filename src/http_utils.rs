use hyper::{StatusCode};
use hyper::mime;
use hyper::header::{ContentLength, ContentType};
use hyper::server::{Request, Response};
use hyper::error::Error;

use futures::future::{Future};
use futures::{future, Stream};

use std::collections::HashMap;

use hyper;
use error;

#[derive(Serialize, Debug)]
pub struct Status {
    pub status: String
}

pub fn status_ok() -> Status {
    Status { status: String::from("OK") }
}

pub fn query_params(query: &str) -> HashMap<&str, &str> {
    let mut params = HashMap::new();
    let pairs: Vec<&str> = query.split("&").collect();

    for pair in pairs {
        let split: Vec<&str> = pair.split("=").collect();
        params.insert(split[0], split[1]);
    }

    params
}

pub fn read_body(request: Request) -> Box<Future<Item=String, Error=hyper::Error>> {
    Box::new(
        request.body()
            .fold(Vec::new(), |mut acc, chunk| {
                acc.extend_from_slice(&*chunk);
                future::ok::<_, hyper::Error>(acc)
            })
            .and_then(|bytes| {
                match String::from_utf8(bytes) {
                    Ok(data) => future::ok(data),
                    Err(err) => future::err(Error::Utf8(err.utf8_error()))
                }
            })
    )
}

pub fn response_with_body(body: String) -> Response {
    Response::new()
        .with_header(ContentLength(body.len() as u64))
        .with_header(ContentType(mime::APPLICATION_JSON))
        .with_status(StatusCode::Ok)
        .with_body(body)
}

pub fn response_with_error(error: error::Error) -> Response {
    use error::Error::*;
    match error {
        Json(err) => response_with_body(err.to_string()).with_status(StatusCode::UnprocessableEntity)
    }
}

pub fn response_not_found() -> Response {
    response_with_body("Not found".to_string()).with_status(StatusCode::NotFound)
}
