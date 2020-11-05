use function_name::named;
use futures::{channel::mpsc, StreamExt as _};
use medea_client_api_proto::{
    Command, Event, PeerId, TrackId, TrackPatchCommand, TrackPatchEvent,
    TrackUpdate,
};

use crate::{
    grpc_control_api::{create_room_req, ControlClient},
    if_let_next,
    signalling::{SendCommand, TestMember},
    test_name,
};

#[actix_rt::test]
#[named]
async fn track_mute_doesnt_renegotiates() {
    let mut client = ControlClient::new().await;
    let credentials = client.create(create_room_req(test_name!())).await;

    let (publisher_tx, mut publisher_rx) = mpsc::unbounded();
    let publisher = TestMember::connect(
        credentials.get("publisher").unwrap(),
        Some(Box::new(move |event, _, _| {
            publisher_tx.unbounded_send(event.clone()).unwrap();
        })),
        None,
        TestMember::DEFAULT_DEADLINE,
        true,
    )
    .await;
    let (subscriber_tx, mut subscriber_rx) = mpsc::unbounded();
    let _subscriber = TestMember::connect(
        credentials.get("responder").unwrap(),
        Some(Box::new(move |event, _, _| {
            subscriber_tx.unbounded_send(event.clone()).unwrap();
        })),
        None,
        TestMember::DEFAULT_DEADLINE,
        true,
    )
    .await;

    // wait until initial negotiation finishes
    if_let_next! {
        Event::SdpAnswerMade { .. } = publisher_rx {}
    }

    publisher
        .send(SendCommand(Command::UpdateTracks {
            peer_id: PeerId(0),
            tracks_patches: vec![TrackPatchCommand {
                id: TrackId(0),
                is_disabled: None,
                is_muted: Some(true),
            }],
        }))
        .await
        .unwrap();

    loop {
        if let Event::TracksApplied {
            peer_id,
            updates,
            negotiation_role,
        } = publisher_rx.select_next_some().await
        {
            assert_eq!(peer_id, PeerId(0));

            assert!(negotiation_role.is_none());

            assert_eq!(updates.len(), 1);
            assert_eq!(
                updates[0],
                TrackUpdate::Updated(TrackPatchEvent {
                    is_muted: Some(true),
                    id: TrackId(0),
                    is_disabled_general: None,
                    is_disabled_individual: None
                })
            );
            break;
        }
    }

    loop {
        if let Event::TracksApplied {
            peer_id,
            updates,
            negotiation_role,
        } = subscriber_rx.select_next_some().await
        {
            assert_eq!(peer_id, PeerId(1));

            assert!(negotiation_role.is_none());

            assert_eq!(updates.len(), 1);
            assert_eq!(
                updates[0],
                TrackUpdate::Updated(TrackPatchEvent {
                    is_muted: Some(true),
                    id: TrackId(0),
                    is_disabled_general: None,
                    is_disabled_individual: None
                })
            );
            break;
        }
    }
}
