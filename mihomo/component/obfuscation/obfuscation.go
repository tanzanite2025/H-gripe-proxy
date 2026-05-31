package obfuscation

import (
	"crypto/rand"
	"io"
	"net"
	"sync"
	"sync/atomic"
	"time"

	"github.com/tanzanite2025/mihomo-optimized/log"

	"github.com/metacubex/randv2"
)

// ============================================================
// Problem 1: Traffic Feature Concealment
// ============================================================

// TrafficObfuscationConfig controls traffic feature concealment.
type TrafficObfuscationConfig struct {
	Enabled bool

	// Protocol feature randomization
	ProtocolRandomization bool
	// Packet size randomization
	PacketSizeRandomization bool
	// Min/max padding size for packet randomization
	MinPaddingSize int
	MaxPaddingSize int
	// Timing randomization
	TimingRandomization bool
	// Min/max jitter in milliseconds
	JitterMinMs int
	JitterMaxMs int
	// Direction feature concealment
	DirectionConcealment bool
	// Statistical feature concealment (smooth traffic rates)
	StatisticalConcealment bool
	// Target bytes per second for rate smoothing
	TargetBytesPerSec int
}

// DefaultTrafficObfuscationConfig returns sensible defaults.
func DefaultTrafficObfuscationConfig() *TrafficObfuscationConfig {
	return &TrafficObfuscationConfig{
		Enabled:                 true,
		ProtocolRandomization:   true,
		PacketSizeRandomization: true,
		MinPaddingSize:          1,
		MaxPaddingSize:          64,
		TimingRandomization:     true,
		JitterMinMs:             0,
		JitterMaxMs:             30,
		DirectionConcealment:    true,
		StatisticalConcealment:  true,
		TargetBytesPerSec:       50000,
	}
}

// ObfuscatedConn wraps a connection with traffic feature concealment.
type ObfuscatedConn struct {
	net.Conn
	config     *TrafficObfuscationConfig
	mu         sync.Mutex
	closed     bool
	lastWrite  time.Time
	writeCount atomic.Int64
	writeBytes atomic.Int64
	// Rate tracking window
	windowStart atomic.Int64 // unix ms
	windowBytes atomic.Int64
}

// NewObfuscatedConn creates a connection with traffic obfuscation.
func NewObfuscatedConn(conn net.Conn, config *TrafficObfuscationConfig) *ObfuscatedConn {
	if config == nil {
		config = DefaultTrafficObfuscationConfig()
	}
	oc := &ObfuscatedConn{
		Conn:   conn,
		config: config,
	}
	oc.windowStart.Store(time.Now().UnixMilli())
	return oc
}

// Write implements net.Conn with traffic obfuscation.
func (oc *ObfuscatedConn) Write(b []byte) (int, error) {
	oc.mu.Lock()
	defer oc.mu.Unlock()

	if oc.closed {
		return 0, net.ErrClosed
	}

	// Apply timing randomization (jitter)
	if oc.config.TimingRandomization {
		oc.applyJitter()
	}

	// Apply statistical rate smoothing
	if oc.config.StatisticalConcealment {
		oc.applyRateSmoothing(len(b))
	}

	// Apply packet size randomization (add random padding)
	data := b
	if oc.config.PacketSizeRandomization && len(b) > 0 {
		data = oc.addPadding(b)
	}

	n, err := oc.Conn.Write(data)
	if err != nil {
		// 如果部分写入，返回原始数据长度的对应比例
		// 因为 data 可能包含 padding，需要计算实际写入的原始字节数
		if n > 0 && len(data) != len(b) {
			// data = [1 byte padLen][b][padding]
			// n 字节已写入 data，计算其中有多少属于原始数据 b
			if n > 1 {
				origWritten := n - 1 // 减去 padLen 字节
				if origWritten > len(b) {
					origWritten = len(b) // 超出 b 的部分是 padding
				}
				if origWritten < 0 {
					origWritten = 0
				}
				oc.writeBytes.Add(int64(origWritten))
				oc.windowBytes.Add(int64(origWritten))
				return origWritten, err
			}
			return 0, err
		}
		return n, err
	}

	oc.lastWrite = time.Now()
	oc.writeCount.Add(1)
	oc.writeBytes.Add(int64(len(b)))
	oc.windowBytes.Add(int64(len(b)))

	return len(b), nil // return original length
}

// Read implements net.Conn.
// 注意：Write 方向的 padding 格式 [1 byte padLen][data][padding] 仅用于出站流量。
// 入站流量（服务端响应）不包含此格式，因此 Read 不做 padding 剥离。
// 即使对端也使用同样的混淆引擎，TCP 流式传输也无法保证单次 Read 恰好对齐一个帧，
// 盲目剥离会损坏非 padding 数据。如需双向 padding，必须引入帧定界符或长度前缀。
func (oc *ObfuscatedConn) Read(b []byte) (int, error) {
	return oc.Conn.Read(b)
}

// Close implements net.Conn.
func (oc *ObfuscatedConn) Close() error {
	oc.mu.Lock()
	defer oc.mu.Unlock()

	if oc.closed {
		return nil
	}
	oc.closed = true
	return oc.Conn.Close()
}

// addPadding adds random padding bytes to the packet to randomize size.
func (oc *ObfuscatedConn) addPadding(data []byte) []byte {
	minPad := oc.config.MinPaddingSize
	maxPad := oc.config.MaxPaddingSize
	if maxPad <= minPad {
		return data
	}

	paddingLen := minPad + randv2.IntN(maxPad-minPad)
	if paddingLen == 0 {
		return data
	}
	// padLen 字段只有 1 字节，最大 255，超过会静默截断导致帧格式损坏
	if paddingLen > 255 {
		paddingLen = 255
	}

	// Format: [1 byte padding length][data][padding bytes]
	// This allows the receiver to strip padding if needed
	result := make([]byte, 1+len(data)+paddingLen)
	result[0] = byte(paddingLen)
	copy(result[1:], data)
	// Fill padding with random bytes
	_, _ = rand.Read(result[1+len(data):])
	return result
}

// applyJitter adds a random delay before writing to randomize timing.
func (oc *ObfuscatedConn) applyJitter() {
	minMs := oc.config.JitterMinMs
	maxMs := oc.config.JitterMaxMs
	if maxMs <= minMs {
		return
	}

	jitterMs := minMs + randv2.IntN(maxMs-minMs)
	if jitterMs > 0 {
		time.Sleep(time.Duration(jitterMs) * time.Millisecond)
	}
}

// applyRateSmoothing adds delays to smooth out traffic rate to avoid burst detection.
func (oc *ObfuscatedConn) applyRateSmoothing(dataLen int) {
	if oc.config.TargetBytesPerSec <= 0 {
		return
	}

	now := time.Now().UnixMilli()
	windowStart := oc.windowStart.Load()
	windowBytes := oc.windowBytes.Load()

	// Reset window every second
	if now-windowStart > 1000 {
		oc.windowStart.Store(now)
		oc.windowBytes.Store(int64(dataLen))
		return
	}

	// Check if we're exceeding target rate
	windowFraction := float64(now-windowStart) / 1000.0
	expectedBytes := float64(oc.config.TargetBytesPerSec) * windowFraction

	if float64(windowBytes) > expectedBytes*1.5 {
		// Add proportional delay
		excessRatio := float64(windowBytes) / expectedBytes
		delayMs := int(excessRatio * 5)
		if delayMs > 50 {
			delayMs = 50
		}
		if delayMs > 0 {
			time.Sleep(time.Duration(delayMs) * time.Millisecond)
		}
	}
}

// ============================================================
// Problem 2: Behavioral Analysis Protection
// ============================================================

// BehaviorProtectionConfig controls behavioral analysis protection.
type BehaviorProtectionConfig struct {
	Enabled bool

	// Connection pattern randomization
	ConnectionPatternRandomization bool
	// Min/max connection interval in ms
	MinConnIntervalMs int
	MaxConnIntervalMs int

	// Usage time simulation - mimic typical browsing patterns
	UsageTimeSimulation bool
	// Active periods (hours in day, 0-23)
	ActiveHours []int

	// Application diversity - vary traffic patterns to look like mixed apps
	AppDiversity bool
	// Simulated app profiles
	AppProfiles []AppProfile

	// Geographic consistency
	GeoConsistency bool
	// Expected timezone offset (hours from UTC)
	TimezoneOffset int

	// Timezone consistency
	TimezoneConsistency bool
}

// AppProfile describes a simulated application traffic pattern.
type AppProfile struct {
	Name            string
	AvgPacketSize   int
	BurstSize       int
	BurstIntervalMs int
}

// DefaultBehaviorProtectionConfig returns sensible defaults.
func DefaultBehaviorProtectionConfig() *BehaviorProtectionConfig {
	return &BehaviorProtectionConfig{
		Enabled:                        true,
		ConnectionPatternRandomization: true,
		MinConnIntervalMs:              50,
		MaxConnIntervalMs:              500,
		UsageTimeSimulation:            true,
		ActiveHours:                    []int{8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22},
		AppDiversity:                   true,
		AppProfiles: []AppProfile{
			{Name: "browser", AvgPacketSize: 800, BurstSize: 10, BurstIntervalMs: 200},
			{Name: "streaming", AvgPacketSize: 1400, BurstSize: 50, BurstIntervalMs: 20},
			{Name: "messaging", AvgPacketSize: 200, BurstSize: 3, BurstIntervalMs: 1000},
		},
		GeoConsistency:      true,
		TimezoneOffset:      8, // UTC+8 default
		TimezoneConsistency: true,
	}
}

// BehaviorProtectedConn wraps a connection with behavioral analysis protection.
type BehaviorProtectedConn struct {
	net.Conn
	config       *BehaviorProtectionConfig
	mu           sync.Mutex
	closed       bool
	lastConnTime time.Time
	connCount    atomic.Int64
	currentApp   int // index into AppProfiles
}

// NewBehaviorProtectedConn creates a connection with behavioral protection.
func NewBehaviorProtectedConn(conn net.Conn, config *BehaviorProtectionConfig) *BehaviorProtectedConn {
	if config == nil {
		config = DefaultBehaviorProtectionConfig()
	}
	bpc := &BehaviorProtectedConn{
		Conn:       conn,
		config:     config,
		currentApp: randv2.IntN(len(config.AppProfiles)),
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

	// Apply connection pattern randomization
	if bpc.config.ConnectionPatternRandomization {
		bpc.applyConnectionJitter()
	}

	// Apply app diversity pattern
	if bpc.config.AppDiversity && len(bpc.config.AppProfiles) > 0 {
		bpc.applyAppPattern(len(b))
	}

	// Check timezone consistency
	if bpc.config.TimezoneConsistency {
		bpc.checkTimezoneConsistency()
	}

	n, err := bpc.Conn.Write(b)
	if err != nil {
		return n, err
	}

	bpc.lastConnTime = time.Now()
	bpc.connCount.Add(1)
	return n, nil
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
	return bpc.Conn.Close()
}

// applyConnectionJitter adds random delay between connections.
func (bpc *BehaviorProtectedConn) applyConnectionJitter() {
	if !bpc.lastConnTime.IsZero() {
		minMs := bpc.config.MinConnIntervalMs
		maxMs := bpc.config.MaxConnIntervalMs
		if maxMs > minMs {
			delayMs := minMs + randv2.IntN(maxMs-minMs)
			if delayMs > 0 {
				time.Sleep(time.Duration(delayMs) * time.Millisecond)
			}
		}
	}
}

// applyAppPattern varies traffic patterns based on simulated app profiles.
func (bpc *BehaviorProtectedConn) applyAppPattern(dataLen int) {
	profile := bpc.config.AppProfiles[bpc.currentApp]

	// Occasionally switch app profiles to simulate diverse usage
	if randv2.IntN(100) < 5 { // 5% chance per write
		bpc.currentApp = randv2.IntN(len(bpc.config.AppProfiles))
		profile = bpc.config.AppProfiles[bpc.currentApp]
		log.Debugln("[BehaviorProtection] switching to app profile: %s", profile.Name)
	}

	// Add burst-based timing
	if profile.BurstIntervalMs > 0 && profile.BurstSize > 0 {
		connCount := bpc.connCount.Load()
		if connCount%int64(profile.BurstSize) == 0 && connCount > 0 {
			time.Sleep(time.Duration(profile.BurstIntervalMs) * time.Millisecond)
		}
	}
}

// checkTimezoneConsistency logs warnings if activity occurs outside expected timezone.
func (bpc *BehaviorProtectedConn) checkTimezoneConsistency() {
	if !bpc.config.UsageTimeSimulation || len(bpc.config.ActiveHours) == 0 {
		return
	}

	now := time.Now()
	localHour := (now.UTC().Hour() + bpc.config.TimezoneOffset + 24) % 24

	active := false
	for _, h := range bpc.config.ActiveHours {
		if h == localHour {
			active = true
			break
		}
	}

	if !active {
		// Don't block, just log - unusual activity times are a behavioral signal
		log.Debugln("[BehaviorProtection] activity outside expected active hours (current: %d)", localHour)
	}
}

// IsActiveHour checks if the current hour is within the configured active hours.
func (bpc *BehaviorProtectedConn) IsActiveHour() bool {
	if !bpc.config.UsageTimeSimulation || len(bpc.config.ActiveHours) == 0 {
		return true
	}
	now := time.Now()
	localHour := (now.UTC().Hour() + bpc.config.TimezoneOffset + 24) % 24
	for _, h := range bpc.config.ActiveHours {
		if h == localHour {
			return true
		}
	}
	return false
}

// ============================================================
// Problem 3: Proxy Detection Protection
// ============================================================

// DetectionProtectionConfig controls proxy detection protection.
type DetectionProtectionConfig struct {
	Enabled bool

	// Service fingerprint concealment
	FingerprintConcealment bool
	// Response delay randomization
	ResponseDelayRandomization bool
	// Min/max response delay in ms
	MinResponseDelayMs int
	MaxResponseDelayMs int
	// Error response forgery - return realistic error responses
	ErrorResponseForgery bool
	// Honeypot trap detection
	HoneypotDetection bool
	// Reverse probe protection
	ReverseProbeProtection bool
	// Known probe IP ranges (CIDR)
	ProbeIPRanges []string
}

// DefaultDetectionProtectionConfig returns sensible defaults.
func DefaultDetectionProtectionConfig() *DetectionProtectionConfig {
	return &DetectionProtectionConfig{
		Enabled:                    true,
		FingerprintConcealment:     true,
		ResponseDelayRandomization: true,
		MinResponseDelayMs:         10,
		MaxResponseDelayMs:         100,
		ErrorResponseForgery:       true,
		HoneypotDetection:          true,
		ReverseProbeProtection:     true,
		ProbeIPRanges:              []string{},
	}
}

// DetectionProtectedConn wraps a connection with proxy detection protection.
type DetectionProtectedConn struct {
	net.Conn
	config *DetectionProtectionConfig
	mu     sync.Mutex
	closed bool
}

// NewDetectionProtectedConn creates a connection with detection protection.
func NewDetectionProtectedConn(conn net.Conn, config *DetectionProtectionConfig) *DetectionProtectedConn {
	if config == nil {
		config = DefaultDetectionProtectionConfig()
	}
	return &DetectionProtectedConn{
		Conn:   conn,
		config: config,
	}
}

// Write implements net.Conn with detection protection.
func (dpc *DetectionProtectedConn) Write(b []byte) (int, error) {
	dpc.mu.Lock()
	defer dpc.mu.Unlock()

	if dpc.closed {
		return 0, net.ErrClosed
	}

	// Apply response delay randomization
	if dpc.config.ResponseDelayRandomization {
		minMs := dpc.config.MinResponseDelayMs
		maxMs := dpc.config.MaxResponseDelayMs
		if maxMs > minMs {
			delayMs := minMs + randv2.IntN(maxMs-minMs)
			if delayMs > 0 {
				time.Sleep(time.Duration(delayMs) * time.Millisecond)
			}
		}
	}

	return dpc.Conn.Write(b)
}

// Read implements net.Conn with detection protection.
func (dpc *DetectionProtectedConn) Read(b []byte) (int, error) {
	n, err := dpc.Conn.Read(b)

	// Error response forgery: 仅在连接被探测性关闭时生效。
	// 注意：伪造 HTTP 响应仅适用于 HTTP 代理场景，对 SOCKS/TLS 等协议
	// 可能导致协议错误。因此默认关闭，仅在明确配置时启用。
	if err != nil && dpc.config.ErrorResponseForgery {
		if err == io.EOF || err == io.ErrUnexpectedEOF {
			// 返回空数据而非伪造 HTTP 响应，避免破坏非 HTTP 协议
			return 0, err
		}
	}

	return n, err
}

// Close implements net.Conn.
func (dpc *DetectionProtectedConn) Close() error {
	dpc.mu.Lock()
	defer dpc.mu.Unlock()

	if dpc.closed {
		return nil
	}
	dpc.closed = true
	return dpc.Conn.Close()
}

// IsProbeConnection checks if a remote address looks like a probe/detection system.
func IsProbeConnection(remoteAddr string, knownProbes []string) bool {
	for _, probe := range knownProbes {
		if remoteAddr == probe {
			log.Warnln("[DetectionProtection] probe connection detected from: %s", remoteAddr)
			return true
		}
	}
	return false
}

// ============================================================
// Composite Obfuscation Engine
// ============================================================

// ObfuscationEngineConfig combines all obfuscation configurations.
type ObfuscationEngineConfig struct {
	Traffic   *TrafficObfuscationConfig
	Behavior  *BehaviorProtectionConfig
	Detection *DetectionProtectionConfig
}

// ObfuscationEngine wraps a connection with all obfuscation layers.
type ObfuscationEngine struct {
	config *ObfuscationEngineConfig
}

// NewObfuscationEngine creates a new obfuscation engine.
func NewObfuscationEngine(config *ObfuscationEngineConfig) *ObfuscationEngine {
	if config == nil {
		config = &ObfuscationEngineConfig{
			Traffic:   DefaultTrafficObfuscationConfig(),
			Behavior:  DefaultBehaviorProtectionConfig(),
			Detection: DefaultDetectionProtectionConfig(),
		}
	}
	return &ObfuscationEngine{config: config}
}

// WrapConn applies all enabled obfuscation layers to a connection.
// Layers are applied from innermost to outermost:
// 1. Detection protection (innermost - closest to the raw connection)
// 2. Behavior protection
// 3. Traffic obfuscation (outermost - closest to the application)
func (oe *ObfuscationEngine) WrapConn(conn net.Conn) net.Conn {
	// Layer 1: Detection protection
	if oe.config.Detection != nil && oe.config.Detection.Enabled {
		conn = NewDetectionProtectedConn(conn, oe.config.Detection)
	}

	// Layer 2: Behavior protection
	if oe.config.Behavior != nil && oe.config.Behavior.Enabled {
		conn = NewBehaviorProtectedConn(conn, oe.config.Behavior)
	}

	// Layer 3: Traffic obfuscation
	if oe.config.Traffic != nil && oe.config.Traffic.Enabled {
		conn = NewObfuscatedConn(conn, oe.config.Traffic)
	}

	return conn
}

// Config returns the engine configuration.
func (oe *ObfuscationEngine) Config() *ObfuscationEngineConfig {
	return oe.config
}

// ============================================================
// Global engine instance
// ============================================================

var (
	// DefaultEngine is the default obfuscation engine.
	DefaultEngine = NewObfuscationEngine(nil)
)
