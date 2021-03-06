//! Typed representations of modem error codes.
#![allow(missing_docs)]
/// A CMS (SMS-related) error code.
///
/// I can't be bothered to write out all the error code meanings twice. If you hit the `[src]`
/// button in rustdoc, it'll take you to the definition of this `enum`, where the meanings of each
/// variant are annotated with `#[fail(display)]` attributes.
///
/// Obviously, this means that this `enum` has a rather useful `Display` implementation.
#[derive(FromPrimitive, Fail, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum CmsError {
    #[fail(display = "Unassigned (unallocated) number")]
    UnassignedNumber = 1,
    #[fail(display = "Operatior determined barring")]
    OperatorDeterminedBarring = 8,
    #[fail(display = "Call barred")]
    CallBarred = 10,
    #[fail(display = "Short message transfer rejected")]
    TransferRejected = 21,
    #[fail(display = "Destination out of service")]
    DestinationOutOfService = 27,
    #[fail(display = "Unidentified subscriber")]
    UnidentifiedSubscriber = 28,
    #[fail(display = "Facility rejected")]
    FacilityRejected = 29,
    #[fail(display = "Unknown subscriber")]
    UnknownSubscriber = 30,
    #[fail(display = "Network out of order")]
    NetworkOutOfOrder = 38,
    #[fail(display = "Temporary failure")]
    TemporaryFailure = 41,
    #[fail(display = "Congestion")]
    Congestion = 42,
    #[fail(display = "Resources unavailable, unspecified")]
    ResourcesUnavailable = 47,
    #[fail(display = "Requested facility not subscribed")]
    NotSubscribed = 50,
    #[fail(display = "Requested facility not implemented")]
    NotImplemented = 69,
    #[fail(display = "Invalid short message transfer reference value")]
    InvalidReferenceValue = 81,
    #[fail(display = "Invalid message, unspecified")]
    InvalidMessage = 95,
    #[fail(display = "Invalid mandatory information")]
    InvalidMandatoryInformation = 96,
    #[fail(display = "Message type non-existent or not implemented")]
    NonexistentMessageType = 97,
    #[fail(display = "Message not compatible with short message protocol state")]
    IncompatibleMessage = 98,
    #[fail(display = "Information element non-existent or not implemented")]
    NonexistentInformationElement = 99,
    #[fail(display = "Protocol error, unspecified")]
    ProtocolError = 111,
    #[fail(display = "Internetworking, unspecified")]
    InternetworkingError = 127,
    #[fail(display = "ME failure")]
    MeFailure = 300,
    #[fail(display = "SMS service of ME reserved")]
    SmsServiceReserved = 301,
    #[fail(display = "Operation not allowed")]
    NotAllowed = 302,
    #[fail(display = "Operation not supported")]
    NotSupported = 303,
    #[fail(display = "Invalid PDU mode parameter")]
    InvalidPduModeParameter = 304,
    #[fail(display = "Invalid text mode parameter")]
    InvalidTextModeParameter = 305,
    #[fail(display = "(U)SIM not inserted")]
    SimNotInserted = 310,
    #[fail(display = "(U)SIM PIN required")]
    SimPinRequired = 311,
    #[fail(display = "PH-(U)SIM PIN required")]
    PhSimPinRequired = 312,
    #[fail(display = "(U)SIM failure")]
    SimFailure = 313,
    #[fail(display = "(U)SIM busy")]
    SimBusy = 314,
    #[fail(display = "(U)SIM wrong")]
    SimWrong = 315,
    #[fail(display = "(U)SIM PUK required")]
    SimPukRequired = 316,
    #[fail(display = "(U)SIM PIN2 required")]
    SimPin2Required = 317,
    #[fail(display = "(U)SIM PUK2 required")]
    SimPuk2Required = 318,
    #[fail(display = "Memory failure")]
    MemoryFailure = 320,
    #[fail(display = "Invalid memory index")]
    InvalidMemoryIndex = 321,
    #[fail(display = "Memory full")]
    MemoryFull = 322,
    #[fail(display = "SMSC address unknown")]
    SmscAddressUnknown = 330,
    #[fail(display = "No network service")]
    NoNetworkService = 331,
    #[fail(display = "Network timeout")]
    NetworkTimeout = 332,
    #[fail(display = "No `+CNMA` acknowledgement expected")]
    NoCnmaAcknowledgementExpected = 340,
    #[fail(display = "Unknown error")]
    UnknownError = 500,
}
