package trojan

import (
	"crypto/rand"
	"encoding/binary"
	"io"
	"net"
	"sync"
	"time"

	"github.com/tanzanite2025/mihomo-optimized/common/pool"
	"github.com/tanzanite2025/mihomo-optimized/log"

	"github.com/metacubex/randv2"
)

const (
	// Padding command - Trojan-Go extension for anti-DPI
	CommandPadding byte = 0x42
	// Padding header size: 1 byte padding length + padding bytes + CRLF
	minPaddingSize = 4
	maxPaddingSize = 256
	// Traffic shaping parameters
	defaultJitterMin = 0    // ms
	defaultJitterMax = 50  // ms
	defaultBurstSize = 20  // packets before shaping
)

// PaddingConfig controls anti-DPI padding behavior.
type PaddingConfig struct {
	Enabled    bool
	MinPadding int
	MaxPadding int
	// Traffic shaping
	JitterMin   int // minimum random delay in ms
	JitterMax   int // maximum random delay in ms
	BurstSize   int // packets before applying jitter
	burstCount  int // internal counter
}

// DefaultPaddingConfig returns a sensible default padding configuration.
func DefaultPaddingConfig() *PaddingConfig {
	return &PaddingConfig{
		Enabled:    true,
		MinPadding: minPaddingSize,
		MaxPadding: maxPaddingSize,
		JitterMin:  defaultJitterMin,
		JitterMax:  defaultJitterMax,
		BurstSize:  defaultBurstSize,
	}
}

// WriteHeaderWithPadding writes a Trojan header with random padding to resist
// size-based DPI analysis. The padding is inserted between the command+address
// and the final CRLF, making each header appear different in size.
//
// Format: hexPassword CRLF command socks5Addr paddingLen padding CRLF
func WriteHeaderWithPadding(w io.Writer, hexPassword [KeyLength]byte, command Command, socks5Addr []byte, config *PaddingConfig) error {
	buf := pool.GetBuffer()
	defer pool.PutBuffer(buf)

	buf.Write(hexPassword[:])
	buf.Write(crlf)

	buf.WriteByte(command)
	buf.Write(socks5Addr)

	if config != nil && config.Enabled {
		// Add random padding
		paddingLen := config.MinPadding + randv2.IntN(config.MaxPadding-config.MinPadding)
		buf.WriteByte(byte(paddingLen))
		padding := make([]byte, paddingLen)
		_, _ = rand.Read(padding)
		buf.Write(padding)
	}

	buf.Write(crlf)

	_, err := w.Write(buf.Bytes())
	return err
}

// ReadPadding reads and discards padding bytes from a Trojan header.
// Returns the number of padding bytes consumed.
func ReadPadding(r io.Reader) (int, error) {
	paddingLenByte := make([]byte, 1)
	if _, err := io.ReadFull(r, paddingLenByte); err != nil {
		return 0, err
	}
	paddingLen := int(paddingLenByte[0])
	if paddingLen == 0 {
		return 0, nil
	}
	padding := make([]byte, paddingLen)
	if _, err := io.ReadFull(r, padding); err != nil {
		return paddingLen, err
	}
	return paddingLen, nil
}

// ShapedConn wraps a net.Conn with traffic shaping to resist timing-based DPI.
// It adds random jitter to writes after a burst threshold, making traffic
// patterns less distinguishable from normal HTTPS traffic.
type ShapedConn struct {
	net.Conn
	config *PaddingConfig
	mu     sync.Mutex
}

// NewShapedConn creates a traffic-shaped connection wrapper.
func NewShapedConn(conn net.Conn, config *PaddingConfig) *ShapedConn {
	if config == nil {
		config = DefaultPaddingConfig()
	}
	return &ShapedConn{
		Conn:   conn,
		config: config,
	}
}

// Write implements net.Conn with optional traffic shaping.
func (sc *ShapedConn) Write(b []byte) (int, error) {
	sc.mu.Lock()
	defer sc.mu.Unlock()

	n, err := sc.Conn.Write(b)
	if err != nil {
		return n, err
	}

	// Apply jitter after burst threshold
	if sc.config.JitterMax > 0 {
		sc.config.burstCount++
		if sc.config.burstCount > sc.config.BurstSize {
			jitter := time.Duration(sc.config.JitterMin+randv2.IntN(sc.config.JitterMax-sc.config.JitterMin)) * time.Millisecond
			if jitter > 0 {
				time.Sleep(jitter)
			}
			// Reset burst counter periodically
			if sc.config.burstCount > sc.config.BurstSize*2 {
				sc.config.burstCount = 0
			}
		}
	}

	return n, nil
}

// CamouflageConn wraps a Trojan connection to make its traffic pattern
// look more like normal HTTPS. It injects dummy keep-alive packets
// and adds inter-packet timing noise.
type CamouflageConn struct {
	net.Conn
	config      *PaddingConfig
	mu          sync.Mutex
	closed      bool
	lastWrite   time.Time
	keepAlive   time.Duration
	done        chan struct{}
	once        sync.Once
}

// NewCamouflageConn creates a connection with HTTPS-like camouflage.
func NewCamouflageConn(conn net.Conn, config *PaddingConfig) *CamouflageConn {
	cc := &CamouflageConn{
		Conn:      conn,
		config:    config,
		keepAlive: 30 * time.Second, // mimic HTTPS keep-alive
		done:      make(chan struct{}),
	}

	// Start background keep-alive pinger
	go cc.keepAlivePinger()

	return cc
}

func (cc *CamouflageConn) keepAlivePinger() {
	ticker := time.NewTicker(cc.keepAlive)
	defer ticker.Stop()

	for {
		select {
		case <-cc.done:
			return
		case <-ticker.C:
			cc.mu.Lock()
			if cc.closed {
				cc.mu.Unlock()
				return
			}
			// Send a small dummy packet that looks like an HTTP/2 PING frame
			// This makes the connection appear as a long-lived HTTPS session
			frame := make([]byte, 9+8) // HTTP/2 PING frame: 9 byte header + 8 byte opaque data
			// Frame length: 8
			frame[0] = 0
			frame[1] = 0
			frame[2] = 8
			// Frame type: PING (0x06)
			frame[3] = 0x06
			// Flags: 0
			frame[4] = 0
			// Stream ID: 0
			frame[5] = 0
			frame[6] = 0
			frame[7] = 0
			frame[8] = 0
			// Opaque data: random
			_, _ = rand.Read(frame[9:])
			_, _ = cc.Conn.Write(frame)
			cc.mu.Unlock()
		}
	}
}

// Write implements net.Conn with camouflage.
func (cc *CamouflageConn) Write(b []byte) (int, error) {
	cc.mu.Lock()
	defer cc.mu.Unlock()

	if cc.closed {
		return 0, net.ErrClosed
	}

	n, err := cc.Conn.Write(b)
	cc.lastWrite = time.Now()

	return n, err
}

// Close implements net.Conn.
func (cc *CamouflageConn) Close() error {
	cc.mu.Lock()
	defer cc.mu.Unlock()

	if cc.closed {
		return nil
	}
	cc.closed = true

	cc.once.Do(func() {
		close(cc.done)
	})

	return cc.Conn.Close()
}

// writePacketWithPadding writes a Trojan UDP packet with random padding.
func writePacketWithPadding(w io.Writer, socks5Addr, payload []byte, config *PaddingConfig) (int, error) {
	buf := pool.GetBuffer()
	defer pool.PutBuffer(buf)

	buf.Write(socks5Addr)
	binary.Write(buf, binary.BigEndian, uint16(len(payload)))
	buf.Write(crlf)
	buf.Write(payload)

	if config != nil && config.Enabled {
		// Add trailing padding to obscure packet size
		paddingLen := config.MinPadding + randv2.IntN(config.MaxPadding-config.MinPadding)
		buf.WriteByte(byte(paddingLen))
		padding := make([]byte, paddingLen)
		_, _ = rand.Read(padding)
		buf.Write(padding)
	}

	return w.Write(buf.Bytes())
}

// IsPaddingEnabled checks if padding is configured and enabled.
func IsPaddingEnabled(config *PaddingConfig) bool {
	return config != nil && config.Enabled
}

// ApplyJitter adds a random delay to resist timing analysis.
func ApplyJitter(config *PaddingConfig) {
	if config == nil || config.JitterMax <= 0 {
		return
	}
	jitter := time.Duration(config.JitterMin+randv2.IntN(config.JitterMax-config.JitterMin)) * time.Millisecond
	if jitter > 0 {
		log.Debugln("[Trojan] applying jitter: %v", jitter)
		time.Sleep(jitter)
	}
}
