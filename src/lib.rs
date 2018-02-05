//! Users is a microservice responsible for authentication and managing user profiles.
//! The layered structure of the app is
//!
//! `Application -> Controller -> Service -> Repo + HttpClient`
//!
//! Each layer can only face exceptions in its base layers and can only expose its own errors.
//! E.g. `Service` layer will only deal with `Repo` and `HttpClient` errors and will only return
//! `ServiceError`. That way Controller will only have to deal with ServiceError, but not with `Repo`
//! or `HttpClient` repo.

extern crate config as config_crate;
extern crate futures;
extern crate futures_cpupool;
extern crate tokio_core;
extern crate hyper;
extern crate regex;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate diesel;
extern crate r2d2;
extern crate r2d2_diesel;
#[macro_use]
extern crate validator_derive;
extern crate validator;
extern crate jsonwebtoken;
extern crate hyper_tls;
extern crate chrono;
extern crate sha3;
extern crate rand;
extern crate base64;


#[macro_use]
pub mod macros;
pub mod app;
pub mod controller;
pub mod models;
pub mod repos;
pub mod services;
pub mod config;
pub mod types;
pub mod http;

use std::sync::Arc;
use std::process;

use futures::{Future, Stream};
use futures::future;
use futures_cpupool::CpuPool;
use hyper::server::Http;
use diesel::pg::PgConnection;
use r2d2_diesel::ConnectionManager;
use tokio_core::reactor::Core;

use app::Application;
use config::Config;
use repos::acl::RolesCacheImpl;


/// Starts new web service from provided `Config`
pub fn start_server(config: Config) {
    // Prepare logger
    env_logger::init().unwrap();

    // Prepare reactor
    let mut core = Core::new().expect("Unexpected error creating event loop core");
    let handle = Arc::new(core.handle());

    let client = http::client::Client::new(&config, &handle);
    let client_handle = client.handle();
    let client_stream = client.stream();
    handle.spawn(
        client_stream.for_each(|_| Ok(()))
    );

    // Prepare server
    let thread_count = config.server.thread_count.clone();
    let address = config.server.address.parse().expect("Address must be set in configuration");


    let serve = Http::new().serve_addr_handle(&address, &handle, move || {
        // Prepare database pool
        let database_url: String = config.server.database.parse().expect("Database URL must be set in configuration");
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let r2d2_pool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create connection pool");

        // Prepare CPU pool
        let cpu_pool = CpuPool::new(thread_count);

        let roles_cache = RolesCacheImpl::new(r2d2_pool.clone(), cpu_pool.clone());

        let controller = controller::Controller::new(r2d2_pool, cpu_pool, client_handle.clone(), config.clone(), roles_cache);

        // Prepare application
        let app = Application {
            controller,
        };

        Ok(app)
    }).unwrap_or_else(|why| {
        error!("Http Server Initialization Error: {}", why);
        process::exit(1);
    });

    let handle_arc2 = handle.clone();
    handle.spawn(
        serve.for_each(move |conn| {
            handle_arc2.spawn(
                conn.map(|_| ())
                    .map_err(|why| error!("Server Error: {:?}", why)),
            );
            Ok(())
        })
        .map_err(|_| ()),
    );

    info!("Listening on http://{}, threads: {}", address, thread_count);
    core.run(future::empty::<(), ()>()).unwrap();
}
