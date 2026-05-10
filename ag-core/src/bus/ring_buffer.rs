use std::ffi::CString;
use std::sync::atomic::{AtomicU8, Ordering, fence};

use libc::{
    MAP_FAILED, MAP_SHARED, O_CREAT, O_RDWR, PROT_READ, PROT_WRITE,
    close, ftruncate, mmap, munmap, off_t, shm_open, shm_unlink,
};
use thiserror::Error;

use super::slot::{
    MAX_AUTH_SIZE, MAX_PATH_SIZE, MAX_PAYLOAD_SIZE, MAX_RESPONSE_SIZE,
    REQ_OFFSET_AUTH_CLAIMS, REQ_OFFSET_BODY_LEN, REQ_OFFSET_METHOD,
    REQ_OFFSET_PATH, REQ_OFFSET_PATH_LEN, REQ_OFFSET_PAYLOAD,
    REQ_OFFSET_REQUEST_ID, REQ_OFFSET_STATE, RESP_OFFSET_DATA,
    RESP_OFFSET_LEN, RESP_OFFSET_STATE, RESP_OFFSET_STATUS, SLOT_SIZE,
    SlotState,
};

// ── Bus constants ────────────────────────────────────────────────────────────

/// Default POSIX shared memory segment name.
pub const BUS_DEFAULT_NAME: &str = "/ag-bus-v1";

pub const BUS_MAGIC: u64 = 0xAEAD_0001_0000_0001;
pub const BUS_VERSION: u32 = 1;
pub const SEGMENT_SIZE: usize = 64 * 1024 * 1024; // 64 MB
pub const HEADER_SIZE: usize = 256;

/// Maximum number of concurrent in-flight request slots.
pub const MAX_SLOTS: usize = (SEGMENT_SIZE - HEADER_SIZE) / SLOT_SIZE;

// ── Header byte offsets ──────────────────────────────────────────────────────

const HDR_MAGIC: usize = 0;
const HDR_VERSION: usize = 8;
const HDR_CAPACITY: usize = 12;
const HDR_WRITE_HEAD: usize = 16;
const HDR_READ_HEAD: usize = 24;

// ── Error type ───────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum BusError {
    #[error("shm_open failed: errno {0}")]
    ShmOpen(i32),
    #[error("ftruncate failed: errno {0}")]
    Ftruncate(i32),
    #[error("mmap failed: errno {0}")]
    Mmap(i32),
    #[error("shared memory magic mismatch (got {got:#x}, expected {expected:#x})")]
    MagicMismatch { got: u64, expected: u64 },
    #[error("shared memory version {got} incompatible with this build (requires {required})")]
    VersionMismatch { got: u32, required: u32 },
    #[error("payload too large: {size} bytes exceeds slot limit of {limit} bytes")]
    PayloadTooLarge { size: usize, limit: usize },
    #[error("bus full: all {0} slots are occupied; apply backpressure")]
    BusFull(usize),
}

// ── MemoryBus ────────────────────────────────────────────────────────────────

/// A POSIX shared memory ring buffer connecting the Rust Shield layer to the
/// Go Brain layer with nanosecond-range inter-layer communication latency.
///
/// Memory layout:
/// ```text
/// [0..256)             Header (magic, version, capacity, heads)
/// [256..256+8192*N)    N slots (N = MAX_SLOTS), each 8192 bytes
///                        [slot+0..slot+4096)    Request area
///                        [slot+4096..slot+8192) Response area
/// ```
///
/// Slot state transitions are coordinated via lock-free atomic byte operations:
/// ```text
/// Empty -> Writing (Rust: CAS claim)
/// Writing -> Ready (Rust: release store after writing request data)
/// Ready -> Reading (Go: CAS claim)
/// Reading -> Done  (Go: release store after writing response data)
/// Done -> Empty    (Rust: release store after reading response)
/// ```
pub struct MemoryBus {
    /// Base pointer to the mapped memory region.
    ptr: *mut u8,
    /// Total mapped size in bytes.
    len: usize,
    /// Whether this instance is responsible for segment lifecycle.
    owner: bool,
    /// Cached slot count from the header.
    capacity: usize,
    /// Segment name; stored to support proper cleanup in Drop.
    shm_name: CString,
}

// SAFETY: MemoryBus is designed for concurrent multi-threaded access. All
// slot state mutations use AtomicU8 operations with correct memory ordering.
// The raw pointer `ptr` is valid for the lifetime of the mmap, bounded by Drop.
unsafe impl Send for MemoryBus {}
unsafe impl Sync for MemoryBus {}

impl MemoryBus {
    // ── Constructors ─────────────────────────────────────────────────────────

    /// Creates and initialises a new shared memory segment.
    ///
    /// The segment is created with `O_CREAT | O_RDWR`, sized to `SEGMENT_SIZE`,
    /// and memory-mapped with `PROT_READ | PROT_WRITE | MAP_SHARED`. The header
    /// is written with a full `SeqCst` fence before this function returns.
    pub fn create(name: &str) -> Result<Self, BusError> {
        let cname = CString::new(name).unwrap();
        // SAFETY: shm_open is a standard POSIX call. Returns -1 on failure.
        let fd = unsafe { shm_open(cname.as_ptr(), O_CREAT | O_RDWR, 0o600) };
        if fd < 0 {
            return Err(BusError::ShmOpen(errno()));
        }
        // SAFETY: ftruncate sets the size of the shared memory object.
        if unsafe { ftruncate(fd, SEGMENT_SIZE as off_t) } < 0 {
            unsafe { close(fd) };
            return Err(BusError::Ftruncate(errno()));
        }
        let ptr = map_fd(fd)?;
        unsafe { close(fd) };

        let bus = Self {
            ptr,
            len: SEGMENT_SIZE,
            owner: true,
            capacity: MAX_SLOTS,
            shm_name: cname,
        };
        bus.init_header();
        Ok(bus)
    }

    /// Opens an existing shared memory segment created by [`MemoryBus::create`].
    ///
    /// The header magic and version are validated before this function returns.
    pub fn open(name: &str) -> Result<Self, BusError> {
        let cname = CString::new(name).unwrap();
        // Open with RDWR so Go workers can write responses.
        let fd = unsafe { shm_open(cname.as_ptr(), O_RDWR, 0) };
        if fd < 0 {
            return Err(BusError::ShmOpen(errno()));
        }
        let ptr = map_fd(fd)?;
        unsafe { close(fd) };

        let bus = Self {
            ptr,
            len: SEGMENT_SIZE,
            owner: false,
            capacity: MAX_SLOTS,
            shm_name: cname,
        };
        bus.validate_header()?;
        Ok(bus)
    }

    // ── Header ───────────────────────────────────────────────────────────────

    fn init_header(&self) {
        unsafe {
            write_u64_le(self.ptr, HDR_MAGIC, BUS_MAGIC);
            write_u32_le(self.ptr, HDR_VERSION, BUS_VERSION);
            write_u32_le(self.ptr, HDR_CAPACITY, MAX_SLOTS as u32);
            write_u64_le(self.ptr, HDR_WRITE_HEAD, 0);
            write_u64_le(self.ptr, HDR_READ_HEAD, 0);
        }
        // Full fence: header writes must be globally visible before slot ops.
        fence(Ordering::SeqCst);
    }

    fn validate_header(&self) -> Result<(), BusError> {
        fence(Ordering::Acquire);
        let magic = unsafe { read_u64_le(self.ptr, HDR_MAGIC) };
        if magic != BUS_MAGIC {
            return Err(BusError::MagicMismatch { got: magic, expected: BUS_MAGIC });
        }
        let version = unsafe { read_u32_le(self.ptr, HDR_VERSION) };
        if version != BUS_VERSION {
            return Err(BusError::VersionMismatch { got: version, required: BUS_VERSION });
        }
        Ok(())
    }

    // ── Slot addressing ──────────────────────────────────────────────────────

    #[inline(always)]
    fn slot_base(&self, index: usize) -> usize {
        HEADER_SIZE + index * SLOT_SIZE
    }

    #[inline(always)]
    fn state_atomic(&self, index: usize) -> &AtomicU8 {
        // SAFETY: The byte at this offset is within the mapped segment.
        // AtomicU8 has the same size and alignment as u8.
        unsafe {
            &*(self.ptr.add(self.slot_base(index) + REQ_OFFSET_STATE) as *const AtomicU8)
        }
    }

    #[inline(always)]
    fn resp_state_atomic(&self, index: usize) -> &AtomicU8 {
        // SAFETY: Same invariants as state_atomic.
        unsafe {
            &*(self.ptr.add(self.slot_base(index) + RESP_OFFSET_STATE) as *const AtomicU8)
        }
    }

    // ── Write path (Rust Shield -> Go Brain) ─────────────────────────────────

    /// Attempts to claim an empty slot for writing via compare-and-swap.
    ///
    /// Returns the claimed slot index, or `BusError::BusFull` when all slots
    /// are occupied (circuit breaker: caller should return HTTP 503 immediately
    /// rather than queuing in memory and risking OOM).
    pub fn claim_write_slot(&self) -> Result<usize, BusError> {
        for i in 0..self.capacity {
            if self
                .state_atomic(i)
                .compare_exchange(
                    SlotState::Empty as u8,
                    SlotState::Writing as u8,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Ok(i);
            }
        }
        Err(BusError::BusFull(self.capacity))
    }

    /// Writes request data into a previously claimed slot and marks it READY.
    ///
    /// The caller must own the slot (obtained via [`claim_write_slot`]).
    /// After this call the slot is visible to Go workers.
    pub fn write_request(
        &self,
        index: usize,
        request_id: u64,
        method: u8,
        path: &[u8],
        auth_claims: &[u8],
        payload: &[u8],
    ) {
        let base = self.slot_base(index);
        let path_len = path.len().min(MAX_PATH_SIZE) as u16;
        let body_len = payload.len().min(MAX_PAYLOAD_SIZE) as u32;
        let auth_len = auth_claims.len().min(MAX_AUTH_SIZE);

        unsafe {
            write_u8(self.ptr, base + REQ_OFFSET_METHOD, method);
            write_u16_le(self.ptr, base + REQ_OFFSET_PATH_LEN, path_len);
            write_u32_le(self.ptr, base + REQ_OFFSET_BODY_LEN, body_len);
            write_u64_le(self.ptr, base + REQ_OFFSET_REQUEST_ID, request_id);

            let auth_dst = self.ptr.add(base + REQ_OFFSET_AUTH_CLAIMS);
            auth_dst.write_bytes(0, MAX_AUTH_SIZE);
            auth_dst.copy_from_nonoverlapping(auth_claims.as_ptr(), auth_len);

            let path_dst = self.ptr.add(base + REQ_OFFSET_PATH);
            path_dst.write_bytes(0, MAX_PATH_SIZE);
            path_dst.copy_from_nonoverlapping(path.as_ptr(), path_len as usize);

            let payload_dst = self.ptr.add(base + REQ_OFFSET_PAYLOAD);
            payload_dst.write_bytes(0, MAX_PAYLOAD_SIZE);
            payload_dst.copy_from_nonoverlapping(payload.as_ptr(), body_len as usize);
        }

        // Reset response state so the Go side does not see a stale DONE.
        self.resp_state_atomic(index)
            .store(SlotState::Empty as u8, Ordering::Relaxed);

        // Release store: all writes above become visible before state = READY.
        self.state_atomic(index)
            .store(SlotState::Ready as u8, Ordering::Release);
    }

    /// Spin-waits for the Go side to write a response into the slot.
    ///
    /// Returns `(http_status, response_body)`. Resets the slot to Empty
    /// after reading so it is available for the next request.
    pub fn wait_response(&self, index: usize) -> (u16, Vec<u8>) {
        let resp_state = self.resp_state_atomic(index);
        loop {
            if resp_state.load(Ordering::Acquire) == SlotState::Done as u8 {
                break;
            }
            std::hint::spin_loop();
        }

        let base = self.slot_base(index);
        let status = unsafe { read_u16_le(self.ptr, base + RESP_OFFSET_STATUS) };
        let len = (unsafe { read_u32_le(self.ptr, base + RESP_OFFSET_LEN) } as usize)
            .min(MAX_RESPONSE_SIZE);
        let data = unsafe {
            std::slice::from_raw_parts(self.ptr.add(base + RESP_OFFSET_DATA), len).to_vec()
        };

        // Release the slot back to the pool.
        self.state_atomic(index)
            .store(SlotState::Empty as u8, Ordering::Release);

        (status, data)
    }

    // ── Read path (Go Brain -> Rust Shield) ──────────────────────────────────

    /// Scans for the first READY slot and claims it for reading.
    ///
    /// Returns the slot index, or `None` if no READY slot is available.
    /// Intended primarily for the Go Brain runtime; also used internally
    /// by tests and the Rust benchmark harness.
    pub fn claim_read_slot(&self) -> Option<usize> {
        for i in 0..self.capacity {
            if self
                .state_atomic(i)
                .compare_exchange(
                    SlotState::Ready as u8,
                    SlotState::Reading as u8,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Some(i);
            }
        }
        None
    }

    /// Reads request data from a slot claimed by [`claim_read_slot`].
    pub fn read_request(&self, index: usize) -> RequestData {
        let base = self.slot_base(index);
        fence(Ordering::Acquire);

        let method = unsafe { read_u8(self.ptr, base + REQ_OFFSET_METHOD) };
        let path_len = (unsafe { read_u16_le(self.ptr, base + REQ_OFFSET_PATH_LEN) } as usize)
            .min(MAX_PATH_SIZE);
        let body_len = (unsafe { read_u32_le(self.ptr, base + REQ_OFFSET_BODY_LEN) } as usize)
            .min(MAX_PAYLOAD_SIZE);
        let request_id = unsafe { read_u64_le(self.ptr, base + REQ_OFFSET_REQUEST_ID) };

        let path =
            unsafe { std::slice::from_raw_parts(self.ptr.add(base + REQ_OFFSET_PATH), path_len) }
                .to_vec();
        let payload = unsafe {
            std::slice::from_raw_parts(self.ptr.add(base + REQ_OFFSET_PAYLOAD), body_len)
        }
        .to_vec();

        // Read auth_claims up to the last non-zero byte.
        let auth_claims = unsafe {
            let slice =
                std::slice::from_raw_parts(self.ptr.add(base + REQ_OFFSET_AUTH_CLAIMS), MAX_AUTH_SIZE);
            let end = slice.iter().rposition(|&b| b != 0).map(|p| p + 1).unwrap_or(0);
            slice[..end].to_vec()
        };

        RequestData { request_id, method, path, auth_claims, payload }
    }

    /// Writes response data into a claimed slot and transitions it to DONE.
    ///
    /// After this call, the Rust side unblocks from [`wait_response`].
    pub fn write_response(&self, index: usize, status: u16, data: &[u8]) {
        let base = self.slot_base(index);
        let len = data.len().min(MAX_RESPONSE_SIZE);

        unsafe {
            write_u16_le(self.ptr, base + RESP_OFFSET_STATUS, status);
            write_u32_le(self.ptr, base + RESP_OFFSET_LEN, len as u32);
            self.ptr
                .add(base + RESP_OFFSET_DATA)
                .copy_from_nonoverlapping(data.as_ptr(), len);
        }

        // Release store: response data is visible before resp_state = Done.
        self.resp_state_atomic(index)
            .store(SlotState::Done as u8, Ordering::Release);
    }

    /// Returns the number of slots in this bus segment.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Drop for MemoryBus {
    fn drop(&mut self) {
        // SAFETY: ptr and len are valid and unchanged since construction.
        // munmap is called exactly once here.
        unsafe { munmap(self.ptr as *mut _, self.len) };
        if self.owner {
            // SAFETY: shm_name is a valid C string set in the constructor.
            // shm_unlink removes the filesystem name; existing mappings survive.
            unsafe { shm_unlink(self.shm_name.as_ptr()) };
        }
    }
}

// ── Raw memory helpers ───────────────────────────────────────────────────────

#[inline(always)]
unsafe fn write_u8(base: *mut u8, offset: usize, val: u8) {
    base.add(offset).write(val);
}

#[inline(always)]
unsafe fn read_u8(base: *mut u8, offset: usize) -> u8 {
    base.add(offset).read()
}

#[inline(always)]
unsafe fn write_u16_le(base: *mut u8, offset: usize, val: u16) {
    (base.add(offset) as *mut u16).write_unaligned(val.to_le());
}

#[inline(always)]
unsafe fn read_u16_le(base: *mut u8, offset: usize) -> u16 {
    u16::from_le((base.add(offset) as *const u16).read_unaligned())
}

#[inline(always)]
unsafe fn write_u32_le(base: *mut u8, offset: usize, val: u32) {
    (base.add(offset) as *mut u32).write_unaligned(val.to_le());
}

#[inline(always)]
unsafe fn read_u32_le(base: *mut u8, offset: usize) -> u32 {
    u32::from_le((base.add(offset) as *const u32).read_unaligned())
}

#[inline(always)]
unsafe fn write_u64_le(base: *mut u8, offset: usize, val: u64) {
    (base.add(offset) as *mut u64).write_unaligned(val.to_le());
}

#[inline(always)]
unsafe fn read_u64_le(base: *mut u8, offset: usize) -> u64 {
    u64::from_le((base.add(offset) as *const u64).read_unaligned())
}

fn map_fd(fd: i32) -> Result<*mut u8, BusError> {
    // SAFETY: mmap is called with a valid fd, correct flags, and a non-zero length.
    let ptr = unsafe {
        mmap(std::ptr::null_mut(), SEGMENT_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0)
    };
    if ptr == MAP_FAILED {
        return Err(BusError::Mmap(errno()));
    }
    Ok(ptr as *mut u8)
}

fn errno() -> i32 {
    // SAFETY: __errno_location returns a valid thread-local errno pointer.
    unsafe { *libc::__errno_location() }
}

// ── Supporting types ─────────────────────────────────────────────────────────

/// Decoded request data read from a ring buffer slot.
#[derive(Debug, Clone)]
pub struct RequestData {
    pub request_id: u64,
    pub method: u8,
    pub path: Vec<u8>,
    pub auth_claims: Vec<u8>,
    pub payload: Vec<u8>,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_slots_calculation() {
        assert_eq!(MAX_SLOTS, (64 * 1024 * 1024 - 256) / 8192);
    }

    #[test]
    fn slot_base_offset_arithmetic() {
        assert_eq!(HEADER_SIZE + 0 * SLOT_SIZE, 256);
        assert_eq!(HEADER_SIZE + 1 * SLOT_SIZE, 256 + 8192);
        assert_eq!(HEADER_SIZE + 10 * SLOT_SIZE, 256 + 81920);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn create_open_write_read_roundtrip() {
        let name = "/ag-bus-test-rt";
        // Remove any leftover from a prior run.
        let cname = CString::new(name).unwrap();
        unsafe { shm_unlink(cname.as_ptr()) };

        let writer = MemoryBus::create(name).expect("create");
        let reader = MemoryBus::open(name).expect("open");

        let idx = writer.claim_write_slot().expect("claim write slot");
        writer.write_request(idx, 99, 1, b"/api/v1/ping", b"", b"{\"msg\":\"hi\"}");

        let ridx = reader.claim_read_slot().expect("claim read slot");
        assert_eq!(ridx, idx);

        let req = reader.read_request(ridx);
        assert_eq!(req.request_id, 99);
        assert_eq!(req.method, 1);
        assert_eq!(&req.path, b"/api/v1/ping");
        assert_eq!(&req.payload, b"{\"msg\":\"hi\"}");

        reader.write_response(ridx, 200, b"{\"msg\":\"pong\"}");

        let (status, body) = writer.wait_response(idx);
        assert_eq!(status, 200);
        assert_eq!(&body, b"{\"msg\":\"pong\"}");
    }
}
