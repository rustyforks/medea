//! Service which provides CRUD actions for [`Room`].

use std::{collections::HashMap, marker::PhantomData};

use actix::{
    Actor, Addr, Context, Handler, MailboxError, Message, ResponseFuture,
};
use derive_more::Display;
use failure::Fail;
use futures::future::{self, Future};
use medea_control_api_proto::grpc::control_api::Element as ElementProto;

use crate::{
    api::control::{
        endpoints::EndpointSpec,
        load_static_specs_from_dir,
        local_uri::{LocalUri, StatefulLocalUri, ToEndpoint, ToMember, ToRoom},
        LoadStaticControlSpecsError, MemberSpec, RoomId, RoomSpec,
    },
    log::prelude::*,
    shutdown::{self, GracefulShutdown},
    signalling::{
        room::{
            Close, CreateEndpoint, CreateMember, Delete, RoomError,
            SerializeProto,
        },
        room_repo::RoomRepository,
        Room,
    },
    AppContext,
};

/// Errors of [`RoomService`].
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Fail, Display)]
pub enum RoomServiceError {
    /// [`Room`] not found in [`RoomRepository`].
    #[display(fmt = "Room [id = {}] not found.", _0)]
    RoomNotFound(LocalUri<ToRoom>),

    /// Wrapper for [`Room`]'s [`MailboxError`].
    #[display(fmt = "Room mailbox error: {:?}", _0)]
    RoomMailboxErr(MailboxError),

    /// Try to create [`Room`] with [`RoomId`] which already exists in
    /// [`RoomRepository`].
    #[display(fmt = "Room [id = {}] already exists.", _0)]
    RoomAlreadyExists(LocalUri<ToRoom>),

    /// Some error happened in [`Room`].
    ///
    /// For more info read [`RoomError`] docs.
    #[display(fmt = "{}", _0)]
    RoomError(RoomError),

    /// Error which can happen while loading static [Control API] specs.
    ///
    /// [Control API]: http://tiny.cc/380uaz
    #[display(fmt = "Failed to load static specs. {:?}", _0)]
    FailedToLoadStaticSpecs(LoadStaticControlSpecsError),

    /// Provided empty [`LocalUri`] list.
    #[display(fmt = "Empty URIs list.")]
    EmptyUrisList,

    /// Provided not the same [`RoomId`]s in [`LocalUri`] list.
    ///
    /// Atm this error can happen in `Delete` method because `Delete` should be
    /// called only for one [`Room`].
    #[display(
        fmt = "Provided not the same Room IDs in elements IDs [ids = {:?}].",
        _1
    )]
    NotSameRoomIds(RoomId, RoomId),
}

impl From<RoomError> for RoomServiceError {
    fn from(err: RoomError) -> Self {
        Self::RoomError(err)
    }
}

impl From<LoadStaticControlSpecsError> for RoomServiceError {
    fn from(err: LoadStaticControlSpecsError) -> Self {
        Self::FailedToLoadStaticSpecs(err)
    }
}

/// Service for controlling [`Room`]s.
pub struct RoomService {
    /// Repository that stores [`Room`]s addresses.
    room_repo: RoomRepository,

    /// Global app context.
    app: AppContext,

    /// Address to [`GracefulShutdown`].
    ///
    /// Use for subscribe newly created [`Room`]s to [`GracefulShutdown`] and
    /// unsubscribe deleted [`Room`]s from [`GracefulShutdown`].
    graceful_shutdown: Addr<GracefulShutdown>,

    /// Path to directory with static [Сontrol API] specs.
    ///
    /// [Control API]: http://tiny.cc/380uaz
    static_specs_dir: String,
}

impl RoomService {
    /// Creates new [`RoomService`].
    pub fn new(
        room_repo: RoomRepository,
        app: AppContext,
        graceful_shutdown: Addr<GracefulShutdown>,
    ) -> Self {
        Self {
            static_specs_dir: app.config.control_api.static_specs_dir.clone(),
            room_repo,
            app,
            graceful_shutdown,
        }
    }

    /// Closes [`Room`] with provided [`RoomId`].
    ///
    /// This is also deletes this [`Room`] from [`RoomRepository`].
    fn close_room(
        &self,
        id: RoomId,
    ) -> Box<dyn Future<Item = (), Error = MailboxError>> {
        if let Some(room) = self.room_repo.get(&id) {
            shutdown::unsubscribe(
                &self.graceful_shutdown,
                room.clone().recipient(),
                shutdown::Priority(2),
            );

            let room_repo = self.room_repo.clone();

            Box::new(room.send(Close).map(move |_| {
                debug!("Room [id = {}] removed.", id);
                room_repo.remove(&id);
            }))
        } else {
            Box::new(futures::future::ok(()))
        }
    }
}

impl Actor for RoomService {
    type Context = Context<Self>;
}

/// Returns [`LocalUri`] pointing to [`Room`].
///
/// __Note__ this function don't check presence of [`Room`] in this
/// [`RoomService`].
fn get_local_uri_to_room(room_id: RoomId) -> LocalUri<ToRoom> {
    LocalUri::<ToRoom>::new(room_id)
}

/// Signal for load all static specs and start [`Room`]s.
#[derive(Message)]
#[rtype(result = "Result<(), RoomServiceError>")]
pub struct StartStaticRooms;

impl Handler<StartStaticRooms> for RoomService {
    type Result = Result<(), RoomServiceError>;

    fn handle(
        &mut self,
        _: StartStaticRooms,
        _: &mut Self::Context,
    ) -> Self::Result {
        let room_specs = load_static_specs_from_dir(&self.static_specs_dir)?;

        for spec in room_specs {
            if self.room_repo.contains_room_with_id(spec.id()) {
                return Err(RoomServiceError::RoomAlreadyExists(
                    get_local_uri_to_room(spec.id),
                ));
            }

            let room_id = spec.id().clone();

            let room = Room::new(&spec, &self.app)?.start();
            shutdown::subscribe(
                &self.graceful_shutdown,
                room.clone().recipient(),
                shutdown::Priority(2),
            );

            self.room_repo.add(room_id, room);
        }
        Ok(())
    }
}

/// Signal for creating new [`Room`].
#[derive(Message)]
#[rtype(result = "Result<(), RoomServiceError>")]
pub struct CreateRoom {
    /// [Control API] spec for [`Room`].
    ///
    /// [Control API]: http://tiny.cc/380uaz
    pub spec: RoomSpec,
}

impl Handler<CreateRoom> for RoomService {
    type Result = Result<(), RoomServiceError>;

    fn handle(
        &mut self,
        msg: CreateRoom,
        _: &mut Self::Context,
    ) -> Self::Result {
        let room_spec = msg.spec;

        if self.room_repo.get(&room_spec.id).is_some() {
            return Err(RoomServiceError::RoomAlreadyExists(
                get_local_uri_to_room(room_spec.id),
            ));
        }

        let room = Room::new(&room_spec, &self.app)?;
        let room_addr = room.start();

        shutdown::subscribe(
            &self.graceful_shutdown,
            room_addr.clone().recipient(),
            shutdown::Priority(2),
        );

        debug!("New Room [id = {}] started.", room_spec.id);
        self.room_repo.add(room_spec.id, room_addr);

        Ok(())
    }
}

/// Signal for create new [`Member`] in [`Room`]
///
/// [`Member`]: crate::signalling::elements::member::Member
#[derive(Message)]
#[rtype(result = "Result<(), RoomServiceError>")]
pub struct CreateMemberInRoom {
    pub uri: LocalUri<ToMember>,
    pub spec: MemberSpec,
}

impl Handler<CreateMemberInRoom> for RoomService {
    type Result = ResponseFuture<(), RoomServiceError>;

    fn handle(
        &mut self,
        msg: CreateMemberInRoom,
        _: &mut Self::Context,
    ) -> Self::Result {
        let (room_id, member_id) = msg.uri.take_all();

        if let Some(room) = self.room_repo.get(&room_id) {
            Box::new(
                room.send(CreateMember(member_id, msg.spec))
                    .map_err(RoomServiceError::RoomMailboxErr)
                    .and_then(|r| r.map_err(RoomServiceError::from)),
            )
        } else {
            Box::new(future::err(RoomServiceError::RoomNotFound(LocalUri::<
                ToRoom,
            >::new(
                room_id
            ))))
        }
    }
}

/// Signal for create new [`Endpoint`] in [`Room`]
///
/// [`Endpoint`]: crate::signalling::elements::endpoints::Endpoint
#[derive(Message)]
#[rtype(result = "Result<(), RoomServiceError>")]
pub struct CreateEndpointInRoom {
    pub uri: LocalUri<ToEndpoint>,
    pub spec: EndpointSpec,
}

impl Handler<CreateEndpointInRoom> for RoomService {
    type Result = ResponseFuture<(), RoomServiceError>;

    fn handle(
        &mut self,
        msg: CreateEndpointInRoom,
        _: &mut Self::Context,
    ) -> Self::Result {
        let (room_id, member_id, endpoint_id) = msg.uri.take_all();

        if let Some(room) = self.room_repo.get(&room_id) {
            Box::new(
                room.send(CreateEndpoint {
                    member_id,
                    endpoint_id,
                    spec: msg.spec,
                })
                .map_err(RoomServiceError::RoomMailboxErr)
                .and_then(|r| r.map_err(RoomServiceError::from)),
            )
        } else {
            Box::new(future::err(RoomServiceError::RoomNotFound(LocalUri::<
                ToRoom,
            >::new(
                room_id
            ))))
        }
    }
}

/// State which indicates that [`DeleteElements`] message was validated and can
/// be send to [`RoomService`].
pub struct Validated;

/// State which indicates that [`DeleteElements`] message is unvalidated and
/// should be validated with `validate()` function of [`DeleteElements`] in
/// [`Unvalidated`] state before sending to [`RoomService`].
pub struct Unvalidated;

// Clippy lint show use_self errors for DeleteElements with generic state. This
// is fix for it. This allow not works on function.
#[allow(clippy::use_self)]
impl DeleteElements<Unvalidated> {
    pub fn new() -> Self {
        Self {
            uris: Vec::new(),
            _validation_state: PhantomData,
        }
    }

    pub fn add_uri(&mut self, uri: StatefulLocalUri) {
        self.uris.push(uri)
    }

    // TODO: tests
    /// Validates request. It must have at least one uri, all uris must share
    /// same [`RoomId`].
    pub fn validate(
        self,
    ) -> Result<DeleteElements<Validated>, RoomServiceError> {
        if self.uris.is_empty() {
            return Err(RoomServiceError::EmptyUrisList);
        }

        let first_room_id = self.uris[0].room_id();

        for id in &self.uris {
            if first_room_id != id.room_id() {
                return Err(RoomServiceError::NotSameRoomIds(
                    first_room_id.clone(),
                    id.room_id().clone(),
                ));
            }
        }

        Ok(DeleteElements {
            uris: self.uris,
            _validation_state: PhantomData,
        })
    }
}

/// Signal for delete [Control API] elements.
///
/// This message can be in two states: [`Validated`] and [`Unvalidated`].
///
/// For ability to send this message to [`RoomService`] [`DeleteElements`]
/// should be in [`Validated`] state. You can go to [`Validated`] state
/// from [`Unvalidated`] with [`DeleteElements::validate`] function
/// which will validate all [`StatefulLocalUri`]s.
///
/// Validation doesn't guarantee that message can't return [`RoomServiceError`].
/// This is just validation for errors which we can catch before sending
/// message.
///
/// [Control API]: http://tiny.cc/380uaz
#[derive(Message, Default)]
#[rtype(result = "Result<(), RoomServiceError>")]
pub struct DeleteElements<T> {
    uris: Vec<StatefulLocalUri>,
    _validation_state: PhantomData<T>,
}

impl Handler<DeleteElements<Validated>> for RoomService {
    type Result = ResponseFuture<(), RoomServiceError>;

    // TODO: delete 'clippy::unnecessary_filter_map` when drain_filter TODO will
    // be resolved.
    #[allow(clippy::if_not_else, clippy::unnecessary_filter_map)]
    fn handle(
        &mut self,
        msg: DeleteElements<Validated>,
        _: &mut Self::Context,
    ) -> Self::Result {
        let mut deletes_from_room: Vec<StatefulLocalUri> = Vec::new();
        // TODO: use Vec::drain_filter when it will be in stable
        let room_messages_futs: Vec<
            Box<dyn Future<Item = (), Error = MailboxError>>,
        > = msg
            .uris
            .into_iter()
            .filter_map(|l| {
                if let StatefulLocalUri::Room(room_id) = l {
                    Some(self.close_room(room_id.take_room_id()))
                } else {
                    deletes_from_room.push(l);
                    None
                }
            })
            .collect();

        if !room_messages_futs.is_empty() {
            Box::new(
                futures::future::join_all(room_messages_futs)
                    .map(|_| ())
                    .map_err(RoomServiceError::RoomMailboxErr),
            )
        } else if !deletes_from_room.is_empty() {
            let room_id = deletes_from_room[0].room_id().clone();

            if let Some(room) = self.room_repo.get(&room_id) {
                Box::new(
                    room.send(Delete(deletes_from_room))
                        .map_err(RoomServiceError::RoomMailboxErr),
                )
            } else {
                Box::new(future::ok(()))
            }
        } else {
            Box::new(future::err(RoomServiceError::EmptyUrisList))
        }
    }
}

/// Message which returns serialized to protobuf objects by provided
/// [`LocalUri`].
#[derive(Message)]
#[rtype(result = "Result<HashMap<StatefulLocalUri, ElementProto>, \
                  RoomServiceError>")]
pub struct Get(pub Vec<StatefulLocalUri>);

impl Handler<Get> for RoomService {
    type Result = ResponseFuture<
        HashMap<StatefulLocalUri, ElementProto>,
        RoomServiceError,
    >;

    fn handle(&mut self, msg: Get, _: &mut Self::Context) -> Self::Result {
        let mut rooms_elements = HashMap::new();
        for uri in msg.0 {
            let room_id = uri.room_id();

            if let Some(room) = self.room_repo.get(room_id) {
                rooms_elements
                    .entry(room)
                    .or_insert_with(Vec::new)
                    .push(uri);
            } else {
                return Box::new(future::err(RoomServiceError::RoomNotFound(
                    uri.into(),
                )));
            }
        }

        let mut futs = Vec::new();
        for (room, elements) in rooms_elements {
            futs.push(room.send(SerializeProto(elements)));
        }

        Box::new(
            futures::future::join_all(futs)
                .map_err(RoomServiceError::RoomMailboxErr)
                .and_then(|results| {
                    let mut all = HashMap::new();
                    for result in results {
                        match result {
                            Ok(res) => all.extend(res),
                            Err(e) => return Err(RoomServiceError::from(e)),
                        }
                    }
                    Ok(all)
                }),
        )
    }
}

#[cfg(test)]
mod delete_elements_validation_specs {
    use std::convert::TryFrom as _;

    use super::*;

    #[test]
    fn empty_uris_list() {
        let elements = DeleteElements::new();
        match elements.validate() {
            Ok(_) => panic!(
                "Validation should fail with EmptyUrisList but returned Ok."
            ),
            Err(e) => match e {
                RoomServiceError::EmptyUrisList => (),
                _ => panic!(
                    "Validation should fail with EmptyList error but errored \
                     with {:?}.",
                    e
                ),
            },
        }
    }

    #[test]
    fn error_if_not_same_room_ids() {
        let mut elements = DeleteElements::new();
        ["local://room_id/member", "local://another_room_id/member"]
            .into_iter()
            .map(|uri| StatefulLocalUri::try_from(uri.to_string()).unwrap())
            .for_each(|uri| elements.add_uri(uri));

        match elements.validate() {
            Ok(_) => panic!(
                "Validation should fail with NotSameRoomIds but returned Ok."
            ),
            Err(e) => match e {
                RoomServiceError::NotSameRoomIds(first, another) => {
                    assert_eq!(&first.to_string(), "room_id");
                    assert_eq!(&another.to_string(), "another_room_id");
                }
                _ => panic!(
                    "Validation should fail with NotSameRoomIds error but \
                     errored with {:?}.",
                    e
                ),
            },
        }
    }

    #[test]
    fn success_if_all_ok() {
        let mut elements = DeleteElements::new();
        [
            "local://room_id/member_id",
            "local://room_id/another_member_id",
            "local://room_id/member_id/endpoint_id",
        ]
        .into_iter()
        .map(|uri| StatefulLocalUri::try_from(uri.to_string()).unwrap())
        .for_each(|uri| elements.add_uri(uri));

        assert!(elements.validate().is_ok());
    }
}
