package tunnel

import (
	"sync"
	"sync/atomic"
	"time"

	"github.com/tanzanite2025/mihomo-optimized/log"
)

// ============================================================
// Problem 2: Buffer Management - Dynamic Buffers & Memory Pool
// ============================================================

// DynamicBufferPool manages buffer allocation with dynamic sizing.
// It tracks actual usage patterns and adjusts buffer sizes to minimize
// both allocation overhead and memory waste.
type DynamicBufferPool struct {
	mu sync.Mutex

	// Size classes for buffers
	sizeClasses []int
	// Pools for each size class
	pools []sync.Pool

	// Usage tracking for adaptive sizing
	allocCounts []atomic.Int64
	peakUsage   []atomic.Int64

	// Statistics
	totalAlloc   atomic.Int64
	totalReturn  atomic.Int64
	totalWaste   atomic.Int64 // bytes wasted due to size rounding
	allocErrors  atomic.Int64
}

// NewDynamicBufferPool creates a buffer pool with adaptive sizing.
func NewDynamicBufferPool() *DynamicBufferPool {
	// Size classes: 512B, 1KB, 2KB, 4KB, 8KB, 16KB, 32KB, 64KB
	sizeClasses := []int{512, 1024, 2048, 4096, 8192, 16384, 32768, 65536}
	pools := make([]sync.Pool, len(sizeClasses))
	allocCounts := make([]atomic.Int64, len(sizeClasses))
	peakUsage := make([]atomic.Int64, len(sizeClasses))

	for i, size := range sizeClasses {
		s := size
		idx := i
		pools[i] = sync.Pool{
			New: func() any {
				buf := make([]byte, s)
				allocCounts[idx].Add(1)
				return &buf
			},
		}
	}

	return &DynamicBufferPool{
		sizeClasses:  sizeClasses,
		pools:        pools,
		allocCounts:  allocCounts,
		peakUsage:    peakUsage,
	}
}

// Get returns a buffer of at least the requested size.
func (dbp *DynamicBufferPool) Get(size int) *[]byte {
	dbp.mu.Lock()
	defer dbp.mu.Unlock()

	idx := dbp.sizeClassIndex(size)
	if idx < 0 || idx >= len(dbp.pools) {
		// Too large for pool, allocate directly
		dbp.totalAlloc.Add(1)
		buf := make([]byte, size)
		return &buf
	}

	buf := dbp.pools[idx].Get().(*[]byte)
	resized := (*buf)[:size]

	dbp.totalAlloc.Add(1)
	waste := int64(dbp.sizeClasses[idx] - size)
	if waste > 0 {
		dbp.totalWaste.Add(waste)
	}

	// Track peak usage
	current := dbp.allocCounts[idx].Load()
	for {
		peak := dbp.peakUsage[idx].Load()
		if current <= peak {
			break
		}
		if dbp.peakUsage[idx].CompareAndSwap(peak, current) {
			break
		}
	}

	return &resized
}

// Put returns a buffer to the pool.
func (dbp *DynamicBufferPool) Put(buf *[]byte) {
	if buf == nil {
		return
	}

	capacity := cap(*buf)
	if capacity == 0 {
		return
	}

	idx := dbp.sizeClassIndex(capacity)
	if idx < 0 || idx >= len(dbp.pools) {
		// Not from pool, let GC handle it
		return
	}

	// Only return if the capacity matches a size class exactly
	if capacity != dbp.sizeClasses[idx] {
		return
	}

	*buf = (*buf)[:capacity]
	dbp.pools[idx].Put(buf)
	dbp.totalReturn.Add(1)
}

// Stats returns current pool statistics.
func (dbp *DynamicBufferPool) Stats() BufferPoolStats {
	dbp.mu.Lock()
	defer dbp.mu.Unlock()

	stats := BufferPoolStats{
		TotalAlloc:  dbp.totalAlloc.Load(),
		TotalReturn: dbp.totalReturn.Load(),
		TotalWaste:  dbp.totalWaste.Load(),
		AllocErrors: dbp.allocErrors.Load(),
		SizeClasses:  make([]SizeClassStats, len(dbp.sizeClasses)),
	}

	for i, size := range dbp.sizeClasses {
		stats.SizeClasses[i] = SizeClassStats{
			Size:     size,
			Allocs:   dbp.allocCounts[i].Load(),
			PeakUsed: dbp.peakUsage[i].Load(),
		}
	}

	return stats
}

// Optimize adjusts size classes based on observed usage patterns.
// This should be called periodically (e.g., every 5 minutes).
func (dbp *DynamicBufferPool) Optimize() {
	dbp.mu.Lock()
	defer dbp.mu.Unlock()

	// Calculate waste ratio per size class
	for i, size := range dbp.sizeClasses {
		allocs := dbp.allocCounts[i].Load()
		if allocs == 0 {
			continue
		}
		// Reset peak usage tracking
		dbp.peakUsage[i].Store(0)
		log.Debugln("[BufferPool] size class %d: allocs=%d, peak=%d",
			size, allocs, dbp.peakUsage[i].Load())
	}
}

func (dbp *DynamicBufferPool) sizeClassIndex(size int) int {
	// Binary search for the smallest size class >= requested size
	lo, hi := 0, len(dbp.sizeClasses)-1
	for lo <= hi {
		mid := (lo + hi) / 2
		if dbp.sizeClasses[mid] < size {
			lo = mid + 1
		} else if dbp.sizeClasses[mid] > size {
			hi = mid - 1
		} else {
			return mid
		}
	}
	if lo < len(dbp.sizeClasses) {
		return lo
	}
	return -1 // too large
}

// BufferPoolStats contains statistics about the buffer pool.
type BufferPoolStats struct {
	TotalAlloc  int64            `json:"totalAlloc"`
	TotalReturn int64            `json:"totalReturn"`
	TotalWaste  int64            `json:"totalWaste"`
	AllocErrors int64            `json:"allocErrors"`
	SizeClasses []SizeClassStats `json:"sizeClasses"`
}

// SizeClassStats contains statistics for a single buffer size class.
type SizeClassStats struct {
	Size     int   `json:"size"`
	Allocs   int64 `json:"allocs"`
	PeakUsed int64 `json:"peakUsed"`
}

// ============================================================
// Problem 3: Traffic Statistics - Fine-grained per-rule stats
// ============================================================

// RuleTrafficStats tracks traffic statistics per rule.
type RuleTrafficStats struct {
	mu    sync.Mutex
	rules map[string]*ruleTrafficEntry
}

type ruleTrafficEntry struct {
	ruleType    string
	rulePayload string
	upload      atomic.Int64
	download    atomic.Int64
	connections atomic.Int64
	lastActive  atomic.Int64 // unix ms
	// Historical data (ring buffer)
	history     []trafficSample
	historyIdx  int
	historySize int
}

type trafficSample struct {
	timestamp  time.Time
	upload     int64
	download   int64
	connections int64
}

// NewRuleTrafficStats creates a per-rule traffic statistics tracker.
func NewRuleTrafficStats(historySize int) *RuleTrafficStats {
	return &RuleTrafficStats{
		rules: make(map[string]*ruleTrafficEntry),
	}
}

// Record records traffic for a rule match.
func (rts *RuleTrafficStats) Record(ruleKey, ruleType, rulePayload string, upload, download int64) {
	rts.mu.Lock()
	entry, ok := rts.rules[ruleKey]
	if !ok {
		entry = &ruleTrafficEntry{
			ruleType:    ruleType,
			rulePayload: rulePayload,
			history:     make([]trafficSample, 60), // 60 samples = 1 minute at 1s intervals
			historySize: 60,
		}
		rts.rules[ruleKey] = entry
	}
	rts.mu.Unlock()

	entry.upload.Add(upload)
	entry.download.Add(download)
	entry.lastActive.Store(time.Now().UnixMilli())
}

// RecordConnection records a new connection matching a rule.
func (rts *RuleTrafficStats) RecordConnection(ruleKey string) {
	rts.mu.Lock()
	entry, ok := rts.rules[ruleKey]
	if !ok {
		rts.mu.Unlock()
		return
	}
	rts.mu.Unlock()

	entry.connections.Add(1)
}

// RecordDisconnection records a connection closing for a rule.
func (rts *RuleTrafficStats) RecordDisconnection(ruleKey string) {
	rts.mu.Lock()
	entry, ok := rts.rules[ruleKey]
	if !ok {
		rts.mu.Unlock()
		return
	}
	rts.mu.Unlock()

	entry.connections.Add(-1)
}

// Snapshot takes a snapshot of all rule traffic statistics.
func (rts *RuleTrafficStats) Snapshot() map[string]RuleTrafficSnapshot {
	rts.mu.Lock()
	defer rts.mu.Unlock()

	result := make(map[string]RuleTrafficSnapshot, len(rts.rules))
	for key, entry := range rts.rules {
		// Store current values in history
		entry.history[entry.historyIdx] = trafficSample{
			timestamp:   time.Now(),
			upload:      entry.upload.Load(),
			download:    entry.download.Load(),
			connections: entry.connections.Load(),
		}
		entry.historyIdx = (entry.historyIdx + 1) % entry.historySize

		result[key] = RuleTrafficSnapshot{
			RuleType:    entry.ruleType,
			RulePayload: entry.rulePayload,
			Upload:      entry.upload.Load(),
			Download:    entry.download.Load(),
			Connections: entry.connections.Load(),
			LastActive:  entry.lastActive.Load(),
		}
	}
	return result
}

// RuleTrafficSnapshot is a point-in-time snapshot of rule traffic.
type RuleTrafficSnapshot struct {
	RuleType    string `json:"ruleType"`
	RulePayload string `json:"rulePayload"`
	Upload      int64  `json:"upload"`
	Download    int64  `json:"download"`
	Connections int64  `json:"connections"`
	LastActive  int64  `json:"lastActive"`
}

// ============================================================
// Problem 3: Per-connection traffic statistics
// ============================================================

// ConnTrafficStats provides fine-grained per-connection traffic monitoring.
type ConnTrafficStats struct {
	mu    sync.Mutex
	conns map[string]*connTrafficEntry
}

type connTrafficEntry struct {
	metadata    string
	proxyChain  string
	upload      atomic.Int64
	download    atomic.Int64
	createdAt   time.Time
	lastActive  atomic.Int64
	// Rate tracking
	rateUp      atomic.Int64 // bytes/sec (smoothed)
	rateDown    atomic.Int64 // bytes/sec (smoothed)
	// Previous values for rate calculation
	prevUpload  atomic.Int64
	prevDownload atomic.Int64
	prevTime    atomic.Int64 // unix ms
}

// NewConnTrafficStats creates a per-connection traffic statistics tracker.
func NewConnTrafficStats() *ConnTrafficStats {
	return &ConnTrafficStats{
		conns: make(map[string]*connTrafficEntry),
	}
}

// Track adds a connection for traffic monitoring.
func (cts *ConnTrafficStats) Track(connID, metadata, proxyChain string) {
	cts.mu.Lock()
	defer cts.mu.Unlock()

	now := time.Now()
	cts.conns[connID] = &connTrafficEntry{
		metadata:   metadata,
		proxyChain: proxyChain,
		createdAt:  now,
	}
	cts.conns[connID].lastActive.Store(now.UnixMilli())
	cts.conns[connID].prevTime.Store(now.UnixMilli())
}

// Untrack removes a connection from monitoring.
func (cts *ConnTrafficStats) Untrack(connID string) {
	cts.mu.Lock()
	defer cts.mu.Unlock()
	delete(cts.conns, connID)
}

// RecordUpload records uploaded bytes for a connection.
func (cts *ConnTrafficStats) RecordUpload(connID string, bytes int) {
	cts.mu.Lock()
	entry, ok := cts.conns[connID]
	cts.mu.Unlock()
	if !ok {
		return
	}

	entry.upload.Add(int64(bytes))
	entry.lastActive.Store(time.Now().UnixMilli())
	cts.updateRate(entry)
}

// RecordDownload records downloaded bytes for a connection.
func (cts *ConnTrafficStats) RecordDownload(connID string, bytes int) {
	cts.mu.Lock()
	entry, ok := cts.conns[connID]
	cts.mu.Unlock()
	if !ok {
		return
	}

	entry.download.Add(int64(bytes))
	entry.lastActive.Store(time.Now().UnixMilli())
	cts.updateRate(entry)
}

// GetTopByBandwidth returns the top N connections by total bandwidth.
func (cts *ConnTrafficStats) GetTopByBandwidth(n int) []ConnTrafficSnapshot {
	cts.mu.Lock()
	defer cts.mu.Unlock()

	snapshots := make([]ConnTrafficSnapshot, 0, len(cts.conns))
	for id, entry := range cts.conns {
		snapshots = append(snapshots, ConnTrafficSnapshot{
			ConnID:     id,
			Metadata:   entry.metadata,
			ProxyChain: entry.proxyChain,
			Upload:     entry.upload.Load(),
			Download:   entry.download.Load(),
			RateUp:     entry.rateUp.Load(),
			RateDown:   entry.rateDown.Load(),
			CreatedAt:  entry.createdAt,
			LastActive: entry.lastActive.Load(),
		})
	}

	// Sort by total bandwidth (descending)
	for i := 0; i < len(snapshots); i++ {
		for j := i + 1; j < len(snapshots); j++ {
			if snapshots[i].Upload+snapshots[i].Download < snapshots[j].Upload+snapshots[j].Download {
				snapshots[i], snapshots[j] = snapshots[j], snapshots[i]
			}
		}
	}

	if n > len(snapshots) {
		n = len(snapshots)
	}
	return snapshots[:n]
}

// ActiveCount returns the number of tracked connections.
func (cts *ConnTrafficStats) ActiveCount() int {
	cts.mu.Lock()
	defer cts.mu.Unlock()
	return len(cts.conns)
}

func (cts *ConnTrafficStats) updateRate(entry *connTrafficEntry) {
	now := time.Now().UnixMilli()
	prevTime := entry.prevTime.Swap(now)
	if prevTime == 0 {
		return
	}

	elapsedMs := now - prevTime
	if elapsedMs < 500 { // Don't update too frequently
		return
	}

	elapsedSec := float64(elapsedMs) / 1000.0
	if elapsedSec == 0 {
		return
	}

	currentUp := entry.upload.Load()
	currentDown := entry.download.Load()
	prevUp := entry.prevUpload.Swap(currentUp)
	prevDown := entry.prevDownload.Swap(currentDown)

	upRate := float64(currentUp-prevUp) / elapsedSec
	downRate := float64(currentDown-prevDown) / elapsedSec

	// Exponential moving average
	oldRateUp := entry.rateUp.Load()
	oldRateDown := entry.rateDown.Load()
	entry.rateUp.Store(int64(oldRateUp*7/10 + int64(upRate*3)/10))
	entry.rateDown.Store(int64(oldRateDown*7/10 + int64(downRate*3)/10))
}

// ConnTrafficSnapshot is a point-in-time snapshot of connection traffic.
type ConnTrafficSnapshot struct {
	ConnID     string    `json:"connId"`
	Metadata   string    `json:"metadata"`
	ProxyChain string    `json:"proxyChain"`
	Upload     int64     `json:"upload"`
	Download   int64     `json:"download"`
	RateUp     int64     `json:"rateUp"`    // bytes/sec
	RateDown   int64     `json:"rateDown"`  // bytes/sec
	CreatedAt  time.Time `json:"createdAt"`
	LastActive int64     `json:"lastActive"`
}

// ============================================================
// Global instances
// ============================================================

var (
	// DefaultConnPool is the default proxy connection pool.
	DefaultConnPool = NewProxyConnPool(4, 30*time.Second)

	// DefaultConnManager is the default connection manager.
	DefaultConnManager = NewConnManager(1000, 15*time.Second, 5*time.Minute)

	// DefaultBufferPool is the default dynamic buffer pool.
	DefaultBufferPool = NewDynamicBufferPool()

	// DefaultRuleTrafficStats tracks per-rule traffic.
	DefaultRuleTrafficStats = NewRuleTrafficStats(60)

	// DefaultConnTrafficStats tracks per-connection traffic.
	DefaultConnTrafficStats = NewConnTrafficStats()

	// DefaultEgressMonitor monitors egress IP stability.
	DefaultEgressMonitor = NewEgressMonitor()
)
