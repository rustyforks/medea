//! Implementation of the [`CommandHandler`] for the [`Room`] and related
//! definitions.

use std::collections::HashMap;

use actix::WrapFuture as _;
use medea_client_api_proto::{
    CommandHandler, Event, IceCandidate, Mid, PeerId, PeerMetrics, TrackId,
    TrackPatch,
};

use crate::{
    log::prelude::*,
    media::{
        Peer, RenegotiationReason, Stable, WaitLocalHaveRemote, WaitLocalSdp,
        WaitRemoteSdp,
    },
};

use super::{ActFuture, Room, RoomError};

impl CommandHandler for Room {
    type Output = Result<ActFuture<Result<(), RoomError>>, RoomError>;

    /// Sends [`Event::PeerCreated`] to provided [`Peer`] partner. Provided
    /// [`Peer`] state must be [`WaitLocalSdp`] and will be changed to
    /// [`WaitRemoteSdp`], partners [`Peer`] state must be [`Stable`] and will
    /// be changed to [`WaitLocalHaveRemote`].
    fn on_make_sdp_offer(
        &mut self,
        from_peer_id: PeerId,
        sdp_offer: String,
        mids: HashMap<TrackId, Mid>,
    ) -> Self::Output {
        let mut from_peer: Peer<WaitLocalSdp> =
            self.peers.take_inner_peer(from_peer_id)?;
        from_peer.set_mids(mids)?;

        let to_peer_id = from_peer.partner_peer_id();
        let to_peer: Peer<Stable> = self.peers.take_inner_peer(to_peer_id)?;

        let from_peer = from_peer.set_local_sdp(sdp_offer.clone());
        let to_peer = to_peer.set_remote_sdp(sdp_offer.clone());

        let to_member_id = to_peer.member_id();
        let ice_servers = to_peer.ice_servers_list().ok_or_else(|| {
            RoomError::NoTurnCredentials(to_member_id.clone())
        })?;

        let event = match from_peer.renegotiation_reason() {
            Some(RenegotiationReason::TracksAdded) => Event::TracksAdded {
                peer_id: to_peer.id(),
                sdp_offer: Some(sdp_offer),
                tracks: to_peer.get_new_tracks(),
            },
            Some(RenegotiationReason::TracksRemoved) => Event::TracksRemoved {
                peer_id: to_peer_id,
                tracks_ids: to_peer.removed_tracks_ids(),
                sdp_offer: Some(sdp_offer),
            },
            None => Event::PeerCreated {
                peer_id: to_peer.id(),
                sdp_offer: Some(sdp_offer),
                tracks: to_peer.get_new_tracks(),
                ice_servers,
                force_relay: to_peer.is_force_relayed(),
            },
        };

        self.peers.add_peer(from_peer);
        self.peers.add_peer(to_peer);

        Ok(Box::new(
            self.members
                .send_event_to_member(to_member_id, event)
                .into_actor(self),
        ))
    }

    /// Sends [`Event::SdpAnswerMade`] to provided [`Peer`] partner. Provided
    /// [`Peer`] state must be [`WaitLocalHaveRemote`] and will be changed to
    /// [`Stable`], partners [`Peer`] state must be [`WaitRemoteSdp`] and will
    /// be changed to [`Stable`].
    fn on_make_sdp_answer(
        &mut self,
        from_peer_id: PeerId,
        sdp_answer: String,
    ) -> Self::Output {
        let from_peer: Peer<WaitLocalHaveRemote> =
            self.peers.take_inner_peer(from_peer_id)?;

        let to_peer_id = from_peer.partner_peer_id();
        let to_peer: Peer<WaitRemoteSdp> =
            self.peers.take_inner_peer(to_peer_id)?;

        let from_peer = from_peer.set_local_sdp(sdp_answer.clone());
        let to_peer = to_peer.set_remote_sdp(&sdp_answer);

        let to_member_id = to_peer.member_id();
        let event = Event::SdpAnswerMade {
            peer_id: to_peer_id,
            sdp_answer,
        };

        self.peers.add_peer(from_peer);
        self.peers.add_peer(to_peer);

        Ok(Box::new(
            self.members
                .send_event_to_member(to_member_id, event)
                .into_actor(self),
        ))
    }

    /// Sends [`Event::IceCandidateDiscovered`] to provided [`Peer`] partner.
    /// Both [`Peer`]s may have any state except [`Stable`].
    fn on_set_ice_candidate(
        &mut self,
        from_peer_id: PeerId,
        candidate: IceCandidate,
    ) -> Self::Output {
        // TODO: add E2E test
        if candidate.candidate.is_empty() {
            warn!("Empty candidate from Peer: {}, ignoring", from_peer_id);
            return Ok(Box::new(actix::fut::ok(())));
        }

        let to_peer_id =
            self.peers.get_peer_by_id(from_peer_id)?.partner_peer_id();
        let to_member_id = self.peers.get_peer_by_id(to_peer_id)?.member_id();
        let event = Event::IceCandidateDiscovered {
            peer_id: to_peer_id,
            candidate,
        };

        Ok(Box::new(
            self.members
                .send_event_to_member(to_member_id, event)
                .into_actor(self),
        ))
    }

    /// Does nothing atm.
    fn on_add_peer_connection_metrics(
        &mut self,
        _: PeerId,
        _: PeerMetrics,
    ) -> Self::Output {
        Ok(Box::new(actix::fut::ok(())))
    }

    /// Sends [`Event::TracksUpdated`] with data from the received
    /// [`Command::UpdateTracks`].
    ///
    /// [`Command::UpdateTracks`]: medea_client_api_proto::Command::UpdateTracks
    fn on_update_tracks(
        &mut self,
        peer_id: PeerId,
        tracks_patches: Vec<TrackPatch>,
    ) -> Self::Output {
        if let Ok(p) = self.peers.get_peer_by_id(peer_id) {
            let member_id = p.member_id();
            Ok(Box::new(
                self.members
                    .send_event_to_member(
                        member_id,
                        Event::TracksUpdated {
                            peer_id,
                            tracks_patches,
                        },
                    )
                    .into_actor(self),
            ))
        } else {
            Ok(Box::new(actix::fut::ok(())))
        }
    }
}
