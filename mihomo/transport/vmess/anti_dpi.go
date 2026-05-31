package vmess

import (
	"crypto/rand"
	"net"
	"sync"
	"time"

	"github.com/metacubex/mihomo/common/pool"
	"github.com/metacubex/mihomo/log"

	"github.com/metacubex/randv2"
)

// AntiDPIConfig controls anti-DPI features for VMess/VLESS connections.
type AntiDPIConfig struct {
	Enabled      bool
	PaddingMode  string // "random", "size_uniform", "none"
	MinPadding   int
	MaxPadding   int
	JitterMs     int // max random delay between writes in ms
	BurstBefore  int // packets before applying jitter
	DummyTraffic bool // inject dummy traffic during idle periods
}

// DefaultAntiDPIConfig returns sensible defaults.
func DefaultAntiDPIConfig() *AntiDPIConfig {
	return &AntiDPIConfig{
		Enabled:      true,
		PaddingMode:  "random",
		MinPadding:   1,
		MaxPadding:   64,
		JitterMs:     30,
		BurstBefore:  15,
		DummyTraffic: false,
	}
}

// AntiDPIConn wraps a VMess/VLESS connection with anti-DPI features.
// It adds padding, timing jitter, and optional dummy traffic to make
// the connection pattern harder to fingerprint.
type AntiDPIConn struct {
	net.Conn
	config     *AntiDPIConfig
	mu         sync.Mutex
	writeCount int
	closed     bool
	done       chan struct{}
	once       sync.Once
}

// NewAntiDPIConn wraps a connection with anti-DPI features.
func NewAntiDPIConn(conn net.Conn, config *AntiDPIConfig) *AntiDPIConn {
	if config == nil {
		config = DefaultAntiDPIConfig()
	}
	ac := &AntiDPIConn{
		Conn:   conn,
		config: config,
		done:   make(chan struct{}),
	}

	if config.DummyTraffic {
		go ac.dummyTrafficPump()
	}

	return ac
}

// Write implements net.Conn with padding and timing jitter.
func (ac *AntiDPIConn) Write(b []byte) (int, error) {
	ac.mu.Lock()
	defer ac.mu.Unlock()

	if ac.closed {
		return 0, net.ErrClosed
	}

	// Write the actual data
	n, err := ac.Conn.Write(b)
	if err != nil {
		return n, err
	}

	ac.writeCount++

	// Apply padding after data write
	if ac.config.PaddingMode != "none" && ac.config.Enabled {
		paddingLen := ac.config.MinPadding + randv2.IntN(ac.config.MaxPadding-ac.config.MinPadding+1)
		if paddingLen > 0 {
			padding := pool.Get(paddingLen)
			_, _ = rand.Read(padding)
			_, _ = ac.Conn.Write(padding)
			pool.Put(padding)
		}
	}

	// Apply timing jitter after burst threshold
	if ac.config.JitterMs > 0 && ac.writeCount > ac.config.BurstBefore {
		jitter := time.Duration(randv2.IntN(ac.config.JitterMs)) * time.Millisecond
		if jitter > 0 {
			ac.mu.Unlock()
			time.Sleep(jitter)
			ac.mu.Lock()
		}
		// Reset counter periodically
		if ac.writeCount > ac.config.BurstBefore*3 {
			ac.writeCount = 0
		}
	}

	return n, nil
}

// Close implements net.Conn.
func (ac *AntiDPIConn) Close() error {
	ac.mu.Lock()
	defer ac.mu.Unlock()

	if ac.closed {
		return nil
	}
	ac.closed = true

	ac.once.Do(func() {
		close(ac.done)
	})

	return ac.Conn.Close()
}

// dummyTrafficPump injects small dummy packets during idle periods
// to make the connection appear as a continuous HTTPS stream.
func (ac *AntiDPIConn) dummyTrafficPump() {
	ticker := time.NewTicker(15 * time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-ac.done:
			return
		case <-ticker.C:
			ac.mu.Lock()
			if ac.closed {
				ac.mu.Unlock()
				return
			}
			// Send a small dummy frame (looks like HTTP/2 WINDOW_UPDATE or PING)
			dummy := make([]byte, 9+4) // HTTP/2 frame header + small payload
			// Frame length: 4
			dummy[0] = 0
			dummy[1] = 0
			dummy[2] = 4
			// Frame type: WINDOW_UPDATE (0x08)
			dummy[3] = 0x08
			// Flags: 0
			dummy[4] = 0
			// Stream ID: 0
			dummy[5] = 0
			dummy[6] = 0
			dummy[7] = 0
			dummy[8] = 0
			// Random increment
			_, _ = rand.Read(dummy[9:])
			_, _ = ac.Conn.Write(dummy)
			ac.mu.Unlock()

			log.Debugln("[AntiDPI] sent dummy traffic keep-alive")
		}
	}
}

// SizeUniformWriter pads writes to a uniform block size, making all
// outgoing packets the same size. This resists size-based DPI analysis.
type SizeUniformWriter struct {
	net.Conn
	blockSize int
	buf       []byte
	mu        sync.Mutex
}

// NewSizeUniformWriter creates a connection that pads writes to a uniform block size.
func NewSizeUniformWriter(conn net.Conn, blockSize int) *SizeUniformWriter {
	if blockSize <= 0 {
		blockSize = 128
	}
	return &SizeUniformWriter{
		Conn:      conn,
		blockSize: blockSize,
	}
}

// Write pads the data to a multiple of blockSize before writing.
func (su *SizeUniformWriter) Write(b []byte) (int, error) {
	su.mu.Lock()
	defer su.mu.Unlock()

	dataLen := len(b)
	remainder := dataLen % su.blockSize
	if remainder == 0 {
		return su.Conn.Write(b)
	}

	paddingLen := su.blockSize - remainder
	buf := make([]byte, dataLen+paddingLen)
	copy(buf, b)
	// Fill padding with random bytes
	_, _ = rand.Read(buf[dataLen:])

	_, err := su.Conn.Write(buf)
	if err != nil {
		return 0, err
	}
	return dataLen, nil
}
