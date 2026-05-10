// Package bus provides the Go-side implementation of the Anti-Gravital Memory
// Bus: a POSIX shared memory ring buffer for nanosecond-latency communication
// between the Rust Shield layer and the Go Brain layer.
//
// Memory layout is defined in ag-core/src/bus/ring_buffer.rs and must be kept
// in sync. All numeric fields are stored in little-endian byte order.
package bus

/*
#include <sys/mman.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>
#include <errno.h>

// Open an existing POSIX shared memory segment for read/write.
// Returns the file descriptor or -1 on error; writes errno to out_errno.
static int shm_open_rdwr(const char *name, int *out_errno) {
    int fd = shm_open(name, O_RDWR, 0);
    if (fd < 0) {
        *out_errno = errno;
    }
    return fd;
}

// Map a file descriptor into the process address space.
// Returns MAP_FAILED on error; writes errno to out_errno.
static void *map_segment(int fd, size_t size, int *out_errno) {
    void *ptr = mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (ptr == MAP_FAILED) {
        *out_errno = errno;
    }
    return ptr;
}
*/
import "C"

import (
	"encoding/binary"
	"fmt"
	"sync/atomic"
	"unsafe"
)

// Constants mirroring ag-core/src/bus/ring_buffer.rs. Any change here must
// be reflected there and vice versa.
const (
	BusDefaultName = "/ag-bus-v1"
	BusMagic       = uint64(0xAEAD_0001_0000_0001)
	BusVersion     = uint32(1)
	SegmentSize    = 64 * 1024 * 1024 // 64 MB
	HeaderSize     = 256
	SlotSize       = 8192
	MaxSlots       = (SegmentSize - HeaderSize) / SlotSize
)

// Header byte offsets.
const (
	hdrMagic     = 0
	hdrVersion   = 8
	hdrCapacity  = 12
	hdrWriteHead = 16
	hdrReadHead  = 24
)

// Request area offsets (relative to slot base).
const (
	reqOffState      = 0
	reqOffMethod     = 1
	reqOffPathLen    = 2
	reqOffBodyLen    = 4
	reqOffRequestID  = 8
	reqOffAuthClaims = 64
	reqOffPath       = 576
	reqOffPayload    = 1088

	maxAuthSize    = 512
	maxPathSize    = 512
	maxPayloadSize = 3008
)

// Response area offsets (relative to slot base).
const (
	respAreaOffset  = 4096
	respOffState    = respAreaOffset + 0
	respOffStatus   = respAreaOffset + 2
	respOffLen      = respAreaOffset + 4
	respOffData     = respAreaOffset + 64
	maxResponseSize = 4032
)

// Slot state constants mirroring ag-core/src/bus/slot.rs.
const (
	StateEmpty   = uint8(0)
	StateWriting = uint8(1)
	StateReady   = uint8(2)
	StateReading = uint8(3)
	StateDone    = uint8(4)
)

// MemoryBus holds a mapping to the POSIX shared memory segment created by the
// Rust Shield layer. The Go Brain uses it to consume incoming requests and
// deliver responses.
type MemoryBus struct {
	ptr      unsafe.Pointer
	capacity int
}

// Open maps the shared memory segment with the given POSIX name.
// The caller must call Close when done.
func Open(name string) (*MemoryBus, error) {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))

	var cerr C.int

	// CGO: shm_open opens the segment created by the Rust Shield.
	fd := C.shm_open_rdwr(cname, &cerr)
	if fd < 0 {
		return nil, fmt.Errorf("shm_open(%q): errno %d", name, int(cerr))
	}
	defer C.close(fd)

	// CGO: mmap maps the full segment into our address space.
	ptr := C.map_segment(fd, C.size_t(SegmentSize), &cerr)
	if ptr == C.MAP_FAILED {
		return nil, fmt.Errorf("mmap(%q, %d): errno %d", name, SegmentSize, int(cerr))
	}

	bus := &MemoryBus{
		ptr:      ptr,
		capacity: MaxSlots,
	}
	if err := bus.validateHeader(); err != nil {
		_ = bus.Close()
		return nil, err
	}
	return bus, nil
}

// Close unmaps the shared memory segment. It does not unlink the segment
// (only the Rust owner does that).
func (b *MemoryBus) Close() error {
	rc := C.munmap(b.ptr, C.size_t(SegmentSize))
	b.ptr = nil
	if rc != 0 {
		return fmt.Errorf("munmap failed")
	}
	return nil
}

// Capacity returns the number of slots in this bus.
func (b *MemoryBus) Capacity() int {
	return b.capacity
}

// ── Header ───────────────────────────────────────────────────────────────────

func (b *MemoryBus) validateHeader() error {
	magic := b.readU64LE(hdrMagic)
	if magic != BusMagic {
		return fmt.Errorf("bus: magic mismatch (got 0x%016X, expected 0x%016X)", magic, BusMagic)
	}
	version := b.readU32LE(hdrVersion)
	if version != BusVersion {
		return fmt.Errorf("bus: version mismatch (got %d, required %d)", version, BusVersion)
	}
	return nil
}

// ── Slot scanning ─────────────────────────────────────────────────────────────

// ClaimReadSlot scans the ring buffer for a READY slot and transitions it to
// READING via an atomic compare-and-swap. Returns the slot index, or -1 if no
// READY slot was found.
//
// The CAS operates on the 4-byte word at the start of the request area, which
// contains [state, method, path_len_lo, path_len_hi] in little-endian order.
// Only the state byte (bits 0..7) is modified; the other bytes remain intact.
func (b *MemoryBus) ClaimReadSlot() int {
	for i := 0; i < b.capacity; i++ {
		slotBase := HeaderSize + i*SlotSize
		wordPtr := (*uint32)(unsafe.Pointer(uintptr(b.ptr) + uintptr(slotBase+reqOffState)))

		for {
			old := atomic.LoadUint32(wordPtr)
			if uint8(old) != StateReady {
				break
			}
			// Replace only the state byte; preserve method and path_len bytes.
			newWord := (old & 0xFFFFFF00) | uint32(StateReading)
			if atomic.CompareAndSwapUint32(wordPtr, old, newWord) {
				return i
			}
			// Another goroutine claimed the slot; move on.
		}
	}
	return -1
}

// ── Request reading ───────────────────────────────────────────────────────────

// RequestData holds decoded request fields read from a ring buffer slot.
type RequestData struct {
	RequestID  uint64
	Method     uint8
	Path       []byte
	AuthClaims []byte
	Payload    []byte
}

// ReadRequest reads the request data from a slot that has been claimed by
// ClaimReadSlot. The caller must own the slot.
func (b *MemoryBus) ReadRequest(slotIdx int) RequestData {
	base := HeaderSize + slotIdx*SlotSize

	method := b.readU8(base + reqOffMethod)
	pathLen := int(b.readU16LE(base + reqOffPathLen))
	if pathLen > maxPathSize {
		pathLen = maxPathSize
	}
	bodyLen := int(b.readU32LE(base + reqOffBodyLen))
	if bodyLen > maxPayloadSize {
		bodyLen = maxPayloadSize
	}
	requestID := b.readU64LE(base + reqOffRequestID)

	path := b.readBytes(base+reqOffPath, pathLen)
	payload := b.readBytes(base+reqOffPayload, bodyLen)

	// Read auth_claims up to the last non-zero byte.
	authRaw := b.readBytes(base+reqOffAuthClaims, maxAuthSize)
	authLen := lastNonZero(authRaw)
	authClaims := authRaw[:authLen]

	return RequestData{
		RequestID:  requestID,
		Method:     method,
		Path:       path,
		AuthClaims: authClaims,
		Payload:    payload,
	}
}

// ── Response writing ──────────────────────────────────────────────────────────

// WriteResponse writes the response into the slot and transitions resp_state
// to DONE. The Rust Shield's wait_response spin-wait will unblock immediately.
func (b *MemoryBus) WriteResponse(slotIdx int, status uint16, data []byte) {
	base := HeaderSize + slotIdx*SlotSize

	if len(data) > maxResponseSize {
		data = data[:maxResponseSize]
	}

	b.writeU16LE(base+respOffStatus, status)
	b.writeU32LE(base+respOffLen, uint32(len(data)))
	if len(data) > 0 {
		dst := unsafe.Pointer(uintptr(b.ptr) + uintptr(base+respOffData))
		src := unsafe.Pointer(&data[0])
		C.memcpy(dst, src, C.size_t(len(data)))
	}

	// Release store: all response writes must be visible before resp_state = Done.
	// Use an atomic word operation on the first 4 bytes of the response area to
	// guarantee release semantics on the state byte.
	wordPtr := (*uint32)(unsafe.Pointer(uintptr(b.ptr) + uintptr(base+respOffState)))
	old := atomic.LoadUint32(wordPtr)
	newWord := (old & 0xFFFFFF00) | uint32(StateDone)
	atomic.StoreUint32(wordPtr, newWord)
}

// ── Raw memory helpers ────────────────────────────────────────────────────────

func (b *MemoryBus) byteAt(offset int) *byte {
	return (*byte)(unsafe.Pointer(uintptr(b.ptr) + uintptr(offset)))
}

func (b *MemoryBus) readU8(offset int) uint8 {
	return *b.byteAt(offset)
}

func (b *MemoryBus) readU16LE(offset int) uint16 {
	buf := (*[2]byte)(unsafe.Pointer(uintptr(b.ptr) + uintptr(offset)))
	return binary.LittleEndian.Uint16(buf[:])
}

func (b *MemoryBus) readU32LE(offset int) uint32 {
	buf := (*[4]byte)(unsafe.Pointer(uintptr(b.ptr) + uintptr(offset)))
	return binary.LittleEndian.Uint32(buf[:])
}

func (b *MemoryBus) readU64LE(offset int) uint64 {
	buf := (*[8]byte)(unsafe.Pointer(uintptr(b.ptr) + uintptr(offset)))
	return binary.LittleEndian.Uint64(buf[:])
}

func (b *MemoryBus) writeU16LE(offset int, val uint16) {
	buf := (*[2]byte)(unsafe.Pointer(uintptr(b.ptr) + uintptr(offset)))
	binary.LittleEndian.PutUint16(buf[:], val)
}

func (b *MemoryBus) writeU32LE(offset int, val uint32) {
	buf := (*[4]byte)(unsafe.Pointer(uintptr(b.ptr) + uintptr(offset)))
	binary.LittleEndian.PutUint32(buf[:], val)
}

func (b *MemoryBus) readBytes(offset, length int) []byte {
	if length == 0 {
		return nil
	}
	ptr := unsafe.Pointer(uintptr(b.ptr) + uintptr(offset))
	// Copy into a Go-managed slice so the caller does not hold a pointer into
	// shared memory longer than the slot lifecycle requires.
	buf := make([]byte, length)
	C.memcpy(unsafe.Pointer(&buf[0]), ptr, C.size_t(length))
	return buf
}

// lastNonZero returns the length of the slice up to and including the last
// non-zero byte, or 0 if all bytes are zero.
func lastNonZero(b []byte) int {
	for i := len(b) - 1; i >= 0; i-- {
		if b[i] != 0 {
			return i + 1
		}
	}
	return 0
}
