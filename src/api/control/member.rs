//! Member definitions and implementations.

use std::{convert::TryFrom, fmt::Display, sync::Arc};

use serde::Deserialize;

use super::{
    endpoint::{WebRtcPlayEndpoint, WebRtcPublishEndpoint},
    pipeline::Pipeline,
    room::RoomSpec,
    Element, TryFromElementError,
};

/// ID of [`Member`].
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Id(pub String);

impl Display for Id {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}", self.0)
    }
}

/// Media server user with its ID, credentials and spec.
#[derive(Clone, Debug)]
pub struct Member {
    /// ID of [`Member`].
    id: Id,

    /// Control API specification of this [`Member`].
    spec: Arc<MemberSpec>,

    /// Receivers of this [`Member`]'s publish endpoints.
    receivers: Vec<Id>,
}

impl Member {
    pub fn new(
        id: Id,
        spec: MemberSpec,
        room_spec: &RoomSpec,
    ) -> Result<Self, TryFromElementError> {
        Ok(Self {
            receivers: room_spec.get_receivers_for_member(&id)?,
            spec: Arc::new(spec),
            id,
        })
    }

    /// Returns [`Id`] of [`Member`].
    pub fn id(&self) -> &Id {
        &self.id
    }

    /// Returns credentials to authorize [`Member`] with.
    pub fn credentials(&self) -> &str {
        self.spec.credentials()
    }

    /// Returns all [`WebRtcPlayEndpoint`]s of this [`Member`].
    pub fn play_endpoints(&self) -> Vec<&WebRtcPlayEndpoint> {
        self.spec.play_endpoints()
    }

    /// Returns all [`WebRtcPublishEndpoint`]s of this [`Member`].
    pub fn publish_endpoints(&self) -> Vec<&WebRtcPublishEndpoint> {
        self.spec.publish_endpoints()
    }

    /// Returns all receivers [`Id`] of this [`Member`].
    pub fn receivers(&self) -> &Vec<Id> {
        &self.receivers
    }
}

/// Newtype for [`Element::Member`] variant.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
pub struct MemberSpec {
    /// Spec of this [`Member`].
    pipeline: Pipeline,

    /// Credentials to authorize [`Member`] with.
    credentials: String,
}

impl MemberSpec {
    /// Returns all [`WebRtcPlayEndpoint`]s of this [`MemberSpec`].
    pub fn play_endpoints(&self) -> Vec<&WebRtcPlayEndpoint> {
        self.pipeline.play_endpoints()
    }

    /// Returns all [`WebRtcPublishEndpoint`]s of this [`MemberSpec`].
    pub fn publish_endpoints(&self) -> Vec<&WebRtcPublishEndpoint> {
        self.pipeline.publish_endpoints()
    }

    pub fn credentials(&self) -> &str {
        &self.credentials
    }
}

impl TryFrom<&Element> for MemberSpec {
    type Error = TryFromElementError;

    fn try_from(from: &Element) -> Result<Self, Self::Error> {
        match from {
            Element::Member { spec, credentials } => Ok(Self {
                pipeline: spec.clone(),
                credentials: credentials.clone(),
            }),
            _ => Err(TryFromElementError::NotMember),
        }
    }
}
