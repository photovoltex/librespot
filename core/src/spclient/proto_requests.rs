use crate::config::OS;
use crate::version;
use librespot_protocol::resolve::fetch::Type;
use librespot_protocol::resolve::{Fetch, ResolveRequest};
use librespot_protocol::ucs::ucs_request::CallerInfo;
use librespot_protocol::ucs::UcsRequest;
use protobuf::{EnumOrUnknown, MessageField};

// this currently just mocks the request send by mobile and desktop
pub(super) fn ucs_request() -> UcsRequest {
    let (fetch_type, request_origin_id, reason) = match OS {
        "android" | "ios" => (Some(Type::ASYNC), "com.spotify.music", "ASYNC"),
        _ => (None, "desktop", "BLOCKING"),
    };

    let resolve_request = MessageField::some(ResolveRequest {
        property_set_id: version::spotify_property_set_id(),
        fetch_type: MessageField::some(Fetch {
            type_: fetch_type.map(EnumOrUnknown::new).unwrap_or_default(),
            ..Default::default()
        }),
        ..Default::default()
    });

    UcsRequest {
        caller_info: MessageField::some(CallerInfo {
            request_origin_id: request_origin_id.to_string(),
            request_orgin_version: version::spotify_semantic_version(),
            reason: reason.to_string(),
            ..Default::default()
        }),
        resolve_request,
        ..Default::default()
    }
}
