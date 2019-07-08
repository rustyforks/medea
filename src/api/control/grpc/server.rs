use std::sync::Arc;

use actix::{Actor, Addr, Arbiter, Context};
use futures::future::Future;
use grpcio::{Environment, RpcContext, Server, ServerBuilder, UnarySink};

use crate::{
    api::control::{
        grpc::protos::control::{
            ApplyRequest, CreateRequest, GetResponse, IdRequest, Response,
        },
        RoomId, RoomSpec,
    },
    log::prelude::*,
    signalling::room_repo::RoomsRepository,
    App,
};

use super::protos::control_grpc::{create_control_api, ControlApi};
use crate::{
    api::control::grpc::protos::control::Error,
    signalling::room_repo::StartRoom,
};
use futures::future::Either;
use std::collections::HashMap;

#[derive(Clone)]
struct ControlApiService {
    room_repository: Addr<RoomsRepository>,
    app: Arc<App>,
}

impl ControlApi for ControlApiService {
    fn create(
        &mut self,
        ctx: RpcContext,
        req: CreateRequest,
        sink: UnarySink<Response>,
    ) {
        // TODO
        let room_id = RoomId(req.get_id().to_string());

        // TODO
        let room = RoomSpec::try_from_protobuf(room_id.clone(), req.get_room())
            .unwrap();

        //        let sid: HashMap<String, String> = msg
        //            .room
        //            .members()
        //            .iter()
        //            .map(|(id, member)| {
        //                let addr = &self.app.config.server.bind_ip;
        //                let port = self.app.config.server.bind_port;
        //                let base_uri = format!("{}:{}", addr, port);
        //
        //                let uri = format!(
        //                    "wss://{}/{}/{}/{}",
        //                    base_uri,
        //                    &room_id,
        //                    id,
        //                    member.credentials()
        //                );
        //
        //                (id.clone().to_string(), uri)
        //            })
        //            .collect();

        ctx.spawn(
            self.room_repository
                .send(StartRoom(room_id, room))
                .map_err(|e| error!("Start room mailbox error. {:?}", e))
                .and_then(move |r| {
                    if r.is_ok() {
                        let mut res = Response::new();
                        res.set_sid(HashMap::new());
                        Either::A(sink.success(res).map_err(|_| ()))
                    } else {
                        let mut res = Response::new();
                        let mut error = Error::new();
                        error.set_status(500);
                        error.set_code(500);
                        error.set_text(String::new());
                        error.set_element(String::new());
                        res.set_error(error);
                        Either::B(sink.success(res).map_err(|_| ()))
                    }
                }),
        );

        //        self.room_repository.add(room_id, room);

        // debug!("{:?}", self.room_repository);
    }

    fn apply(
        &mut self,
        _ctx: RpcContext,
        _req: ApplyRequest,
        _sink: UnarySink<Response>,
    ) {
        unimplemented!()
    }

    fn delete(
        &mut self,
        _ctx: RpcContext,
        _req: IdRequest,
        _sink: UnarySink<Response>,
    ) {
        //        for id in req.get_id() {
        //            let uri = parse_local_uri(id).unwrap(); // TODO
        //            if uri.is_room_id() {
        //                self.room_repository
        //                    .do_send(DeleteRoom(uri.room_id.unwrap()))
        //            }
        //        }
        //
        //        let mut resp = Response::new();
        //        resp.set_sid(HashMap::new());
        //        ctx.spawn(sink.success(resp).map_err(|_| ()));
    }

    fn get(
        &mut self,
        _ctx: RpcContext,
        _req: IdRequest,
        _sink: UnarySink<GetResponse>,
    ) {
        unimplemented!()
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct GrpcServer {
    server: Server,
}

impl Actor for GrpcServer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.server.start();
        debug!("gRPC server started.");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("Shutdown gRPC.");
        self.server.shutdown().wait().unwrap();
    }
}

pub fn run(
    room_repo: Addr<RoomsRepository>,
    app: Arc<App>,
) -> Addr<GrpcServer> {
    let bind_ip = app.config.grpc.bind_ip.clone().to_string();
    let bind_port = app.config.grpc.bind_port;
    let cq_count = app.config.grpc.completion_queue_count;

    let service = create_control_api(ControlApiService {
        app: app,
        room_repository: room_repo,
    });
    let env = Arc::new(Environment::new(cq_count));

    info!("Starting gRPC server on {}:{}", bind_ip, bind_port);

    let server = ServerBuilder::new(env)
        .register_service(service)
        .bind(bind_ip, bind_port)
        .build()
        .unwrap();

    GrpcServer::start_in_arbiter(&Arbiter::new(), move |_| GrpcServer {
        server,
    })
}