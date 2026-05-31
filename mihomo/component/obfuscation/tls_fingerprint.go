package obfuscation

import (
	"sync"
	"sync/atomic"
	"time"

	"github.com/metacubex/mihomo/log"
	"github.com/metacubex/randv2"
	utls "github.com/metacubex/utls"
)

// ============================================================
// Problem 4: TLS Fingerprint Enhancement
// ============================================================

// TLSFingerprintConfig controls TLS fingerprint obfuscation.
type TLSFingerprintConfig struct {
	Enabled bool

	// Dynamic fingerprint rotation
	DynamicRotation bool
	// Rotation interval in minutes
	RotationIntervalMin int
	// Fingerprint pool - which fingerprints to rotate through
	FingerprintPool []string

	// Real browser matching - prefer fingerprints that match current browser market share
	BrowserMarketShareMatching bool

	// ESNI/ECH support
	ESNISupport bool
}

// DefaultTLSFingerprintConfig returns sensible defaults.
func DefaultTLSFingerprintConfig() *TLSFingerprintConfig {
	return &TLSFingerprintConfig{
		Enabled:                    true,
		DynamicRotation:            true,
		RotationIntervalMin:        30,
		FingerprintPool:            DefaultFingerprintPool,
		BrowserMarketShareMatching: true,
		ESNISupport:                true,
	}
}

// DefaultFingerprintPool is the default pool of TLS fingerprints to rotate through.
var DefaultFingerprintPool = []string{
	"chrome",
	"chrome120",
	"safari",
	"ios",
	"firefox",
	"firefox120",
	"edge",
	"360",
	"qq",
}

// BrowserMarketShare represents approximate browser market share for weighted selection.
var BrowserMarketShare = map[string]int{
	"chrome":  65, // ~65% market share
	"safari":  18, // ~18%
	"firefox": 3,  // ~3%
	"edge":    5,  // ~5%
	"ios":     5,  // iOS Safari
	"android": 2,  // Android
	"360":     1,  // Chinese market
	"qq":      1,  // Chinese market
}

// TLSFingerprintRotator manages dynamic TLS fingerprint rotation.
type TLSFingerprintRotator struct {
	mu                 sync.Mutex
	config             *TLSFingerprintConfig
	currentIndex       int
	currentFingerprint string
	lastRotation       time.Time
	rotationCount      atomic.Int64
}

// NewTLSFingerprintRotator creates a TLS fingerprint rotator.
func NewTLSFingerprintRotator(config *TLSFingerprintConfig) *TLSFingerprintRotator {
	if config == nil {
		config = DefaultTLSFingerprintConfig()
	}

	tfr := &TLSFingerprintRotator{
		config:       config,
		lastRotation: time.Now(),
	}

	// Select initial fingerprint
	tfr.currentFingerprint = tfr.selectWeighted()
	tfr.currentIndex = tfr.findPoolIndex(tfr.currentFingerprint)

	log.Debugln("[TLSFingerprint] initial fingerprint: %s", tfr.currentFingerprint)
	return tfr
}

// CurrentFingerprint returns the current TLS fingerprint name.
func (tfr *TLSFingerprintRotator) CurrentFingerprint() string {
	tfr.mu.Lock()
	// Check if rotation is needed (must hold lock since rotate mutates state)
	if tfr.config.DynamicRotation && time.Since(tfr.lastRotation) > time.Duration(tfr.config.RotationIntervalMin)*time.Minute {
		tfr.rotate()
	}
	fp := tfr.currentFingerprint
	tfr.mu.Unlock()
	return fp
}

// CurrentHelloID returns the current uTLS ClientHelloID.
func (tfr *TLSFingerprintRotator) CurrentHelloID() utls.ClientHelloID {
	name := tfr.CurrentFingerprint()
	return fingerprintNameToHelloID(name)
}

// ForceRotation forces an immediate fingerprint rotation.
func (tfr *TLSFingerprintRotator) ForceRotation() string {
	tfr.mu.Lock()
	defer tfr.mu.Unlock()
	tfr.rotate()
	return tfr.currentFingerprint
}

// rotate selects a new fingerprint different from the current one.
func (tfr *TLSFingerprintRotator) rotate() {
	pool := tfr.config.FingerprintPool
	if len(pool) <= 1 {
		return
	}

	var newFingerprint string
	if tfr.config.BrowserMarketShareMatching {
		newFingerprint = tfr.selectWeighted()
		// Avoid selecting the same fingerprint
		for attempts := 0; attempts < 10 && newFingerprint == tfr.currentFingerprint; attempts++ {
			newFingerprint = tfr.selectWeighted()
		}
	} else {
		// Simple round-robin
		tfr.currentIndex = (tfr.currentIndex + 1) % len(pool)
		newFingerprint = pool[tfr.currentIndex]
	}

	tfr.currentFingerprint = newFingerprint
	tfr.lastRotation = time.Now()
	tfr.rotationCount.Add(1)

	log.Debugln("[TLSFingerprint] rotated to fingerprint: %s (rotation #%d)",
		tfr.currentFingerprint, tfr.rotationCount.Load())
}

// selectWeighted selects a fingerprint based on browser market share weights.
func (tfr *TLSFingerprintRotator) selectWeighted() string {
	pool := tfr.config.FingerprintPool
	if len(pool) == 0 {
		return "chrome"
	}

	// Calculate total weight
	totalWeight := 0
	weights := make([]int, len(pool))
	for i, name := range pool {
		w := BrowserMarketShare[name]
		if w == 0 {
			w = 1 // default weight for unknown fingerprints
		}
		weights[i] = w
		totalWeight += w
	}

	// Weighted random selection
	r := randv2.IntN(totalWeight)
	cumWeight := 0
	for i, w := range weights {
		cumWeight += w
		if r < cumWeight {
			return pool[i]
		}
	}

	return pool[0]
}

// findPoolIndex finds the index of a fingerprint in the pool.
func (tfr *TLSFingerprintRotator) findPoolIndex(name string) int {
	for i, n := range tfr.config.FingerprintPool {
		if n == name {
			return i
		}
	}
	return 0
}

// RotationCount returns the number of rotations performed.
func (tfr *TLSFingerprintRotator) RotationCount() int64 {
	return tfr.rotationCount.Load()
}

// fingerprintNameToHelloID maps a fingerprint name to a uTLS ClientHelloID.
func fingerprintNameToHelloID(name string) utls.ClientHelloID {
	switch name {
	case "chrome":
		return utls.HelloChrome_Auto
	case "chrome120":
		return utls.HelloChrome_120
	case "firefox":
		return utls.HelloFirefox_Auto
	case "firefox120":
		return utls.HelloFirefox_120
	case "safari":
		return utls.HelloSafari_Auto
	case "safari16":
		return utls.HelloSafari_16_0
	case "ios":
		return utls.HelloIOS_Auto
	case "android":
		return utls.HelloAndroid_11_OkHttp
	case "edge":
		return utls.HelloEdge_Auto
	case "360":
		return utls.Hello360_Auto
	case "qq":
		return utls.HelloQQ_Auto
	case "randomized":
		return utls.HelloRandomized
	default:
		return utls.HelloChrome_Auto
	}
}

// ============================================================
// TLS Fingerprint Statistics
// ============================================================

// TLSFingerprintStats tracks TLS fingerprint usage statistics.
type TLSFingerprintStats struct {
	mu          sync.RWMutex
	usageCounts map[string]int64
	totalUsage  atomic.Int64
}

var DefaultTLSFingerprintStats = &TLSFingerprintStats{
	usageCounts: make(map[string]int64),
}

// Record records a TLS fingerprint usage.
func (s *TLSFingerprintStats) Record(fingerprint string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.usageCounts[fingerprint]++
	s.totalUsage.Add(1)
}

// Snapshot returns a snapshot of fingerprint usage statistics.
func (s *TLSFingerprintStats) Snapshot() map[string]int64 {
	s.mu.RLock()
	defer s.mu.RUnlock()

	result := make(map[string]int64, len(s.usageCounts))
	for k, v := range s.usageCounts {
		result[k] = v
	}
	return result
}

// ============================================================
// Global TLS fingerprint rotator
// ============================================================

var (
	// DefaultTLSRotator is the default TLS fingerprint rotator.
	DefaultTLSRotator = NewTLSFingerprintRotator(DefaultTLSFingerprintConfig())
)
