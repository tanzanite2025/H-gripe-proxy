package dns

import (
	"sync"
	"sync/atomic"
	"time"
)

const recentQueryEventLimit = 128

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

	recentMu      sync.RWMutex
	recentQueries []QueryEvent
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

// NewDnsMetricsForTest returns an isolated metrics collector for unit tests.
func NewDnsMetricsForTest() *DnsMetrics {
	return &DnsMetrics{
		serverStats: make(map[string]*serverMetric),
	}
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

// QueryEvent describes a resolved DNS request as observed by the core.
type QueryEvent struct {
	Domain      string    `json:"domain"`
	QType       string    `json:"qType"`
	Server      string    `json:"server"`
	Protocol    string    `json:"protocol"`
	ProxyName   string    `json:"proxyName,omitempty"`
	ProxyChain  string    `json:"proxyChain,omitempty"`
	Egress      string    `json:"egress,omitempty"`
	Rule        string    `json:"rule,omitempty"`
	RulePayload string    `json:"rulePayload,omitempty"`
	Success     bool      `json:"success"`
	Error       string    `json:"error,omitempty"`
	LatencyUs   uint64    `json:"latencyUs"`
	Timestamp   time.Time `json:"-"`
}

// QueryEventSnapshot is the JSON-facing representation of QueryEvent.
type QueryEventSnapshot struct {
	Domain      string `json:"domain"`
	QType       string `json:"qType"`
	Server      string `json:"server"`
	Protocol    string `json:"protocol"`
	ProxyName   string `json:"proxyName,omitempty"`
	ProxyChain  string `json:"proxyChain,omitempty"`
	Egress      string `json:"egress,omitempty"`
	Rule        string `json:"rule,omitempty"`
	RulePayload string `json:"rulePayload,omitempty"`
	Success     bool   `json:"success"`
	Error       string `json:"error,omitempty"`
	LatencyUs   uint64 `json:"latencyUs"`
	Timestamp   string `json:"timestamp"`
}

// RecordQuery records a DNS query result with latency.
func (m *DnsMetrics) RecordQuery(server string, latency time.Duration, success bool, errMsg string) {
	m.RecordQueryEvent(QueryEvent{
		Server:    server,
		Success:   success,
		Error:     errMsg,
		LatencyUs: uint64(latency.Microseconds()),
		Timestamp: time.Now(),
	})
}

// RecordQueryEvent records a complete DNS query event.
func (m *DnsMetrics) RecordQueryEvent(event QueryEvent) {
	m.queryTotal.Add(1)
	if event.Timestamp.IsZero() {
		event.Timestamp = time.Now()
	}

	if event.Success {
		m.querySuccess.Add(1)
	} else {
		m.queryFailed.Add(1)
	}

	m.latencySum.Add(event.LatencyUs)
	m.latencyCount.Add(1)

	// Update max latency (CAS loop)
	for {
		current := m.latencyMax.Load()
		if event.LatencyUs <= current {
			break
		}
		if m.latencyMax.CompareAndSwap(current, event.LatencyUs) {
			break
		}
	}

	// Per-server stats
	m.serverMu.Lock()
	sm, ok := m.serverStats[event.Server]
	if !ok {
		sm = &serverMetric{}
		m.serverStats[event.Server] = sm
	}
	sm.queries++
	sm.lastQuery = event.Timestamp
	if event.Success {
		sm.successes++
		sm.latencySum += event.LatencyUs
	} else {
		sm.failures++
		sm.lastError = event.Error
	}
	m.serverMu.Unlock()

	m.recordRecentQuery(event)
}

func (m *DnsMetrics) recordRecentQuery(event QueryEvent) {
	m.recentMu.Lock()
	defer m.recentMu.Unlock()

	if len(m.recentQueries) >= recentQueryEventLimit {
		copy(m.recentQueries, m.recentQueries[1:])
		m.recentQueries[len(m.recentQueries)-1] = event
		return
	}
	m.recentQueries = append(m.recentQueries, event)
}

func (m *DnsMetrics) GetRecentQueries() []QueryEventSnapshot {
	m.recentMu.RLock()
	defer m.recentMu.RUnlock()

	result := make([]QueryEventSnapshot, 0, len(m.recentQueries))
	for _, event := range m.recentQueries {
		timestamp := ""
		if !event.Timestamp.IsZero() {
			timestamp = event.Timestamp.Format(time.RFC3339)
		}
		result = append(result, QueryEventSnapshot{
			Domain:      event.Domain,
			QType:       event.QType,
			Server:      event.Server,
			Protocol:    event.Protocol,
			ProxyName:   event.ProxyName,
			ProxyChain:  event.ProxyChain,
			Egress:      event.Egress,
			Rule:        event.Rule,
			RulePayload: event.RulePayload,
			Success:     event.Success,
			Error:       event.Error,
			LatencyUs:   event.LatencyUs,
			Timestamp:   timestamp,
		})
	}
	return result
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
	Cache     CacheStats           `json:"cache"`
	Queries   QueryStats           `json:"queries"`
	Servers   []ServerStats        `json:"servers"`
	Recent    []QueryEventSnapshot `json:"recent"`
	Pollution PollutionStats       `json:"pollution"`
	Trust     TrustSummary         `json:"trust"`
}

func (m *DnsMetrics) GetAllStats() AllStats {
	return AllStats{
		Cache:     m.GetCacheStats(),
		Queries:   m.GetQueryStats(),
		Servers:   m.GetServerStats(),
		Recent:    m.GetRecentQueries(),
		Pollution: GetPollutionDetector().GetStats(),
		Trust:     GetTrustEvaluator().GetTrustSummary(),
	}
}

// GetDnsMetrics returns the global DNS metrics instance.
func GetDnsMetrics() *DnsMetrics {
	return defaultMetrics
}
