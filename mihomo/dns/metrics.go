package dns

import (
	"sync"
	"sync/atomic"
	"time"
)

// DnsMetrics tracks DNS query performance and cache statistics.
type DnsMetrics struct {
	// Cache stats
	cacheHit  atomic.Uint64
	cacheMiss atomic.Uint64
	cacheSize atomic.Int32

	// Query stats per resolver
	queryTotal   atomic.Uint64
	querySuccess atomic.Uint64
	queryFailed  atomic.Uint64

	// Latency tracking (in microseconds)
	latencySum   atomic.Uint64
	latencyCount atomic.Uint64
	latencyMax   atomic.Uint64

	// Per-nameserver stats
	serverMu    sync.RWMutex
	serverStats map[string]*serverMetric
}

type serverMetric struct {
	queries    uint64
	successes  uint64
	failures   uint64
	latencySum uint64 // microseconds
	lastQuery  time.Time
	lastError  string
}

var defaultMetrics = &DnsMetrics{
	serverStats: make(map[string]*serverMetric),
}

// RecordCacheHit increments cache hit counter.
func (m *DnsMetrics) RecordCacheHit() {
	m.cacheHit.Add(1)
}

// RecordCacheMiss increments cache miss counter.
func (m *DnsMetrics) RecordCacheMiss() {
	m.cacheMiss.Add(1)
}

// SetCacheSize updates the current cache size.
func (m *DnsMetrics) SetCacheSize(size int) {
	m.cacheSize.Store(int32(size))
}

// RecordQuery records a DNS query result with latency.
func (m *DnsMetrics) RecordQuery(server string, latency time.Duration, success bool, errMsg string) {
	m.queryTotal.Add(1)
	latencyUs := uint64(latency.Microseconds())

	if success {
		m.querySuccess.Add(1)
	} else {
		m.queryFailed.Add(1)
	}

	m.latencySum.Add(latencyUs)
	m.latencyCount.Add(1)

	// Update max latency (CAS loop)
	for {
		current := m.latencyMax.Load()
		if latencyUs <= current {
			break
		}
		if m.latencyMax.CompareAndSwap(current, latencyUs) {
			break
		}
	}

	// Per-server stats
	m.serverMu.Lock()
	sm, ok := m.serverStats[server]
	if !ok {
		sm = &serverMetric{}
		m.serverStats[server] = sm
	}
	sm.queries++
	sm.lastQuery = time.Now()
	if success {
		sm.successes++
		sm.latencySum += latencyUs
	} else {
		sm.failures++
		sm.lastError = errMsg
	}
	m.serverMu.Unlock()
}

// CacheStats returns current cache statistics.
type CacheStats struct {
	Hit     uint64  `json:"hit"`
	Miss    uint64  `json:"miss"`
	Size    int32   `json:"size"`
	HitRate float64 `json:"hitRate"`
}

func (m *DnsMetrics) GetCacheStats() CacheStats {
	hit := m.cacheHit.Load()
	miss := m.cacheMiss.Load()
	total := hit + miss
	var hitRate float64
	if total > 0 {
		hitRate = float64(hit) / float64(total)
	}
	return CacheStats{
		Hit:     hit,
		Miss:    miss,
		Size:    m.cacheSize.Load(),
		HitRate: hitRate,
	}
}

// QueryStats returns aggregate query statistics.
type QueryStats struct {
	Total        uint64 `json:"total"`
	Success      uint64 `json:"success"`
	Failed       uint64 `json:"failed"`
	AvgLatencyUs uint64 `json:"avgLatencyUs"`
	MaxLatencyUs uint64 `json:"maxLatencyUs"`
}

func (m *DnsMetrics) GetQueryStats() QueryStats {
	count := m.latencyCount.Load()
	var avg uint64
	if count > 0 {
		avg = m.latencySum.Load() / count
	}
	return QueryStats{
		Total:        m.queryTotal.Load(),
		Success:      m.querySuccess.Load(),
		Failed:       m.queryFailed.Load(),
		AvgLatencyUs: avg,
		MaxLatencyUs: m.latencyMax.Load(),
	}
}

// ServerStats returns per-nameserver statistics.
type ServerStats struct {
	Server     string `json:"server"`
	Queries    uint64 `json:"queries"`
	Successes  uint64 `json:"successes"`
	Failures   uint64 `json:"failures"`
	AvgLatency uint64 `json:"avgLatencyUs"`
	LastQuery  string `json:"lastQuery"`
	LastError  string `json:"lastError,omitempty"`
}

func (m *DnsMetrics) GetServerStats() []ServerStats {
	m.serverMu.RLock()
	defer m.serverMu.RUnlock()

	result := make([]ServerStats, 0, len(m.serverStats))
	for server, sm := range m.serverStats {
		var avg uint64
		if sm.successes > 0 {
			avg = sm.latencySum / sm.successes
		}
		lastQuery := ""
		if !sm.lastQuery.IsZero() {
			lastQuery = sm.lastQuery.Format(time.RFC3339)
		}
		result = append(result, ServerStats{
			Server:     server,
			Queries:    sm.queries,
			Successes:  sm.successes,
			Failures:   sm.failures,
			AvgLatency: avg,
			LastQuery:  lastQuery,
			LastError:  sm.lastError,
		})
	}
	return result
}

// AllStats returns the complete DNS metrics snapshot.
type AllStats struct {
	Cache     CacheStats     `json:"cache"`
	Queries   QueryStats     `json:"queries"`
	Servers   []ServerStats  `json:"servers"`
	Pollution PollutionStats `json:"pollution"`
	Trust     TrustSummary   `json:"trust"`
}

func (m *DnsMetrics) GetAllStats() AllStats {
	return AllStats{
		Cache:     m.GetCacheStats(),
		Queries:   m.GetQueryStats(),
		Servers:   m.GetServerStats(),
		Pollution: GetPollutionDetector().GetStats(),
		Trust:     GetTrustEvaluator().GetTrustSummary(),
	}
}

// GetDnsMetrics returns the global DNS metrics instance.
func GetDnsMetrics() *DnsMetrics {
	return defaultMetrics
}
