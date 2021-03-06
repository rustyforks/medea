//! Repository that stores [`Room`]s addresses.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use actix::Addr;
use medea_client_api_proto::RoomId;

use crate::{
    api::{client::RpcServerRepository, RpcServer},
    signalling::Room,
};

/// Repository that stores [`Room`]s addresses.
#[derive(Clone, Debug, Default)]
pub struct RoomRepository {
    // TODO: Use crossbeam's concurrent hashmap when its done.
    //       [Tracking](https://github.com/crossbeam-rs/rfcs/issues/32),
    //       or [ConcurrentHashMap port](https://github.com/jonhoo/flurry)
    //       when its done.
    rooms: Arc<Mutex<HashMap<RoomId, Addr<Room>>>>,
}

impl RoomRepository {
    /// Creates new [`Room`]s repository with passed-in [`Room`]s.
    pub fn new(rooms: HashMap<RoomId, Addr<Room>>) -> Self {
        Self {
            rooms: Arc::new(Mutex::new(rooms)),
        }
    }

    /// Returns [`Room`] by its ID.
    pub fn get(&self, id: &RoomId) -> Option<Addr<Room>> {
        let rooms = self.rooms.lock().unwrap();
        rooms.get(id).cloned()
    }

    /// Removes [`Room`] from [`RoomRepository`] by [`RoomId`].
    pub fn remove(&self, id: &RoomId) {
        self.rooms.lock().unwrap().remove(id);
    }

    /// Adds new [`Room`] into [`RoomRepository`].
    pub fn add(&self, id: RoomId, room: Addr<Room>) {
        self.rooms.lock().unwrap().insert(id, room);
    }

    /// Checks existence of [`Room`] in [`RoomRepository`] by provided
    /// [`RoomId`].
    pub fn contains_room_with_id(&self, id: &RoomId) -> bool {
        self.rooms.lock().unwrap().contains_key(id)
    }
}

impl RpcServerRepository for RoomRepository {
    #[inline]
    fn get(&self, room_id: &RoomId) -> Option<Box<dyn RpcServer>> {
        self.get(room_id).map(|r| Box::new(r) as Box<dyn RpcServer>)
    }
}
