package bus

import (
	"testing"
)

func TestStateConstants(t *testing.T) {
	if StateEmpty != 0 {
		t.Errorf("StateEmpty must be 0, got %d", StateEmpty)
	}
	if StateWriting != 1 {
		t.Errorf("StateWriting must be 1, got %d", StateWriting)
	}
	if StateReady != 2 {
		t.Errorf("StateReady must be 2, got %d", StateReady)
	}
	if StateReading != 3 {
		t.Errorf("StateReading must be 3, got %d", StateReading)
	}
	if StateDone != 4 {
		t.Errorf("StateDone must be 4, got %d", StateDone)
	}
}

func TestSlotOffsets(t *testing.T) {
	// Verify slot addressing arithmetic matches Rust constants.
	// slot 0 starts at byte 256 (HeaderSize).
	slot0 := HeaderSize + 0*SlotSize
	if slot0 != 256 {
		t.Errorf("slot 0 base = %d, want 256", slot0)
	}
	// slot 1 starts at 256 + 8192.
	slot1 := HeaderSize + 1*SlotSize
	if slot1 != 256+8192 {
		t.Errorf("slot 1 base = %d, want %d", slot1, 256+8192)
	}
}

func TestMaxSlotsCalculation(t *testing.T) {
	expected := (64*1024*1024 - 256) / 8192
	if MaxSlots != expected {
		t.Errorf("MaxSlots = %d, want %d", MaxSlots, expected)
	}
}

func TestLastNonZero(t *testing.T) {
	cases := []struct {
		input []byte
		want  int
	}{
		{[]byte{}, 0},
		{[]byte{0, 0, 0}, 0},
		{[]byte{1, 0, 0}, 1},
		{[]byte{0, 1, 0}, 2},
		{[]byte{1, 2, 3}, 3},
	}
	for _, c := range cases {
		got := lastNonZero(c.input)
		if got != c.want {
			t.Errorf("lastNonZero(%v) = %d, want %d", c.input, got, c.want)
		}
	}
}
