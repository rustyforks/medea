use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{IceCandidate, IceServer, PeerId, TrackId, TrackPatch};

use crate::snapshots::track::{TrackSnapshot, TrackSnapshotAccessor};

#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct PeerSnapshot {
    pub id: PeerId,
    pub sdp_offer: Option<String>,
    pub sdp_answer: Option<String>,
    pub tracks: HashMap<TrackId, TrackSnapshot>,
    pub ice_servers: HashSet<IceServer>,
    pub is_force_relayed: bool,
    pub ice_candidates: HashSet<IceCandidate>,
}

pub trait PeerSnapshotAccessor {
    type Track: TrackSnapshotAccessor;

    fn new(
        id: PeerId,
        sdp_offer: Option<String>,
        ice_servers: HashSet<IceServer>,
        is_force_relayed: bool,
        tracks: HashMap<TrackId, Self::Track>,
    ) -> Self;

    fn set_sdp_answer(&mut self, sdp_answer: String);

    fn add_ice_candidate(&mut self, ice_candidate: IceCandidate);

    fn update_track<F>(&mut self, track_id: TrackId, update_fn: F)
    where
        F: FnOnce(Option<&mut Self::Track>);

    fn update_tracks(&mut self, patches: Vec<TrackPatch>) {
        for patch in patches {
            self.update_track(patch.id, |track| {
                if let Some(track) = track {
                    track.update(patch);
                }
            });
        }
    }

    fn update_snapshot(&mut self, snapshot: PeerSnapshot);
}

impl PeerSnapshotAccessor for PeerSnapshot {
    type Track = TrackSnapshot;

    fn new(
        id: PeerId,
        sdp_offer: Option<String>,
        ice_servers: HashSet<IceServer>,
        is_force_relayed: bool,
        tracks: HashMap<TrackId, Self::Track>,
    ) -> Self {
        Self {
            id,
            sdp_offer,
            ice_servers,
            is_force_relayed,
            tracks,
            sdp_answer: None,
            ice_candidates: HashSet::new(),
        }
    }

    fn set_sdp_answer(&mut self, sdp_answer: String) {
        self.sdp_answer = Some(sdp_answer);
    }

    fn add_ice_candidate(&mut self, ice_candidate: IceCandidate) {
        self.ice_candidates.insert(ice_candidate);
    }

    fn update_track<F>(&mut self, track_id: TrackId, update_fn: F)
    where
        F: FnOnce(Option<&mut Self::Track>),
    {
        (update_fn)(self.tracks.get_mut(&track_id));
    }

    fn update_snapshot(&mut self, snapshot: PeerSnapshot) {
        self.ice_servers = snapshot.ice_servers;
        self.sdp_offer = snapshot.sdp_offer;
        self.sdp_answer = snapshot.sdp_answer;
        self.ice_candidates = snapshot.ice_candidates;
        self.is_force_relayed = snapshot.is_force_relayed;

        for (track_id, track_snapshot) in snapshot.tracks {
            if let Some(track) = self.tracks.get_mut(&track_id) {
                track.update_snapshot(track_snapshot);
            }
        }
    }
}