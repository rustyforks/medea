use std::{cell::Cell, rc::Rc, sync::Arc};

use actix::Actor;
use failure::Error;
use futures::future::Future;
use hashbrown::HashMap;
use medea::{
    api::{client, control::grpc},
    conf::Conf,
    log::{self, prelude::*},
    signalling::{
        room_repo::RoomsRepository,
        room_service::{RoomService, StartStaticRooms},
    },
    turn::new_turn_auth_service,
    App,
};

fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    let logger = log::new_dual_logger(std::io::stdout(), std::io::stderr());
    let _scope_guard = slog_scope::set_global_logger(logger);
    slog_stdlog::init()?;

    let config = Conf::parse()?;
    info!("{:?}", config);

    // This is crutch for existence of gRPC server throughout the all app's
    // lifetime.
    let grpc_addr = Rc::new(Cell::new(None));
    let grpc_addr_clone = Rc::clone(&grpc_addr);

    actix::run(move || {
        new_turn_auth_service(&config.turn)
            .map_err(|e| error!("{:?}", e))
            .map(Arc::new)
            .and_then(move |turn_service| {
                let app = Arc::new(App {
                    config: config.clone(),
                    turn_service,
                });

                let room_repo = RoomsRepository::new(HashMap::new());
                let room_service =
                    RoomService::new(room_repo.clone(), Arc::clone(&app))
                        .start();

                room_service
                    .clone()
                    .send(StartStaticRooms)
                    .map_err(|e| {
                        error!("StartStaticRooms mailbox error: {:?}", e)
                    })
                    .map(|result| {
                        if let Err(e) = result {
                            panic!("{}", e);
                        }
                    })
                    .map(move |_| {
                        let grpc_addr = grpc::server::run(room_service, app);
                        grpc_addr_clone.set(Some(grpc_addr));
                    })
                    .and_then(move |_| {
                        client::server::run(room_repo, config).map_err(|e| {
                            error!("Client server startup error. {:?}", e)
                        })
                    })
            })
    })
    .unwrap();

    Ok(())
}
