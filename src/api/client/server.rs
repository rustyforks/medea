//! HTTP server for handling WebSocket connections of Client API.

use actix_web::{
    http, middleware, server, ws, App, AsyncResponder, FutureResponse,
    HttpRequest, HttpResponse, Path, State,
};
use futures::{future, Future};
use serde::Deserialize;

use crate::{
    api::client::{GetMember, Id as RoomID, RoomsRepository, WsSession},
    log::prelude::*,
};

/// Contains [`Room`] ID and [`Member`] credentials obtained from request path.
#[derive(Debug, Deserialize)]
struct RequestInfo {
    room_id: RoomID,
    credentials: String,
}

/// Handles all HTTP requests, performs WebSocket handshake (upgrade) and starts
/// new [`WsSessions`] actor for WebSocket connection.
fn ws_index(
    (r, info, state): (
        HttpRequest<AppContext>,
        Path<RequestInfo>,
        State<AppContext>,
    ),
) -> FutureResponse<HttpResponse> {
    debug!("{:?}", info);

    match state.rooms.get(info.room_id) {
        Some(room_addr) => room_addr
            .send(GetMember {
                credentials: info.credentials.clone(),
            })
            .from_err()
            .and_then(move |res| match res {
                Some(member) => ws::start(
                    &r.drop_state(),
                    WsSession::new(member.id, room_addr),
                ),
                None => Ok(HttpResponse::NotFound().into()),
            })
            .responder(),
        None => future::lazy(move || Ok(HttpResponse::NotFound().into()))
            .responder(),
    }
}

/// Context for [`App`] which holds all the necessary dependencies.
pub struct AppContext {
    /// Repository of all currently existing [`Room`]s in application.
    pub rooms: RoomsRepository,
}

/// Starts HTTP server for handling WebSocket connections.
pub fn run(rooms: RoomsRepository) {
    server::new(move || {
        App::with_state(AppContext {
            rooms: rooms.clone(),
        })
        .middleware(middleware::Logger::default())
        .resource("/ws/{room_id}/{credentials}", |r| {
            r.method(http::Method::GET).with(ws_index)
        })
    })
    .bind("0.0.0.0:8080")
    .unwrap()
    .start();

    info!("Started HTTP server on 0.0.0.0:8080");
}

#[cfg(test)]
mod test {
    use std::{ops::Add, thread, time::Duration};

    use actix::prelude::*;
    use actix_web::{http, test, App};
    use futures::Stream;

    use crate::api::{
        client::{session, Room},
        control::Member,
    };

    use super::*;

    fn start_room() -> RoomsRepository {
        let members = hashmap! {
            1 => Member{id: 1, credentials: "caller_credentials".to_owned()},
            2 => Member{id: 2, credentials: "responder_credentials".to_owned()},
        };
        let room = Arbiter::start(move |_| Room::new(1, members));
        let rooms = hashmap! {1 => room};
        RoomsRepository::new(rooms)
    }

    #[test]
    fn responses_with_pong() {
        let mut srv = test::TestServer::with_factory(move || {
            let repo = start_room();
            App::with_state(AppContext { rooms: repo })
                .resource("/ws/{room_id}/{credentials}", |r| {
                    r.method(http::Method::GET).with(ws_index)
                })
        });
        let (reader, mut writer) =
            srv.ws_at("/ws/1/caller_credentials").unwrap();

        writer.text(r#"{"ping":33}"#);
        let (item, _reader) = srv.execute(reader.into_future()).unwrap();
        assert_eq!(item, Some(ws::Message::Text(r#"{"pong":33}"#.to_owned())));
    }

    #[test]
    fn disconnects_on_idle() {
        let mut srv = test::TestServer::with_factory(move || {
            let repo = start_room();
            App::with_state(AppContext { rooms: repo })
                .resource("/ws/{room_id}/{credentials}", |r| {
                    r.method(http::Method::GET).with(ws_index)
                })
        });
        let (reader, mut writer) =
            srv.ws_at("/ws/1/caller_credentials").unwrap();

        writer.text(r#"{"ping":33}"#);
        let (item, reader) = srv.execute(reader.into_future()).unwrap();
        assert_eq!(item, Some(ws::Message::Text(r#"{"pong":33}"#.to_owned())));

        thread::sleep(session::CLIENT_IDLE_TIMEOUT.add(Duration::from_secs(1)));

        let (item, _) = srv.execute(reader.into_future()).unwrap();
        assert_eq!(
            item,
            Some(ws::Message::Close(Some(ws::CloseCode::Normal.into())))
        );
    }
}
