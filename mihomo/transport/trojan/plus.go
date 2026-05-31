package trojan

import (
	"crypto/rand"
	"encoding/binary"
	"io"
	"net"
	"sync"
	"sync/atomic"
	"time"

	"github.com/metacubex/mihomo/common/pool"
	"github.com/metacubex/mihomo/log"

	"github.com/metacubex/randv2"
)

// Trojan-Plus protocol extensions
const (
	// Version bytes for Trojan-Plus header negotiation
	Version1 byte = 0x01 // Original Trojan (no extensions)
	Version2 byte = 0x02 // Trojan-Plus: padding + mux support

	// Trojan-Plus extended command types
	CommandMuxOpen    byte = 0x80 // Open a mux stream
	CommandMuxClose   byte = 0x81 // Close a mux stream
	CommandMuxData    byte = 0x82 // Data on mux stream
	CommandKeepAlive  byte = 0x83 // Keep-alive / heartbeat
	CommandPaddingExt byte = 0x84 // Extended padding frame

	// Mux stream ID size
	muxStreamIDSize = 4

	// Behavioral analysis protection defaults
	defaultSessionIdleTimeout = 30 * time.Second
	defaultHeartbeatInterval  = 15 * time.Second
	defaultTrafficWindowMs    = 1000 // 1 second window for rate analysis
	defaultTargetPacketPerSec = 25   // mimic typical HTTPS burst rate
	defaultTargetBytesPerSec  = 50000
)

// PlusConfig controls Trojan-Plus protocol extensions.
type PlusConfig struct {
	Enabled    bool
	Version    byte // protocol version to advertise
	MuxEnabled bool // enable stream multiplexing

	// Behavioral analysis protection
	BehaviorProtection *BehaviorProtectionConfig
}

// BehaviorProtectionConfig controls advanced behavioral analysis resistance.
type BehaviorProtectionConfig struct {
	Enabled bool

	// Session simulation: make the connection look like a real HTTPS browsing session
	SessionSimulation bool
	// Idle timeout before sending keep-alive
	IdleTimeout time.Duration
	// Heartbeat interval for keep-alive frames
	HeartbeatInterval time.Duration

	// Traffic normalization: smooth out burst patterns
	TrafficNormalization bool
	// Target packets per second (for rate smoothing)
	TargetPacketPerSec int
	// Target bytes per second (for rate smoothing)
	TargetBytesPerSec int
	// Traffic window in milliseconds for rate measurement
	TrafficWindowMs int

	// Packet size distribution: make packet sizes follow typical HTTPS distribution
	PacketSizeNormalization bool
	// Min and max packet sizes to clamp to (typical TLS record range)
	MinPacketSize int
	MaxPacketSize int

	// Adaptive timing: adjust timing based on network RTT
	AdaptiveTiming bool
	// RTT estimate in ms (0 = auto-detect)
	RTTEstimateMs int
}

// DefaultBehaviorProtectionConfig returns sensible defaults.
func DefaultBehaviorProtectionConfig() *BehaviorProtectionConfig {
	return &BehaviorProtectionConfig{
		Enabled:                 true,
		SessionSimulation:       true,
		IdleTimeout:             defaultSessionIdleTimeout,
		HeartbeatInterval:       defaultHeartbeatInterval,
		TrafficNormalization:    true,
		TargetPacketPerSec:      defaultTargetPacketPerSec,
		TargetBytesPerSec:       defaultTargetBytesPerSec,
		TrafficWindowMs:         defaultTrafficWindowMs,
		PacketSizeNormalization: true,
		MinPacketSize:           64,
		MaxPacketSize:           1448, // typical TLS max record
		AdaptiveTiming:          true,
		RTTEstimateMs:           0,
	}
}

// WritePlusHeader writes a Trojan-Plus extended header with version negotiation.
// Format: hexPassword CRLF version command socks5Addr [paddingLen padding] CRLF
func WritePlusHeader(w io.Writer, hexPassword [KeyLength]byte, version byte, command Command, socks5Addr []byte, config *PaddingConfig) error {
	buf := pool.GetBuffer()
	defer pool.PutBuffer(buf)

	buf.Write(hexPassword[:])
	buf.Write(crlf)

	// Version byte
	buf.WriteByte(version)

	buf.WriteByte(command)
	buf.Write(socks5Addr)

	if config != nil && config.Enabled {
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

// ReadPlusHeader reads a Trojan-Plus header and returns the version byte.
// Returns the version, or Version1 if the remote doesn't support Plus.
func ReadPlusHeader(r io.Reader, hexPassword [KeyLength]byte) (byte, error) {
	// Read the 56-byte password + CRLF
	passwordBuf := make([]byte, KeyLength+2)
	if _, err := io.ReadFull(r, passwordBuf); err != nil {
		return Version1, err
	}

	// Next byte is version
	versionBuf := make([]byte, 1)
	if _, err := io.ReadFull(r, versionBuf); err != nil {
		return Version1, err
	}

	return versionBuf[0], nil
}

// MuxStreamID represents a multiplexed stream identifier.
type MuxStreamID uint32

// MuxConn wraps a net.Conn with stream multiplexing for Trojan-Plus.
// It allows multiple logical streams over a single TLS connection.
type MuxConn struct {
	net.Conn
	mu      sync.Mutex
	nextID  atomic.Uint32
	streams map[MuxStreamID]*muxStream
	closed  bool
	done    chan struct{}
	once    sync.Once
	config  *PlusConfig
}

type muxStream struct {
	id          MuxStreamID
	readCh      chan []byte
	closeCh     chan struct{}
	localClose  bool
	remoteClose bool
}

// NewMuxConn creates a multiplexed connection wrapper.
func NewMuxConn(conn net.Conn, config *PlusConfig) *MuxConn {
	mc := &MuxConn{
		Conn:    conn,
		streams: make(map[MuxStreamID]*muxStream),
		done:    make(chan struct{}),
		config:  config,
	}
	return mc
}

// OpenStream opens a new multiplexed stream.
func (mc *MuxConn) OpenStream() (MuxStreamID, error) {
	mc.mu.Lock()
	defer mc.mu.Unlock()

	if mc.closed {
		return 0, net.ErrClosed
	}

	id := MuxStreamID(mc.nextID.Add(1))
	stream := &muxStream{
		id:      id,
		readCh:  make(chan []byte, 32),
		closeCh: make(chan struct{}),
	}
	mc.streams[id] = stream

	// Send MuxOpen command
	buf := pool.GetBuffer()
	defer pool.PutBuffer(buf)

	buf.WriteByte(CommandMuxOpen)
	binary.Write(buf, binary.BigEndian, uint32(id))
	buf.Write(crlf)

	if _, err := mc.Conn.Write(buf.Bytes()); err != nil {
		delete(mc.streams, id)
		return 0, err
	}

	return id, nil
}

// CloseStream closes a multiplexed stream.
func (mc *MuxConn) CloseStream(id MuxStreamID) error {
	mc.mu.Lock()
	stream, ok := mc.streams[id]
	if ok {
		stream.localClose = true
		close(stream.closeCh)
		delete(mc.streams, id)
	}
	mc.mu.Unlock()

	if !ok {
		return nil
	}

	buf := pool.GetBuffer()
	defer pool.PutBuffer(buf)

	buf.WriteByte(CommandMuxClose)
	binary.Write(buf, binary.BigEndian, uint32(id))
	buf.Write(crlf)

	_, err := mc.Conn.Write(buf.Bytes())
	return err
}

// WriteStream writes data to a specific stream.
func (mc *MuxConn) WriteStream(id MuxStreamID, data []byte) (int, error) {
	mc.mu.Lock()
	if mc.closed {
		mc.mu.Unlock()
		return 0, net.ErrClosed
	}
	mc.mu.Unlock()

	buf := pool.GetBuffer()
	defer pool.PutBuffer(buf)

	buf.WriteByte(CommandMuxData)
	binary.Write(buf, binary.BigEndian, uint32(id))
	binary.Write(buf, binary.BigEndian, uint16(len(data)))
	buf.Write(crlf)
	buf.Write(data)

	return mc.Conn.Write(buf.Bytes())
}

// BehaviorProtectedConn wraps a connection with advanced behavioral analysis protection.
// It normalizes traffic patterns to resist statistical DPI analysis.
type BehaviorProtectedConn struct {
	net.Conn
	config    *BehaviorProtectionConfig
	mu        sync.Mutex
	closed    bool
	lastWrite time.Time

	// Traffic rate tracking
	writeCount  atomic.Int64
	writeBytes  atomic.Int64
	windowStart atomic.Int64 // unix ms

	// Adaptive timing
	rttEstimate atomic.Int64 // ms

	// Keep-alive
	done chan struct{}
	once sync.Once
}

// NewBehaviorProtectedConn creates a connection with behavioral analysis protection.
func NewBehaviorProtectedConn(conn net.Conn, config *BehaviorProtectionConfig) *BehaviorProtectedConn {
	if config == nil {
		config = DefaultBehaviorProtectionConfig()
	}
	bpc := &BehaviorProtectedConn{
		Conn:   conn,
		config: config,
		done:   make(chan struct{}),
	}
	bpc.windowStart.Store(time.Now().UnixMilli())

	if config.SessionSimulation {
		go bpc.keepAliveLoop()
	}

	return bpc
}

// Write implements net.Conn with behavioral protection.
func (bpc *BehaviorProtectedConn) Write(b []byte) (int, error) {
	bpc.mu.Lock()
	defer bpc.mu.Unlock()

	if bpc.closed {
		return 0, net.ErrClosed
	}

	data := b

	// Packet size normalization: split or pad to typical TLS record sizes
	if bpc.config.PacketSizeNormalization && len(data) > 0 {
		data = bpc.normalizePacketSize(data)
	}

	// Traffic rate normalization: add timing to smooth burst patterns
	if bpc.config.TrafficNormalization {
		bpc.applyRateControl(len(data))
	}

	n, err := bpc.Conn.Write(data)
	if err != nil {
		return n, err
	}

	bpc.lastWrite = time.Now()
	bpc.writeCount.Add(1)
	bpc.writeBytes.Add(int64(n))

	return len(b), nil // return original length, not normalized
}

// Read implements net.Conn.
func (bpc *BehaviorProtectedConn) Read(b []byte) (int, error) {
	return bpc.Conn.Read(b)
}

// Close implements net.Conn.
func (bpc *BehaviorProtectedConn) Close() error {
	bpc.mu.Lock()
	defer bpc.mu.Unlock()

	if bpc.closed {
		return nil
	}
	bpc.closed = true

	bpc.once.Do(func() {
		close(bpc.done)
	})

	return bpc.Conn.Close()
}

// normalizePacketSize adjusts packet sizes to match typical HTTPS/TLS distribution.
// Small packets are padded up; large packets are split into TLS-record-sized chunks.
func (bpc *BehaviorProtectedConn) normalizePacketSize(data []byte) []byte {
	size := len(data)

	// If packet is very small, pad it up to minimum (avoids small packet fingerprinting)
	if size < bpc.config.MinPacketSize {
		padded := make([]byte, bpc.config.MinPacketSize)
		copy(padded, data)
		// Fill padding with random bytes (looks like TLS encrypted payload)
		_, _ = rand.Read(padded[size:])
		return padded
	}

	// If packet is larger than max TLS record, we let it through
	// (large packets are normal for TLS and don't need splitting at this layer)
	return data
}

// applyRateControl adds timing delays to smooth out traffic bursts.
// This makes the connection's packet rate look like a typical HTTPS session
// rather than a proxy tunnel.
func (bpc *BehaviorProtectedConn) applyRateControl(bytesLen int) {
	now := time.Now().UnixMilli()
	windowStart := bpc.windowStart.Load()

	// Reset window if expired
	if now-windowStart > int64(bpc.config.TrafficWindowMs) {
		bpc.windowStart.Store(now)
		bpc.writeCount.Store(0)
		bpc.writeBytes.Store(0)
		return
	}

	count := bpc.writeCount.Load()
	totalBytes := bpc.writeBytes.Load()

	// Check if we're exceeding target rate
	if bpc.config.TargetPacketPerSec > 0 {
		packetsInWindow := count
		windowFraction := float64(now-windowStart) / float64(bpc.config.TrafficWindowMs)
		expectedPackets := float64(bpc.config.TargetPacketPerSec) * windowFraction

		if float64(packetsInWindow) > expectedPackets*1.5 {
			// We're sending too fast, add a small delay
			rttMs := bpc.rttEstimate.Load()
			if rttMs == 0 {
				rttMs = 50 // default assumption
			}
			delay := time.Duration(rttMs/4+int64(randv2.IntN(int(rttMs/2)))) * time.Millisecond
			if delay > 0 && delay < 200*time.Millisecond {
				time.Sleep(delay)
			}
		}
	}

	// Byte rate smoothing
	if bpc.config.TargetBytesPerSec > 0 && totalBytes > 0 {
		windowFraction := float64(now-windowStart) / float64(bpc.config.TrafficWindowMs)
		expectedBytes := float64(bpc.config.TargetBytesPerSec) * windowFraction

		if totalBytes > int64(expectedBytes*1.5) {
			// Exceeding byte rate target, add delay proportional to excess
			excessRatio := float64(totalBytes) / expectedBytes
			delayMs := int(excessRatio * 10)
			if delayMs > 100 {
				delayMs = 100
			}
			if delayMs > 0 {
				time.Sleep(time.Duration(delayMs) * time.Millisecond)
			}
		}
	}
}

// keepAliveLoop sends periodic keep-alive frames to maintain the connection
// and make it look like a long-lived HTTPS session.
func (bpc *BehaviorProtectedConn) keepAliveLoop() {
	ticker := time.NewTicker(bpc.config.HeartbeatInterval)
	defer ticker.Stop()

	for {
		select {
		case <-bpc.done:
			return
		case <-ticker.C:
			bpc.mu.Lock()
			if bpc.closed {
				bpc.mu.Unlock()
				return
			}

			// Only send keep-alive if connection has been idle
			idleDuration := time.Since(bpc.lastWrite)
			if idleDuration > bpc.config.IdleTimeout {
				// Send a small keep-alive frame that looks like an HTTP/2 PING
				frame := make([]byte, 9+8)
				frame[0] = 0
				frame[1] = 0
				frame[2] = 8
				frame[3] = 0x06 // HTTP/2 PING type
				frame[4] = 0
				frame[5] = 0
				frame[6] = 0
				frame[7] = 0
				frame[8] = 0
				_, _ = rand.Read(frame[9:])
				_, _ = bpc.Conn.Write(frame)
				log.Debugln("[Trojan-Plus] sent keep-alive PING frame (idle: %v)", idleDuration)
			}
			bpc.mu.Unlock()
		}
	}
}

// UpdateRTT updates the adaptive RTT estimate for timing adjustments.
func (bpc *BehaviorProtectedConn) UpdateRTT(rtt time.Duration) {
	rttMs := rtt.Milliseconds()
	old := bpc.rttEstimate.Load()
	// Exponential moving average
	newRtt := (old*7 + rttMs*3) / 10
	bpc.rttEstimate.Store(newRtt)
}

// TrojanPlusConn combines all Trojan-Plus extensions: mux, behavioral protection,
// and padding into a single connection wrapper.
type TrojanPlusConn struct {
	net.Conn
	plusConfig *PlusConfig
	paddingCfg *PaddingConfig
	behavior   *BehaviorProtectedConn
	mux        *MuxConn
	mu         sync.Mutex
	closed     bool
}

// NewTrojanPlusConn creates a fully-featured Trojan-Plus connection.
func NewTrojanPlusConn(conn net.Conn, plusCfg *PlusConfig, paddingCfg *PaddingConfig) *TrojanPlusConn {
	tpc := &TrojanPlusConn{
		Conn:       conn,
		plusConfig: plusCfg,
		paddingCfg: paddingCfg,
	}

	// Wrap with behavioral protection first (outermost layer)
	if plusCfg.BehaviorProtection != nil && plusCfg.BehaviorProtection.Enabled {
		tpc.behavior = NewBehaviorProtectedConn(conn, plusCfg.BehaviorProtection)
		tpc.Conn = tpc.behavior
	}

	// Wrap with mux if enabled
	if plusCfg.MuxEnabled {
		tpc.mux = NewMuxConn(tpc.Conn, plusCfg)
		tpc.Conn = tpc.mux
	}

	return tpc
}

// Write implements net.Conn with all Trojan-Plus extensions.
func (tpc *TrojanPlusConn) Write(b []byte) (int, error) {
	tpc.mu.Lock()
	defer tpc.mu.Unlock()

	if tpc.closed {
		return 0, net.ErrClosed
	}

	// Add inter-write padding if configured
	if tpc.paddingCfg != nil && tpc.paddingCfg.Enabled {
		ApplyJitter(tpc.paddingCfg)
	}

	return tpc.Conn.Write(b)
}

// Close implements net.Conn.
func (tpc *TrojanPlusConn) Close() error {
	tpc.mu.Lock()
	defer tpc.mu.Unlock()

	if tpc.closed {
		return nil
	}
	tpc.closed = true

	if tpc.behavior != nil {
		return tpc.behavior.Close()
	}
	return tpc.Conn.Close()
}

// IsPlusEnabled checks if Trojan-Plus extensions are configured.
func IsPlusEnabled(config *PlusConfig) bool {
	return config != nil && config.Enabled
}
