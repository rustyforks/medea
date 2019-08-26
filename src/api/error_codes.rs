//! All errors which medea can return to control API user.
//!
//! # Error codes ranges
//! * __1000...1000__ Unknow server error
//! * __1001...1099__ Not found errors
//! * __1100...1199__ Spec errors
//! * __1200...1299__ Parse errors
//! * __1300...1399__ Conflicts

use std::string::ToString;

use derive_more::Display;
use medea_grpc_proto::control::Error as ErrorProto;

use crate::{
    api::{
        control::{
            endpoints::webrtc_play_endpoint::SrcParseError,
            grpc::server::ControlApiError, local_uri::LocalUriParseError,
            TryFromElementError, TryFromProtobufError,
        },
        error_codes::ErrorCode::ElementIdIsNotLocal,
    },
    signalling::{
        elements::{member::MemberError, MembersLoadError},
        participants::ParticipantServiceErr,
        room::RoomError,
        room_service::RoomServiceError,
    },
};

/// Medea's control API error response.
pub struct ErrorResponse {
    /// [`ErrorCode`] which will be returned with code and message.
    error_code: ErrorCode,

    /// Element ID where some error happened. May be empty.
    element_id: String,

    /// If some unexpected error will be throwed then this field will
    /// store this error converted to [`String`].
    ///
    /// Normally this field should be [`None`].
    unknown_error: Option<String>,
}

impl ErrorResponse {
    /// New normal [`ErrorResponse`] with [`ErrorCode`] and element ID.
    pub fn new<T: ToString>(error_code: ErrorCode, element_id: &T) -> Self {
        Self {
            error_code,
            element_id: element_id.to_string(),
            unknown_error: None,
        }
    }

    /// New [`ErrorResponse`] only with [`ErrorCode`].
    pub fn new_empty(error_code: ErrorCode) -> Self {
        Self {
            error_code,
            element_id: String::new(),
            unknown_error: None,
        }
    }

    /// [`ErrorResponse`] for all unexpected errors.
    ///
    /// Provide unexpected `Error` in this function.
    pub fn unknown<B: ToString>(unknown_error: &B) -> Self {
        Self {
            error_code: ErrorCode::UnknownError,
            unknown_error: Some(unknown_error.to_string()),
            element_id: String::new(),
        }
    }
}

impl Into<ErrorProto> for ErrorResponse {
    fn into(self) -> ErrorProto {
        let mut error = ErrorProto::new();

        if let Some(unknown_error) = &self.unknown_error {
            error.set_text(format!(
                "{} Here is error: '{}'",
                self.error_code.to_string(),
                unknown_error
            ));
        } else {
            error.set_text(self.error_code.to_string());
        }

        error.set_element(self.element_id.to_string());
        error.set_code(self.error_code as u32);

        error
    }
}

/// Medea control API errors.
#[derive(Display)]
pub enum ErrorCode {
    /// Unknown server error.
    ///
    /// Code: __1000__.
    #[display(fmt = "Unexpected error happened.")]
    UnknownError = 1000,

    ////////////////////////////////////
    // Not found (1001 - 1099 codes) //
    //////////////////////////////////
    /// Publish endpoint not found.
    ///
    /// Code: __1001__.
    #[display(fmt = "Publish endpoint not found.")]
    PublishEndpointNotFound = 1001,
    /// Play endpoint not found.
    ///
    /// Code: __1002__.
    #[display(fmt = "Play endpoint not found.")]
    PlayEndpointNotFound = 1002,
    /// Member not found.
    ///
    /// Code: __1003__.
    #[display(fmt = "Member not found.")]
    MemberNotFound = 1003,
    /// Room not found.
    ///
    /// Code: __1004__.
    #[display(fmt = "Room not found.")]
    RoomNotFound = 1004,
    /// Endpoint not found.
    ///
    /// Code: __1005__.
    #[display(fmt = "Endpoint not found.")]
    EndpointNotFound = 1005,

    //////////////////////////////////////
    // Spec errors (1100 - 1199 codes) //
    ////////////////////////////////////
    /// Medea expects `Room` element in pipeline but received not him.
    ///
    /// Code: __1100__.
    #[display(fmt = "Expecting Room element but it's not.")]
    NotRoomInSpec = 1100,
    /// Medea expects `Member` element in pipeline but received not him.
    ///
    /// Code: __1101__.
    #[display(fmt = "Expecting Member element but it's not.")]
    NotMemberInSpec = 1101,
    /// Invalid source URI in play endpoint.
    ///
    /// Code: __1102__.
    #[display(fmt = "Invalid source ID in publish endpoint spec.")]
    InvalidSrcUri = 1102,
    /// Provided element ID to Room element but element spec is not for Room.
    ///
    /// Code: __1103__.
    #[display(
        fmt = "You provided ID for Room but element's spec is not for Room."
    )]
    ElementIdForRoomButElementIsNot = 1103,
    /// Provided element ID to Member element but element spec is not for
    /// Member.
    ///
    /// Code: __1104__.
    #[display(
        fmt = "You provided ID for Member but element's spec is not for Room."
    )]
    ElementIdForMemberButElementIsNot = 1104,
    /// Provided element ID to Endpoint element but element spec is not for
    /// Endpoint.
    ///
    /// Code: __1105__.
    #[display(fmt = "You provided ID for Endpoint but element's spec is not \
                     for Room.")]
    ElementIdForEndpointButElementIsNot = 1105,
    /// Invalid ID for element.
    ///
    /// Code: __1106__
    #[display(fmt = "Invalid element's URI.")]
    InvalidElementUri = 1106,
    /// Provided not source URI in [`WebRtcPlayEndpoint`].
    ///
    /// Code: __1107__.
    #[display(fmt = "Provided not source URI.")]
    NotSourceUri = 1107,

    /////////////////////////////////
    // Parse errors (1200 - 1299) //
    ///////////////////////////////
    /// Element's ID don't have "local://" prefix.
    ///
    /// Code: __1200__.
    #[display(fmt = "Element's ID's URI not have 'local://' protocol.")]
    ElementIdIsNotLocal = 1200,
    /// Element's ID have too many paths (slashes).
    ///
    /// Code: __1201__.
    #[display(fmt = "In provided element's ID too many slashes.")]
    ElementIdIsTooLong = 1201,
    /// Missing some fields in element's ID.
    ///
    /// Code: __1202__.
    #[display(fmt = "Missing some fields in element ID.")]
    MissingFieldsInSrcUri = 1202,
    /// Empty element ID.
    ///
    /// Code: __1203__.
    #[display(fmt = "Provided empty element ID.")]
    EmptyElementId = 1203,

    /////////////////////////////
    // Conflict (1300 - 1399) //
    ///////////////////////////
    /// Member already exists.
    ///
    /// Code: __1300__.
    #[display(fmt = "Member already exists.")]
    MemberAlreadyExists = 1300,
    /// Endpoint already exists.
    ///
    /// Code: __1301__.
    #[display(fmt = "Endpoint already exists.")]
    EndpointAlreadyExists = 1301,
    /// Room already exists.
    ///
    /// Code: __1302__.
    #[display(fmt = "Room already exists.")]
    RoomAlreadyExists = 1302,
}

impl From<ParticipantServiceErr> for ErrorResponse {
    fn from(err: ParticipantServiceErr) -> Self {
        match err {
            ParticipantServiceErr::EndpointNotFound(id) => {
                Self::new(ErrorCode::EndpointNotFound, &id)
            }
            ParticipantServiceErr::ParticipantNotFound(id) => {
                Self::new(ErrorCode::MemberNotFound, &id)
            }
            ParticipantServiceErr::ParticipantAlreadyExists(id) => {
                Self::new(ErrorCode::MemberAlreadyExists, &id)
            }
            ParticipantServiceErr::EndpointAlreadyExists(id) => {
                Self::new(ErrorCode::EndpointAlreadyExists, &id)
            }
            _ => Self::unknown(&err),
        }
    }
}

impl From<TryFromProtobufError> for ErrorResponse {
    fn from(err: TryFromProtobufError) -> Self {
        match err {
            TryFromProtobufError::SrcUriError(e) => e.into(),
            _ => Self::unknown(&err),
        }
    }
}

impl From<LocalUriParseError> for ErrorResponse {
    fn from(err: LocalUriParseError) -> Self {
        match err {
            LocalUriParseError::NotLocal(text) => {
                Self::new(ElementIdIsNotLocal, &text)
            }
            LocalUriParseError::TooManyFields(_, text) => {
                Self::new(ErrorCode::ElementIdIsTooLong, &text)
            }
            LocalUriParseError::Empty => {
                Self::new_empty(ErrorCode::EmptyElementId)
            }
            LocalUriParseError::MissingFields(text) => {
                Self::new(ErrorCode::MissingFieldsInSrcUri, &text)
            }
        }
    }
}

impl From<RoomError> for ErrorResponse {
    fn from(err: RoomError) -> Self {
        match err {
            RoomError::MemberError(e) => e.into(),
            RoomError::MembersLoadError(e) => e.into(),
            RoomError::ParticipantServiceErr(e) => e.into(),
            _ => Self::unknown(&err),
        }
    }
}

impl From<MembersLoadError> for ErrorResponse {
    fn from(err: MembersLoadError) -> Self {
        match err {
            MembersLoadError::TryFromError(e, id) => match e {
                TryFromElementError::NotMember => {
                    Self::new(ErrorCode::NotMemberInSpec, &id)
                }
                TryFromElementError::NotRoom => {
                    Self::new(ErrorCode::NotRoomInSpec, &id)
                }
            },
            MembersLoadError::MemberNotFound(id) => {
                Self::new(ErrorCode::MemberNotFound, &id)
            }
            MembersLoadError::PublishEndpointNotFound(id) => {
                Self::new(ErrorCode::PublishEndpointNotFound, &id)
            }
            MembersLoadError::PlayEndpointNotFound(id) => {
                Self::new(ErrorCode::PlayEndpointNotFound, &id)
            }
        }
    }
}

impl From<MemberError> for ErrorResponse {
    fn from(err: MemberError) -> Self {
        match err {
            MemberError::PlayEndpointNotFound(id) => {
                Self::new(ErrorCode::PlayEndpointNotFound, &id)
            }
            MemberError::PublishEndpointNotFound(id) => {
                Self::new(ErrorCode::PublishEndpointNotFound, &id)
            }
        }
    }
}

impl From<SrcParseError> for ErrorResponse {
    fn from(err: SrcParseError) -> Self {
        match err {
            SrcParseError::NotSrcUri(text) => {
                Self::new(ErrorCode::NotSourceUri, &text)
            }
            SrcParseError::LocalUriParseError(_, err) => err.into(),
        }
    }
}

impl From<RoomServiceError> for ErrorResponse {
    fn from(err: RoomServiceError) -> Self {
        match err {
            RoomServiceError::RoomNotFound(id) => {
                Self::new(ErrorCode::RoomNotFound, &id)
            }
            RoomServiceError::RoomAlreadyExists(id) => {
                Self::new(ErrorCode::RoomAlreadyExists, &id)
            }
            RoomServiceError::RoomError(e) => e.into(),
            _ => Self::unknown(&err),
        }
    }
}

impl From<ControlApiError> for ErrorResponse {
    fn from(err: ControlApiError) -> Self {
        match err {
            ControlApiError::LocalUri(e) => e.into(),
            ControlApiError::TryFromProtobuf(e) => e.into(),
            _ => Self::unknown(&err),
        }
    }
}