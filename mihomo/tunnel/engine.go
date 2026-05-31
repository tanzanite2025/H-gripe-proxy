package tunnel

import (
	"context"
	"io"
	"net"
	"sync"
	"sync/atomic"
	"time"

	C "github.com/metacubex/mihomo/constant"
	"github.com/metacubex/mihomo/log"
)

// ============================================================
// Problem 1: Chain Proxy Forwarding - Connection Pool & Mux
// ============================================================

// ProxyConnPool manages reusable connections to proxy servers.
type ProxyConnPool struct {
	mu             sync.Mutex
	conns          map[poolKey][]*pooledConn
	maxIdlePerKey  int
	maxIdleTime    time.Duration
	cleanupInterval time.Duration
	closed         bool
	done           chan struct{}
}

type poolKey struct {
	proxyName string
	destAddr  string
	network   string
}

type pooledConn struct {
	conn      net.Conn
	createdAt time.Time
	lastUsed  time.Time
	key       poolKey
}

// NewProxyConnPool creates a connection pool for proxy connection reuse.
func NewProxyConnPool(maxIdlePerKey int, maxIdleTime time.Duration) *ProxyConnPool {
	p := &ProxyConnPool{
		conns:           make(map[poolKey][]*pooledConn),
		maxIdlePerKey:   maxIdlePerKey,
		maxIdleTime:     maxIdleTime,
		cleanupInterval: 30 * time.Second,
		done:            make(chan struct{}),
	}
	go p.cleanupLoop()
	return p
}

// Get retrieves a reusable connection from the pool.
func (p *ProxyConnPool) Get(proxyName, destAddr, network string) (net.Conn, bool) {
	p.mu.Lock()
	defer p.mu.Unlock()

	if p.closed {
		return nil, false
	}

	key := poolKey{proxyName: proxyName, destAddr: destAddr, network: network}
	conns := p.conns[key]
	for i := len(conns) - 1; i >= 0; i-- {
		pc := conns[i]
		if time.Since(pc.lastUsed) > p.maxIdleTime {
			_ = pc.conn.Close()
			conns = append(conns[:i], conns[i+1:]...)
			continue
		}
		if isConnAlive(pc.conn) {
			conns = append(conns[:i], conns[i+1:]...)
			if len(conns) == 0 {
				delete(p.conns, key)
			} else {
				p.conns[key] = conns
			}
			pc.lastUsed = time.Now()
			log.Debugln("[ConnPool] reuse conn: %s/%s -> %s", proxyName, network, destAddr)
			return pc.conn, true
		}
		_ = pc.conn.Close()
		conns = append(conns[:i], conns[i+1:]...)
	}
	if len(conns) == 0 {
		delete(p.conns, key)
	} else {
		p.conns[key] = conns
	}
	return nil, false
}

// Put returns a connection to the pool for future reuse.
func (p *ProxyConnPool) Put(proxyName, destAddr, network string, conn net.Conn) {
	p.mu.Lock()
	defer p.mu.Unlock()

	if p.closed {
		_ = conn.Close()
		return
	}

	key := poolKey{proxyName: proxyName, destAddr: destAddr, network: network}
	conns := p.conns[key]

	if len(conns) >= p.maxIdlePerKey {
		_ = conn.Close()
		return
	}

	p.conns[key] = append(conns, &pooledConn{
		conn:      conn,
		createdAt: time.Now(),
		lastUsed:  time.Now(),
		key:       key,
	})
	log.Debugln("[ConnPool] return conn: %s/%s -> %s (pool size: %d)", proxyName, network, destAddr, len(p.conns[key]))
}

// Close shuts down the pool and closes all idle connections.
func (p *ProxyConnPool) Close() {
	p.mu.Lock()
	defer p.mu.Unlock()

	if p.closed {
		return
	}
	p.closed = true
	close(p.done)

	for key, conns := range p.conns {
		for _, pc := range conns {
			_ = pc.conn.Close()
		}
		delete(p.conns, key)
	}
}

func (p *ProxyConnPool) cleanupLoop() {
	ticker := time.NewTicker(p.cleanupInterval)
	defer ticker.Stop()

	for {
		select {
		case <-p.done:
			return
		case <-ticker.C:
			p.cleanStale()
		}
	}
}

func (p *ProxyConnPool) cleanStale() {
	p.mu.Lock()
	defer p.mu.Unlock()

	now := time.Now()
	for key, conns := range p.conns {
		alive := conns[:0]
		for _, pc := range conns {
			if now.Sub(pc.lastUsed) > p.maxIdleTime || !isConnAlive(pc.conn) {
				_ = pc.conn.Close()
			} else {
				alive = append(alive, pc)
			}
		}
		if len(alive) == 0 {
			delete(p.conns, key)
		} else {
			p.conns[key] = alive
		}
	}
}

// isConnAlive checks if a connection is still usable without reading data.
func isConnAlive(conn net.Conn) bool {
	_ = conn.SetReadDeadline(time.Now().Add(1 * time.Millisecond))
	var b [1]byte
	_, err := conn.Read(b[:])
	_ = conn.SetReadDeadline(time.Time{})
	if err != nil {
		if netErr, ok := err.(net.Error); ok && netErr.Timeout() {
			return true
		}
		return false
	}
	return true
}

// ============================================================
// Problem 1: Traffic Scheduling & Load Balancing
// ============================================================

// ScheduleStrategy defines how to select among multiple proxy adapters.
type ScheduleStrategy int

const (
	ScheduleRoundRobin ScheduleStrategy = iota
	ScheduleLeastConn
	ScheduleLeastLatency
	ScheduleRandom
)

// ProxyScheduler selects proxy adapters based on a scheduling strategy.
type ProxyScheduler struct {
	mu         sync.Mutex
	strategy   ScheduleStrategy
	adapters   []C.ProxyAdapter
	rrIndex    atomic.Int64
	connCounts map[string]*atomic.Int64
	latencies  map[string]*atomic.Int64
}

// NewProxyScheduler creates a scheduler for the given adapters.
func NewProxyScheduler(strategy ScheduleStrategy, adapters []C.ProxyAdapter) *ProxyScheduler {
	s := &ProxyScheduler{
		strategy:   strategy,
		adapters:   adapters,
		connCounts: make(map[string]*atomic.Int64),
		latencies:  make(map[string]*atomic.Int64),
	}
	for _, a := range adapters {
		s.connCounts[a.Name()] = &atomic.Int64{}
		s.latencies[a.Name()] = &atomic.Int64{}
	}
	return s
}

// Select picks the best proxy adapter based on the scheduling strategy.
func (s *ProxyScheduler) Select() C.ProxyAdapter {
	s.mu.Lock()
	adapters := s.adapters
	s.mu.Unlock()

	if len(adapters) == 0 {
		return nil
	}
	if len(adapters) == 1 {
		return adapters[0]
	}

	switch s.strategy {
	case ScheduleRoundRobin:
		idx := s.rrIndex.Add(1) % int64(len(adapters))
		return adapters[idx]

	case ScheduleLeastConn:
		var best C.ProxyAdapter
		var bestCount int64 = 1<<63 - 1
		for _, a := range adapters {
			count := s.connCounts[a.Name()].Load()
			if count < bestCount {
				bestCount = count
				best = a
			}
		}
		return best

	case ScheduleLeastLatency:
		var best C.ProxyAdapter
		var bestLat int64 = 1<<63 - 1
		for _, a := range adapters {
			lat := s.latencies[a.Name()].Load()
			if lat > 0 && lat < bestLat {
				bestLat = lat
				best = a
			}
		}
		if best == nil {
			return adapters[0]
		}
		return best

	case ScheduleRandom:
		idx := int(s.rrIndex.Add(1)) % len(adapters)
		return adapters[idx]

	default:
		return adapters[0]
	}
}

// RecordConnect records a connection being established to a proxy.
func (s *ProxyScheduler) RecordConnect(name string) {
	if counter, ok := s.connCounts[name]; ok {
		counter.Add(1)
	}
}

// RecordDisconnect records a connection being closed.
func (s *ProxyScheduler) RecordDisconnect(name string) {
	if counter, ok := s.connCounts[name]; ok {
		counter.Add(-1)
	}
}

// RecordLatency records a latency measurement for a proxy.
func (s *ProxyScheduler) RecordLatency(name string, latency time.Duration) {
	if tracker, ok := s.latencies[name]; ok {
		ms := latency.Milliseconds()
		old := tracker.Load()
		if old == 0 {
			tracker.Store(ms)
		} else {
			tracker.Store((old*7 + ms*3) / 10)
		}
	}
}

// ============================================================
// Problem 1: Failover
// ============================================================

// FailoverGroup wraps multiple proxy adapters with automatic failover.
type FailoverGroup struct {
	mu             sync.Mutex
	adapters       []C.ProxyAdapter
	index          int
	failCounts     map[string]int
	maxFails       int
	cooldown       time.Duration
	cooldownUntil  map[string]time.Time
}

// NewFailoverGroup creates a failover group from a list of proxy adapters.
func NewFailoverGroup(adapters []C.ProxyAdapter, maxFails int, cooldown time.Duration) *FailoverGroup {
	return &FailoverGroup{
		adapters:      adapters,
		failCounts:    make(map[string]int),
		maxFails:      maxFails,
		cooldown:      cooldown,
		cooldownUntil: make(map[string]time.Time),
	}
}

// DialContext tries to dial through the current primary, falling back on failure.
func (fg *FailoverGroup) DialContext(ctx context.Context, metadata *C.Metadata) (C.Conn, error) {
	fg.mu.Lock()
	adapters := fg.adapters
	startIdx := fg.index
	fg.mu.Unlock()

	for i := 0; i < len(adapters); i++ {
		idx := (startIdx + i) % len(adapters)
		adapter := adapters[idx]

		fg.mu.Lock()
		if until, ok := fg.cooldownUntil[adapter.Name()]; ok && time.Now().Before(until) {
			fg.mu.Unlock()
			continue
		}
		fg.mu.Unlock()

		conn, err := adapter.DialContext(ctx, metadata)
		if err == nil {
			fg.mu.Lock()
			fg.failCounts[adapter.Name()] = 0
			fg.index = idx
			fg.mu.Unlock()
			return conn, nil
		}

		fg.mu.Lock()
		fg.failCounts[adapter.Name()]++
		if fg.failCounts[adapter.Name()] >= fg.maxFails {
			fg.cooldownUntil[adapter.Name()] = time.Now().Add(fg.cooldown)
			log.Warnln("[Failover] adapter %s reached max fails (%d), cooling down for %v",
				adapter.Name(), fg.maxFails, fg.cooldown)
		}
		fg.mu.Unlock()

		log.Debugln("[Failover] adapter %s failed: %s, trying next", adapter.Name(), err)
	}

	return nil, net.ErrClosed
}

// RecordSuccess records a successful connection for the current primary.
func (fg *FailoverGroup) RecordSuccess(name string) {
	fg.mu.Lock()
	defer fg.mu.Unlock()
	fg.failCounts[name] = 0
}

// ============================================================
// Problem 4: Connection Management
// ============================================================

// ConnManager manages active connections with smart reuse, zombie cleanup,
// and TCP keep-alive configuration.
type ConnManager struct {
	mu               sync.Mutex
	conns            map[string]*managedConn
	maxTotal         int
	keepAliveEnabled bool
	keepAlivePeriod  time.Duration
	zombieTimeout    time.Duration
	cleanupInterval  time.Duration
	closed           bool
	done             chan struct{}
}

type managedConn struct {
	conn       net.Conn
	metadata   *C.Metadata
	createdAt  time.Time
	lastActive atomic.Int64
	bytesSent  atomic.Int64
	bytesRecv  atomic.Int64
	priority   int
}

// NewConnManager creates a connection manager with the given limits.
func NewConnManager(maxTotal int, keepAlivePeriod, zombieTimeout time.Duration) *ConnManager {
	cm := &ConnManager{
		conns:            make(map[string]*managedConn),
		maxTotal:         maxTotal,
		keepAliveEnabled: keepAlivePeriod > 0,
		keepAlivePeriod:  keepAlivePeriod,
		zombieTimeout:    zombieTimeout,
		cleanupInterval:  15 * time.Second,
		done:             make(chan struct{}),
	}
	go cm.cleanupLoop()
	return cm
}

// Track adds a connection to the manager.
func (cm *ConnManager) Track(id string, conn net.Conn, metadata *C.Metadata, priority int) {
	cm.mu.Lock()
	defer cm.mu.Unlock()

	if cm.closed {
		return
	}

	if cm.keepAliveEnabled {
		if tcpConn, ok := conn.(*net.TCPConn); ok {
			_ = tcpConn.SetKeepAlive(true)
			_ = tcpConn.SetKeepAlivePeriod(cm.keepAlivePeriod)
		}
	}

	mc := &managedConn{
		conn:      conn,
		metadata:  metadata,
		createdAt: time.Now(),
		priority:  priority,
	}
	mc.lastActive.Store(time.Now().UnixMilli())

	if len(cm.conns) >= cm.maxTotal {
		cm.evictOne()
	}

	cm.conns[id] = mc
}

// Untrack removes a connection from the manager.
func (cm *ConnManager) Untrack(id string) {
	cm.mu.Lock()
	defer cm.mu.Unlock()
	delete(cm.conns, id)
}

// RecordActivity updates the last-active timestamp for a connection.
func (cm *ConnManager) RecordActivity(id string) {
	cm.mu.Lock()
	mc, ok := cm.conns[id]
	cm.mu.Unlock()
	if ok {
		mc.lastActive.Store(time.Now().UnixMilli())
	}
}

// RecordBytes records bytes transferred for a connection.
func (cm *ConnManager) RecordBytes(id string, sent, recv int) {
	cm.mu.Lock()
	mc, ok := cm.conns[id]
	cm.mu.Unlock()
	if ok {
		mc.bytesSent.Add(int64(sent))
		mc.bytesRecv.Add(int64(recv))
	}
}

// ActiveCount returns the number of tracked connections.
func (cm *ConnManager) ActiveCount() int {
	cm.mu.Lock()
	defer cm.mu.Unlock()
	return len(cm.conns)
}

// Close shuts down the connection manager.
func (cm *ConnManager) Close() {
	cm.mu.Lock()
	defer cm.mu.Unlock()
	cm.closed = true
	close(cm.done)
}

func (cm *ConnManager) evictOne() {
	var (
		worstID   string
		worstPri  int = 1<<63 - 1
		worstTime int64
	)
	for id, mc := range cm.conns {
		lastActive := mc.lastActive.Load()
		if mc.priority < worstPri || (mc.priority == worstPri && lastActive < worstTime) {
			worstPri = mc.priority
			worstTime = lastActive
			worstID = id
		}
	}
	if worstID != "" {
		mc := cm.conns[worstID]
		_ = mc.conn.Close()
		delete(cm.conns, worstID)
		log.Debugln("[ConnManager] evicted connection: %s (priority: %d)", worstID, worstPri)
	}
}

func (cm *ConnManager) cleanupLoop() {
	ticker := time.NewTicker(cm.cleanupInterval)
	defer ticker.Stop()

	for {
		select {
		case <-cm.done:
			return
		case <-ticker.C:
			cm.cleanZombies()
		}
	}
}

func (cm *ConnManager) cleanZombies() {
	cm.mu.Lock()
	defer cm.mu.Unlock()

	now := time.Now()
	for id, mc := range cm.conns {
		lastActiveMs := mc.lastActive.Load()
		lastActive := time.UnixMilli(lastActiveMs)
		if now.Sub(lastActive) > cm.zombieTimeout {
			_ = mc.conn.Close()
			delete(cm.conns, id)
			log.Debugln("[ConnManager] cleaned zombie connection: %s (idle: %v)", id, now.Sub(lastActive))
		}
	}
}

// ============================================================
// Problem 5: Link Quality & Egress Monitor
// ============================================================

// LinkQuality tracks quality metrics for a proxy chain link.
type LinkQuality struct {
	mu             sync.Mutex
	latencySamples []time.Duration
	maxSamples     int
	successCount   atomic.Int64
	failCount      atomic.Int64
	totalBytesSent atomic.Int64
	totalBytesRecv atomic.Int64
	lastSuccess    atomic.Int64
	lastFail       atomic.Int64
}

// NewLinkQuality creates a link quality tracker.
func NewLinkQuality(maxSamples int) *LinkQuality {
	return &LinkQuality{
		latencySamples: make([]time.Duration, 0, maxSamples),
		maxSamples:     maxSamples,
	}
}

// RecordSuccess records a successful connection with latency.
func (lq *LinkQuality) RecordSuccess(latency time.Duration) {
	lq.successCount.Add(1)
	lq.lastSuccess.Store(time.Now().UnixMilli())

	lq.mu.Lock()
	defer lq.mu.Unlock()
	lq.latencySamples = append(lq.latencySamples, latency)
	if len(lq.latencySamples) > lq.maxSamples {
		lq.latencySamples = lq.latencySamples[1:]
	}
}

// RecordFailure records a failed connection attempt.
func (lq *LinkQuality) RecordFailure() {
	lq.failCount.Add(1)
	lq.lastFail.Store(time.Now().UnixMilli())
}

// RecordBytes records bytes transferred.
func (lq *LinkQuality) RecordBytes(sent, recv int) {
	lq.totalBytesSent.Add(int64(sent))
	lq.totalBytesRecv.Add(int64(recv))
}

// AvgLatency returns the average latency from recent samples.
func (lq *LinkQuality) AvgLatency() time.Duration {
	lq.mu.Lock()
	defer lq.mu.Unlock()

	if len(lq.latencySamples) == 0 {
		return 0
	}
	var total time.Duration
	for _, s := range lq.latencySamples {
		total += s
	}
	return total / time.Duration(len(lq.latencySamples))
}

// P50Latency returns the median latency.
func (lq *LinkQuality) P50Latency() time.Duration {
	lq.mu.Lock()
	defer lq.mu.Unlock()

	if len(lq.latencySamples) == 0 {
		return 0
	}
	sorted := make([]time.Duration, len(lq.latencySamples))
	copy(sorted, lq.latencySamples)
	for i := 1; i < len(sorted); i++ {
		for j := i; j > 0 && sorted[j] < sorted[j-1]; j-- {
			sorted[j], sorted[j-1] = sorted[j-1], sorted[j]
		}
	}
	return sorted[len(sorted)/2]
}

// SuccessRate returns the success rate as a fraction [0, 1].
func (lq *LinkQuality) SuccessRate() float64 {
	s := lq.successCount.Load()
	f := lq.failCount.Load()
	total := s + f
	if total == 0 {
		return 1.0
	}
	return float64(s) / float64(total)
}

// IsHealthy returns true if the link quality is above thresholds.
func (lq *LinkQuality) IsHealthy(maxLatency time.Duration, minSuccessRate float64) bool {
	if lq.SuccessRate() < minSuccessRate {
		return false
	}
	avg := lq.AvgLatency()
	if avg > 0 && avg > maxLatency {
		return false
	}
	return true
}

// EgressMonitor monitors egress IP stability for a proxy chain.
type EgressMonitor struct {
	mu          sync.Mutex
	egressIPs   map[string]time.Time
	lockedIP    string
	locked      bool
	changeCount atomic.Int64
}

// NewEgressMonitor creates an egress IP monitor.
func NewEgressMonitor() *EgressMonitor {
	return &EgressMonitor{
		egressIPs: make(map[string]time.Time),
	}
}

// RecordEgress records an observed egress IP.
func (em *EgressMonitor) RecordEgress(ip string) {
	em.mu.Lock()
	defer em.mu.Unlock()

	now := time.Now()
	if _, exists := em.egressIPs[ip]; !exists {
		em.egressIPs[ip] = now
		if len(em.egressIPs) > 1 {
			em.changeCount.Add(1)
			log.Warnln("[EgressMonitor] egress IP changed to %s (changes: %d)", ip, em.changeCount.Load())
		}
	}

	if em.locked && em.lockedIP != "" && em.lockedIP != ip {
		log.Warnln("[EgressMonitor] egress IP %s differs from locked IP %s", ip, em.lockedIP)
	}
}

// LockEgress locks the expected egress IP.
func (em *EgressMonitor) LockEgress(ip string) {
	em.mu.Lock()
	defer em.mu.Unlock()
	em.lockedIP = ip
	em.locked = true
}

// IsEgressStable returns true if the egress IP has not changed.
func (em *EgressMonitor) IsEgressStable() bool {
	em.mu.Lock()
	defer em.mu.Unlock()
	return len(em.egressIPs) <= 1
}

// ChangeCount returns the number of egress IP changes observed.
func (em *EgressMonitor) ChangeCount() int64 {
	return em.changeCount.Load()
}

// ============================================================
// Enhanced Relay with statistics tracking
// ============================================================

// RelayWithStats copies between left and right bidirectionally with
// per-connection byte counting and activity tracking.
func RelayWithStats(leftConn, rightConn net.Conn, leftID, rightID string, connMgr *ConnManager) {
	defer func() {
		_ = leftConn.Close()
		_ = rightConn.Close()
		if connMgr != nil {
			connMgr.Untrack(leftID)
			connMgr.Untrack(rightID)
		}
	}()

	ch := make(chan struct{})
	go func() {
		n, _ := bufioCopyWithStats(leftConn, rightConn, rightID, connMgr)
		_ = closeWriteForRelay(leftConn)
		_ = n
		close(ch)
	}()

	n, _ := bufioCopyWithStats(rightConn, leftConn, leftID, connMgr)
	_ = closeWriteForRelay(rightConn)
	_ = n
	<-ch
}

func bufioCopyWithStats(dst io.Writer, src io.Reader, connID string, connMgr *ConnManager) (int64, error) {
	var written int64
	buf := make([]byte, 32*1024)
	for {
		nr, err := src.Read(buf)
		if nr > 0 {
			nw, errW := dst.Write(buf[:nr])
			if nw > 0 {
				written += int64(nw)
				if connMgr != nil {
					connMgr.RecordActivity(connID)
				}
			}
			if errW != nil {
				return written, errW
			}
			if nr != nw {
				return written, io.ErrShortWrite
			}
		}
		if err != nil {
			return written, err
		}
	}
}

func closeWriteForRelay(conn net.Conn) error {
	if c, ok := conn.(interface{ CloseWrite() error }); ok {
		return c.CloseWrite()
	}
	return nil
}
