package wireguard

import (
	"context"
	"net"
	"sync"
	"sync/atomic"
	"time"

	"github.com/metacubex/mihomo/log"
)

const (
	// MTU detection parameters
	mtuProbeMin     = 576   // IPv4 minimum
	mtuProbeMax     = 9000  // jumbo frame max
	mtuProbeDefault = 1408  // WireGuard default
	mtuProbeStep    = 128   // binary search step
	mtuProbeTimeout = 3 * time.Second

	// Connection health monitoring
	healthCheckInterval = 30 * time.Second
	healthCheckTimeout  = 5 * time.Second
	maxConsecutiveFails = 3
)

// MTUDetector performs path MTU discovery for WireGuard connections.
type MTUDetector struct {
	mtu          atomic.Int32
	lastProbe    atomic.Int64 // unix timestamp
	probing      atomic.Bool
	remoteAddr   string
}

// NewMTUDetector creates an MTU detector with a default value.
func NewMTUDetector(defaultMTU int) *MTUDetector {
	if defaultMTU <= 0 {
		defaultMTU = mtuProbeDefault
	}
	m := &MTUDetector{}
	m.mtu.Store(int32(defaultMTU))
	return m
}

// GetMTU returns the current best-effort MTU value.
func (m *MTUDetector) GetMTU() int {
	return int(m.mtu.Load())
}

// ProbeMTU attempts to discover the path MTU using binary search.
// It sends increasingly large packets until one fails, then backs off.
func (m *MTUDetector) ProbeMTU(ctx context.Context, dialer func(ctx context.Context, network, addr string) (net.Conn, error)) {
	if m.probing.Swap(true) {
		return // already probing
	}
	defer m.probing.Store(false)

	m.lastProbe.Store(time.Now().Unix())

	low := mtuProbeMin
	high := mtuProbeMax
	bestMtu := low

	for low <= high {
		select {
		case <-ctx.Done():
			return
		default:
		}

		mid := (low + high) / 2
		// Round down to multiple of 16 for WireGuard alignment
		mid = mid &^ 15

		if m.tryMTU(ctx, dialer, mid) {
			bestMtu = mid
			low = mid + mtuProbeStep
		} else {
			high = mid - mtuProbeStep
		}
	}

	// Subtract WireGuard overhead (60 bytes for IPv4 + 32 bytes WG header + 8 bytes UDP)
	wireguardOverhead := 60 + 32 + 8
	effectiveMtu := bestMtu - wireguardOverhead
	if effectiveMtu < 576 {
		effectiveMtu = 576
	}

	oldMtu := m.mtu.Load()
	m.mtu.Store(int32(effectiveMtu))

	if int32(effectiveMtu) != oldMtu {
		log.Infoln("[WG-MTU] path MTU discovered: %d (payload: %d)", bestMtu, effectiveMtu)
	}
}

// tryMTU attempts to establish a connection with the given MTU.
// Returns true if the MTU appears to work.
func (m *MTUDetector) tryMTU(ctx context.Context, dialer func(ctx context.Context, network, addr string) (net.Conn, error), mtu int) bool {
	probeCtx, cancel := context.WithTimeout(ctx, mtuProbeTimeout)
	defer cancel()

	conn, err := dialer(probeCtx, "udp", m.remoteAddr)
	if err != nil {
		return false
	}
	defer conn.Close()

	// Send a packet of the target size
	// If the path MTU is smaller, this will fail with EMSGSIZE
	packet := make([]byte, mtu)
	_, err = conn.Write(packet)
	return err == nil
}

// ShouldProbe returns whether it's time for a new MTU probe.
func (m *MTUDetector) ShouldProbe() bool {
	lastProbe := m.lastProbe.Load()
	return time.Now().Unix()-lastProbe > 300 // re-probe every 5 minutes
}

// ConnectionHealthMonitor monitors WireGuard connection health
// and triggers reconnection when the connection degrades.
type ConnectionHealthMonitor struct {
	mu              sync.Mutex
	consecutiveFails atomic.Int32
	lastSuccess     atomic.Int64 // unix timestamp
	monitoring      atomic.Bool
	cancel          context.CancelFunc
	onUnhealthy     func()
}

// NewConnectionHealthMonitor creates a health monitor.
func NewConnectionHealthMonitor(onUnhealthy func()) *ConnectionHealthMonitor {
	return &ConnectionHealthMonitor{
		onUnhealthy: onUnhealthy,
	}
}

// Start begins periodic health monitoring.
func (h *ConnectionHealthMonitor) Start() {
	if h.monitoring.Swap(true) {
		return // already running
	}

	ctx, cancel := context.WithCancel(context.Background())
	h.cancel = cancel

	go h.monitorLoop(ctx)
}

// Stop halts health monitoring.
func (h *ConnectionHealthMonitor) Stop() {
	if !h.monitoring.Swap(false) {
		return
	}
	if h.cancel != nil {
		h.cancel()
	}
}

// RecordSuccess records a successful connection event.
func (h *ConnectionHealthMonitor) RecordSuccess() {
	h.consecutiveFails.Store(0)
	h.lastSuccess.Store(time.Now().Unix())
}

// RecordFailure records a failed connection event.
func (h *ConnectionHealthMonitor) RecordFailure() {
	fails := h.consecutiveFails.Add(1)
	if fails >= maxConsecutiveFails {
		log.Warnln("[WG-Health] %d consecutive failures, triggering recovery", fails)
		h.consecutiveFails.Store(0)
		if h.onUnhealthy != nil {
			go h.onUnhealthy()
		}
	}
}

func (h *ConnectionHealthMonitor) monitorLoop(ctx context.Context) {
	ticker := time.NewTicker(healthCheckInterval)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			lastSuccess := h.lastSuccess.Load()
			elapsed := time.Now().Unix() - lastSuccess
			// If no successful activity in 2x health check interval, consider unhealthy
			if elapsed > int64(2*healthCheckInterval/time.Second) {
				log.Warnln("[WG-Health] no activity for %ds, triggering recovery", elapsed)
				if h.onUnhealthy != nil {
					go h.onUnhealthy()
				}
			}
		}
	}
}

// ConnPool provides connection reuse for WireGuard UDP sessions.
type ConnPool struct {
	mu    sync.Mutex
	conns map[string]*poolEntry
	ttl   time.Duration
}

type poolEntry struct {
	conn      net.Conn
	lastUsed  time.Time
}

// NewConnPool creates a connection pool with the given TTL.
func NewConnPool(ttl time.Duration) *ConnPool {
	return &ConnPool{
		conns: make(map[string]*poolEntry),
		ttl:   ttl,
	}
}

// Get retrieves a cached connection or returns nil.
func (p *ConnPool) Get(key string) net.Conn {
	p.mu.Lock()
	defer p.mu.Unlock()

	entry, ok := p.conns[key]
	if !ok {
		return nil
	}

	// Check if connection is still valid and not expired
	if time.Since(entry.lastUsed) > p.ttl {
		entry.conn.Close()
		delete(p.conns, key)
		return nil
	}

	entry.lastUsed = time.Now()
	return entry.conn
}

// Put stores a connection in the pool.
func (p *ConnPool) Put(key string, conn net.Conn) {
	p.mu.Lock()
	defer p.mu.Unlock()

	// Close existing connection if any
	if old, ok := p.conns[key]; ok {
		old.conn.Close()
	}

	p.conns[key] = &poolEntry{
		conn:     conn,
		lastUsed: time.Now(),
	}
}

// Evict removes and closes expired connections.
func (p *ConnPool) Evict() {
	p.mu.Lock()
	defer p.mu.Unlock()

	now := time.Now()
	for key, entry := range p.conns {
		if now.Sub(entry.lastUsed) > p.ttl {
			entry.conn.Close()
			delete(p.conns, key)
		}
	}
}

// Close closes all pooled connections.
func (p *ConnPool) Close() {
	p.mu.Lock()
	defer p.mu.Unlock()

	for key, entry := range p.conns {
		entry.conn.Close()
		delete(p.conns, key)
	}
}

// StartEviction starts a background goroutine to evict expired connections.
func (p *ConnPool) StartEviction(ctx context.Context) {
	ticker := time.NewTicker(p.ttl / 2)
	go func() {
		defer ticker.Stop()
		for {
			select {
			case <-ctx.Done():
				return
			case <-ticker.C:
				p.Evict()
			}
		}
	}()
}
