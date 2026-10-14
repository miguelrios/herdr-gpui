//! Bounded identity inspection for a partially received raw surface. This does
//! not validate its body or replace strict decoding of the completed message.

use crate::{ClientShellSnapshot, PaneSurfaceFrame};

const MAX_HEADER_BYTES: usize = 512;
// Positional ServerMessage discriminants are part of the generation-1 wire
// contract. Tests encode the actual variants to guard against drift.
const SURFACE_TAG: u32 = 13;
const PATCH_TAG: u32 = 19;

/// Whether a raw surface prefix names the current authoritative projection and
/// advances a previously validated surface. Only the identity header is read;
/// callers must retain the completed-frame decoding and validation barriers.
/// Other encodings and headers exceeding this bounded probe return false.
pub fn has_current_surface_prefix(
    payload: &[u8],
    snapshot: &ClientShellSnapshot,
    surface: &PaneSurfaceFrame,
) -> bool {
    if surface.boot_id != snapshot.boot_id || surface.popup.is_some() {
        return false;
    }
    let payload = &payload[..payload.len().min(MAX_HEADER_BYTES)];
    let config = bincode::config::standard().with_limit::<MAX_HEADER_BYTES>();
    let Ok(((tag, boot_id, projection, revision), used)) =
        bincode::serde::borrow_decode_from_slice::<(u32, &str, u64, u64), _>(payload, config)
    else {
        return false;
    };
    if boot_id != snapshot.boot_id || projection != snapshot.revision {
        return false;
    }
    match tag {
        SURFACE_TAG => revision > surface.surface_revision,
        PATCH_TAG => {
            let Ok((next, _)) =
                bincode::serde::borrow_decode_from_slice::<u64, _>(&payload[used..], config)
            else {
                return false;
            };
            surface.projection_revision == projection
                && revision == surface.surface_revision
                && revision.checked_add(1) == Some(next)
        }
        _ => false,
    }
}
