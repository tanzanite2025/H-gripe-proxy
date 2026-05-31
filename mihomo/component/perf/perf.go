package perf

import (
	"runtime"
	"runtime/debug"
	"sync"
	"sync/atomic"
	"time"
	"unsafe"

	"github.com/tanzanite2025/mihomo-optimized/log"
)

// ============================================================
// 问题1: CPU 优化
// ============================================================

// --- SIMD 辅助: 内存布局优化 ---

// CacheLinePad 避免 false sharing，填充到 64 字节缓存行
type CacheLinePad struct {
	_ [cacheLineSize]byte
}

const cacheLineSize = 64

// PaddedAtomicInt64 是缓存行对齐的 atomic.Int64，避免 false sharing
type PaddedAtomicInt64 struct {
	atomic.Int64
	_ [cacheLineSize - unsafe.Sizeof(atomic.Int64{})]byte
}

// PaddedAtomicUint64 是缓存行对齐的 atomic.Uint64
type PaddedAtomicUint64 struct {
	atomic.Uint64
	_ [cacheLineSize - unsafe.Sizeof(atomic.Uint64{})]byte
}

// --- 规则匹配 SIMD 加速 ---

// SIMDAccelerator 提供 SIMD 风格的批量规则匹配加速
// 在 Go 中无法直接使用 SIMD 指令，但可以通过以下方式获得类似效果：
// 1. 内存连续布局提高缓存命中率
// 2. 批量处理减少分支预测失败
// 3. 循环展开减少循环开销
type SIMDAccelerator struct {
	mu sync.RWMutex
}

var DefaultSIMDAccelerator = &SIMDAccelerator{}

// BatchMatchIP 批量 IP 匹配，使用连续内存布局提高缓存命中率
// 将 IP 地址打包为连续 uint32 数组，一次性遍历
func BatchMatchIP(ips []uint32, rules []uint32, masks []uint32) []bool {
	n := len(ips)
	result := make([]bool, n)

	// 循环展开：每次处理 4 个 IP
	i := 0
	for ; i+3 < n; i += 4 {
		ip0, ip1, ip2, ip3 := ips[i], ips[i+1], ips[i+2], ips[i+3]
		for j := range rules {
			rule, mask := rules[j], masks[j]
			if !result[i] && ip0&mask == rule&mask {
				result[i] = true
			}
			if !result[i+1] && ip1&mask == rule&mask {
				result[i+1] = true
			}
			if !result[i+2] && ip2&mask == rule&mask {
				result[i+2] = true
			}
			if !result[i+3] && ip3&mask == rule&mask {
				result[i+3] = true
			}
			// 全部已匹配则提前退出内层循环
			if result[i] && result[i+1] && result[i+2] && result[i+3] {
				break
			}
		}
	}

	// 处理剩余
	for ; i < n; i++ {
		ip := ips[i]
		for j := range rules {
			if ip&masks[j] == rules[j]&masks[j] {
				result[i] = true
				break
			}
		}
	}

	return result
}

// BatchMatchPort 批量端口匹配，使用位图加速
// 将端口范围预编译为位图，O(1) 查找
type PortBitmap struct {
	bitmap [65536 / 8]uint8 // 8KB 覆盖全部 65536 端口
}

// NewPortBitmap 从端口范围创建位图
func NewPortBitmap(ranges [][2]uint16) *PortBitmap {
	pb := &PortBitmap{}
	for _, r := range ranges {
		for port := r[0]; port <= r[1]; port++ {
			pb.bitmap[port/8] |= 1 << (port % 8)
		}
	}
	return pb
}

// Match 检查端口是否在位图中
func (pb *PortBitmap) Match(port uint16) bool {
	return pb.bitmap[port/8]&(1<<(port%8)) != 0
}

// --- 加密操作优化 ---

// CryptoOptimization 提供加密操作的优化配置
type CryptoOptimization struct {
	// 使用硬件加速的 AES-GCM
	AESHardwareAccel bool
	// 预分配的加密缓冲区
	BufferPool *sync.Pool
}

var DefaultCryptoOpt = &CryptoOptimization{
	AESHardwareAccel: true,
	BufferPool: &sync.Pool{
		New: func() any {
			buf := make([]byte, 16*1024) // 16KB 预分配
			return &buf
		},
	},
}

// GetCryptoBuffer 从池中获取加密缓冲区
func (co *CryptoOptimization) GetCryptoBuffer() *[]byte {
	return co.BufferPool.Get().(*[]byte)
}

// PutCryptoBuffer 归还加密缓冲区
func (co *CryptoOptimization) PutCryptoBuffer(buf *[]byte) {
	co.BufferPool.Put(buf)
}

// ============================================================
// 问题2: 内存优化
// ============================================================

// --- 规则树压缩 ---

// CompactRuleNode 压缩规则树节点
// 使用更紧凑的内存布局减少指针和内存占用
type CompactRuleNode struct {
	// 内联存储常见字段，避免额外分配
	ruleType uint8  // 规则类型枚举
	adapter  uint16 // 适配器索引（替代字符串指针）
	payload  string // payload 使用字符串头共享
	isNegate bool
	children []CompactRuleNode // 子节点
}

// CompactRuleTree 压缩规则树
type CompactRuleTree struct {
	nodes []CompactRuleNode
	// 适配器名称表：索引 → 名称
	adapterTable []string
	adapterMap   map[string]uint16 // 名称 → 索引
	mu           sync.RWMutex
}

// NewCompactRuleTree 创建压缩规则树
func NewCompactRuleTree() *CompactRuleTree {
	return &CompactRuleTree{
		adapterMap: make(map[string]uint16),
	}
}

// GetAdapterIndex 获取或创建适配器索引
func (t *CompactRuleTree) GetAdapterIndex(name string) (uint16, bool) {
	t.mu.Lock()
	defer t.mu.Unlock()

	if idx, ok := t.adapterMap[name]; ok {
		return idx, true
	}
	if len(t.adapterTable) >= 65535 {
		log.Warnln("[Perf] CompactRuleTree adapter table overflow (>65535), cannot add: %s", name)
		return 0, false
	}
	idx := uint16(len(t.adapterTable))
	t.adapterTable = append(t.adapterTable, name)
	t.adapterMap[name] = idx
	return idx, true
}

// GetAdapterName 通过索引获取适配器名称
func (t *CompactRuleTree) GetAdapterName(idx uint16) string {
	t.mu.RLock()
	defer t.mu.RUnlock()
	if int(idx) < len(t.adapterTable) {
		return t.adapterTable[idx]
	}
	return ""
}

// --- 连接表优化 ---

// ShardedConnMap 分片连接表，减少锁竞争
type ShardedConnMap struct {
	shards [connShardCount]connShard
}

const connShardCount = 64 // 必须是 2 的幂

type connShard struct {
	mu    sync.RWMutex
	items map[string]any
}

// NewShardedConnMap 创建分片连接表
func NewShardedConnMap() *ShardedConnMap {
	m := &ShardedConnMap{}
	for i := range m.shards {
		m.shards[i].items = make(map[string]any, 64)
	}
	return m
}

func (m *ShardedConnMap) getShard(key string) *connShard {
	// FNV-1a 哈希取模
	h := uint32(2166136261)
	for _, c := range []byte(key) {
		h ^= uint32(c)
		h *= 16777619
	}
	return &m.shards[h&(connShardCount-1)]
}

// Store 存储连接
func (m *ShardedConnMap) Store(key string, val any) {
	s := m.getShard(key)
	s.mu.Lock()
	s.items[key] = val
	s.mu.Unlock()
}

// Load 加载连接
func (m *ShardedConnMap) Load(key string) (any, bool) {
	s := m.getShard(key)
	s.mu.RLock()
	v, ok := s.items[key]
	s.mu.RUnlock()
	return v, ok
}

// Delete 删除连接
func (m *ShardedConnMap) Delete(key string) {
	s := m.getShard(key)
	s.mu.Lock()
	delete(s.items, key)
	s.mu.Unlock()
}

// Len 返回总连接数
func (m *ShardedConnMap) Len() int {
	total := 0
	for i := range m.shards {
		m.shards[i].mu.RLock()
		total += len(m.shards[i].items)
		m.shards[i].mu.RUnlock()
	}
	return total
}

// --- GC 优化 ---

// GCOptimization 控制 GC 行为以减少停顿
type GCOptimization struct {
	// 目标 GC 百分比（默认 GOGC=100，增大可减少 GC 频率）
	TargetGOGC int
	// 内存限制（字节），设置后使用 GOMEMLIMIT
	MemoryLimit int64
	enabled     atomic.Bool
}

var DefaultGCOpt = &GCOptimization{
	TargetGOGC:  200,               // 比 Go 默认 100 更宽松，减少 GC 频率
	MemoryLimit: 256 * 1024 * 1024, // 256MB
}

// Apply 应用 GC 优化
func (g *GCOptimization) Apply() {
	if g.TargetGOGC > 0 {
		old := debug.SetGCPercent(g.TargetGOGC)
		log.Infoln("[Perf] GOGC adjusted: %d -> %d", old, g.TargetGOGC)
	}
	if g.MemoryLimit > 0 {
		debug.SetMemoryLimit(int64(g.MemoryLimit))
		log.Infoln("[Perf] GOMEMLIMIT set to %d bytes", g.MemoryLimit)
	}
	g.enabled.Store(true)
}

// --- 缓冲区内存池 ---

// BufferPoolConfig 缓冲池配置
type BufferPoolConfig struct {
	// 各大小级别的池
	SmallPoolSize  int // <= 2KB
	MediumPoolSize int // <= 16KB
	LargePoolSize  int // <= 64KB
}

// TieredBufferPool 分层缓冲池
type TieredBufferPool struct {
	small  sync.Pool // <= 2KB
	medium sync.Pool // <= 16KB
	large  sync.Pool // <= 64KB
}

var DefaultTieredBufferPool = &TieredBufferPool{
	small: sync.Pool{
		New: func() any { buf := make([]byte, 2*1024); return &buf },
	},
	medium: sync.Pool{
		New: func() any { buf := make([]byte, 16*1024); return &buf },
	},
	large: sync.Pool{
		New: func() any { buf := make([]byte, 64*1024); return &buf },
	},
}

// Get 获取合适大小的缓冲区
func (p *TieredBufferPool) Get(size int) *[]byte {
	switch {
	case size <= 2*1024:
		return p.small.Get().(*[]byte)
	case size <= 16*1024:
		return p.medium.Get().(*[]byte)
	default:
		return p.large.Get().(*[]byte)
	}
}

// Put 归还缓冲区
func (p *TieredBufferPool) Put(buf *[]byte) {
	size := cap(*buf)
	switch {
	case size <= 2*1024:
		p.small.Put(buf)
	case size <= 16*1024:
		p.medium.Put(buf)
	default:
		p.large.Put(buf)
	}
}

// ============================================================
// 问题4: 启动优化
// ============================================================

// ParallelInit 并行初始化器
type ParallelInit struct {
	tasks []initTask
}

type initTask struct {
	name string
	fn   func() error
}

// NewParallelInit 创建并行初始化器
func NewParallelInit() *ParallelInit {
	return &ParallelInit{}
}

// Add 添加初始化任务
func (pi *ParallelInit) Add(name string, fn func() error) {
	pi.tasks = append(pi.tasks, initTask{name: name, fn: fn})
}

// Execute 并行执行所有初始化任务
func (pi *ParallelInit) Execute() error {
	if len(pi.tasks) == 0 {
		return nil
	}

	errCh := make(chan error, len(pi.tasks))
	var wg sync.WaitGroup

	for _, task := range pi.tasks {
		wg.Add(1)
		go func(t initTask) {
			defer wg.Done()
			start := runtimeNano()
			if err := t.fn(); err != nil {
				errCh <- err
				return
			}
			elapsed := runtimeNano() - start
			log.Infoln("[Perf] init task %s completed in %d ms", t.name, elapsed/1e6)
		}(task)
	}

	wg.Wait()
	close(errCh)

	// 返回第一个错误
	for err := range errCh {
		if err != nil {
			return err
		}
	}
	return nil
}

var runtimeNano = func() int64 { return time.Now().UnixNano() }

// LazyLoader 懒加载器
type LazyLoader struct {
	mu     sync.RWMutex
	loaded bool
	loader func() error
	onLoad func()
}

// NewLazyLoader 创建懒加载器
func NewLazyLoader(loader func() error, onLoad func()) *LazyLoader {
	return &LazyLoader{loader: loader, onLoad: onLoad}
}

// EnsureLoaded 确保已加载
func (ll *LazyLoader) EnsureLoaded() error {
	ll.mu.Lock()
	defer ll.mu.Unlock()
	if ll.loaded {
		return nil
	}
	if err := ll.loader(); err != nil {
		return err
	}
	ll.loaded = true
	if ll.onLoad != nil {
		ll.onLoad()
	}
	return nil
}

// IsLoaded 返回是否已加载
func (ll *LazyLoader) IsLoaded() bool {
	ll.mu.RLock()
	defer ll.mu.RUnlock()
	return ll.loaded
}

// ============================================================
// 问题5: 热更新优化
// ============================================================

// HotReloader 热更新管理器
type HotReloader struct {
	mu          sync.RWMutex
	configHash  string
	ruleVersion atomic.Int64
	// 更新回调
	onConfigUpdate func(old, new string)
	onRuleUpdate   func(version int64)
	// 连接保护：更新期间不关闭现有连接
	protectedConns *ShardedConnMap
}

// NewHotReloader 创建热更新管理器
func NewHotReloader() *HotReloader {
	return &HotReloader{
		protectedConns: NewShardedConnMap(),
	}
}

// ProtectConnection 保护连接不被更新关闭
func (hr *HotReloader) ProtectConnection(id string, conn any) {
	hr.protectedConns.Store(id, conn)
}

// UnprotectConnection 取消连接保护
func (hr *HotReloader) UnprotectConnection(id string) {
	hr.protectedConns.Delete(id)
}

// IsConnectionProtected 检查连接是否受保护
func (hr *HotReloader) IsConnectionProtected(id string) bool {
	_, ok := hr.protectedConns.Load(id)
	return ok
}

// ProtectedCount 返回受保护的连接数
func (hr *HotReloader) ProtectedCount() int {
	return hr.protectedConns.Len()
}

// UpdateConfig 热更新配置
func (hr *HotReloader) UpdateConfig(newHash string) {
	hr.mu.Lock()
	oldHash := hr.configHash
	hr.configHash = newHash
	onUpdate := hr.onConfigUpdate
	hr.mu.Unlock()

	if oldHash != newHash && onUpdate != nil {
		onUpdate(oldHash, newHash)
		oldShort := oldHash
		if len(oldShort) > 8 {
			oldShort = oldShort[:8]
		}
		newShort := newHash
		if len(newShort) > 8 {
			newShort = newShort[:8]
		}
		log.Infoln("[Perf] config hot-updated: %s -> %s", oldShort, newShort)
	}
}

// UpdateRules 热更新规则
func (hr *HotReloader) UpdateRules() int64 {
	newVer := hr.ruleVersion.Add(1)
	hr.mu.RLock()
	onUpdate := hr.onRuleUpdate
	hr.mu.RUnlock()

	if onUpdate != nil {
		onUpdate(newVer)
	}
	log.Infoln("[Perf] rules hot-updated to version %d", newVer)
	return newVer
}

// RuleVersion 返回当前规则版本
func (hr *HotReloader) RuleVersion() int64 {
	return hr.ruleVersion.Load()
}

// SetConfigUpdateCallback 设置配置更新回调
func (hr *HotReloader) SetConfigUpdateCallback(fn func(old, new string)) {
	hr.mu.Lock()
	defer hr.mu.Unlock()
	hr.onConfigUpdate = fn
}

// SetRuleUpdateCallback 设置规则更新回调
func (hr *HotReloader) SetRuleUpdateCallback(fn func(version int64)) {
	hr.mu.Lock()
	defer hr.mu.Unlock()
	hr.onRuleUpdate = fn
}

// ============================================================
// 全局实例
// ============================================================

var (
	DefaultHotReloader = NewHotReloader()
)

// Init 应用所有性能优化
func Init() {
	log.Infoln("[Perf] applying performance optimizations...")

	// GC 优化
	DefaultGCOpt.Apply()

	// Go 1.5+ 默认 GOMAXPROCS=NumCPU，无需手动设置
	// 仅记录当前值供诊断
	log.Infoln("[Perf] GOMAXPROCS=%d, NumCPU=%d", runtime.GOMAXPROCS(0), runtime.NumCPU())

	log.Infoln("[Perf] performance optimizations applied")
}
