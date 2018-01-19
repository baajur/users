extern crate hyper;
extern crate serde_json;
extern crate tokio_core;
extern crate users_lib;
extern crate futures;

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::net::TcpListener;
use std::io::{Read, Write};
use std::str::from_utf8;

use hyper::Method;
use tokio_core::reactor::Core;
use futures::{Future, Stream};
use futures::sync::oneshot;

use users_lib::client::Client;
use users_lib::settings::Settings;
use users_lib::responses::status::StatusMessage;
use users_lib::error::Error;

#[test]
fn test_request() {
    let addr = "127.0.0.1:1234";
    let server = TcpListener::bind(addr).unwrap();
    let mut core = Core::new().unwrap();
    let handle = Arc::new(core.handle());
    let (tx, rx) = oneshot::channel();
    let thread = thread::Builder::new().name(format!("tcp-server<StatusMessage>"));
    thread
        .spawn(move || {
            let mut inc = server.accept().unwrap().0;
            inc.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            inc.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut buf = [0; 4096];
            let mut n = 0;

            let message = StatusMessage::new("OK");
            let message_str = serde_json::to_string(&message).unwrap();

            while n < buf.len() && n < message_str.len() {
                n += match inc.read(&mut buf[n..]) {
                    Ok(n) => n,
                    Err(e) => panic!(
                        "failed to read request, partially read = {:?}, error: {}",
                        from_utf8(&buf[..n]).unwrap(),
                        e
                    ),
                };
            }

            let out = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                message_str.len(),
                message_str
            );
            inc.write_all(out.as_ref()).unwrap();
            let _ = tx.send(());
        })
        .unwrap();

    let settings = Settings::new().unwrap();
    let client = Client::new(&settings, &handle);
    let client_handle = client.handle();
    let client_stream = client.stream();
    handle.spawn(client_stream.for_each(|_| Ok(())));
    let res =
        client_handle.request::<StatusMessage>(Method::Get, format!("http://{}", addr), None, None);
    let rx = rx.map_err(|e| Error::BadRequest(e.to_string()));
    let work = res.join(rx).map(|r| r.0);
    let result = core.run(work).unwrap();

    assert_eq!(result.status, "OK");
}
