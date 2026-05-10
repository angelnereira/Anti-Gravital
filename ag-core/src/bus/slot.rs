/// Byte offsets within the request area of a slot (first 4096 bytes).
pub const REQ_OFFSET_STATE: usize = 0;
pub const REQ_OFFSET_METHOD: usize = 1;
pub const REQ_OFFSET_PATH_LEN: usize = 2;
pub const REQ_OFFSET_BODY_LEN: usize = 4;
pub const REQ_OFFSET_REQUEST_ID: usize = 8;
pub const REQ_OFFSET_AUTH_CLAIMS: usize = 64;
pub const REQ_OFFSET_PATH: usize = 576;
pub const REQ_OFFSET_PAYLOAD: usize = 1088;

/// Byte offsets within the response area of a slot (second 4096 bytes, relative to slot base).
pub const RESP_AREA_OFFSET: usize = 4096;
pub const RESP_OFFSET_STATE: usize = RESP_AREA_OFFSET + 0;
pub const RESP_OFFSET_STATUS: usize = RESP_AREA_OFFSET + 2;
pub const RESP_OFFSET_LEN: usize = RESP_AREA_OFFSET + 4;
pub const RESP_OFFSET_DATA: usize = RESP_AREA_OFFSET + 64;

/// Maximum sizes for variable-length fields within a slot.
pub const MAX_AUTH_SIZE: usize = 512;
pub const MAX_PATH_SIZE: usize = 512;
pub const MAX_PAYLOAD_SIZE: usize = 3008;
pub const MAX_RESPONSE_SIZE: usize = 4032;

/// Total size of one slot: 4096 bytes (request) + 4096 bytes (response).
pub const SLOT_SIZE: usize = 8192;

/// State machine for a ring buffer slot.
///
/// Transitions:
///   Empty -> Writing  (Rust: atomic CAS to claim)
///   Writing -> Ready  (Rust: release store after writing request data)
///   Ready -> Reading  (Go: atomic CAS to claim)
///   Reading -> Done   (Go: release store after writing response data)
///   Done -> Empty     (Rust: release store after reading response)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    Empty   = 0,
    Writing = 1,
    Ready   = 2,
    Reading = 3,
    Done    = 4,
}

impl SlotState {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Empty),
            1 => Some(Self::Writing),
            2 => Some(Self::Ready),
            3 => Some(Self::Reading),
            4 => Some(Self::Done),
            _ => None,
        }
    }
}

/// Numeric codes for HTTP methods stored in slots.
pub mod method {
    pub const GET:     u8 = 0;
    pub const POST:    u8 = 1;
    pub const PUT:     u8 = 2;
    pub const PATCH:   u8 = 3;
    pub const DELETE:  u8 = 4;
    pub const HEAD:    u8 = 5;
    pub const OPTIONS: u8 = 6;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_state_round_trips() {
        for v in 0u8..=4 {
            let state = SlotState::from_u8(v).expect("valid state value");
            assert_eq!(state as u8, v);
        }
        assert!(SlotState::from_u8(5).is_none());
    }
}
