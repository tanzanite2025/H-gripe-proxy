package tunnel

import (
	"sync"

	C "github.com/tanzanite2025/mihomo-optimized/constant"
)

// ruleCacheEntry stores a cached rule match result
type ruleCacheEntry struct {
	proxy C.Proxy
	rule  C.Rule
}

// 分片规则缓存，减少锁竞争
const ruleCacheShardCount = 32

type ruleCacheShard struct {
	mu    sync.RWMutex
	items map[uint64]ruleCacheEntry
	size  int
}

// ruleCache provides a sharded cache for rule matching results
// keyed by the metadata fields that determine rule matching.
// Sharding reduces lock contention under high concurrency.
type ruleCache struct {
	shards [ruleCacheShardCount]ruleCacheShard
}

func newRuleCache(size int) *ruleCache {
	if size <= 0 {
		size = 1000
	}
	perShard := size/ruleCacheShardCount + 1
	rc := &ruleCache{}
	for i := range rc.shards {
		rc.shards[i].items = make(map[uint64]ruleCacheEntry, perShard)
		rc.shards[i].size = perShard
	}
	return rc
}

func (rc *ruleCache) getShard(key uint64) *ruleCacheShard {
	return &rc.shards[key&(ruleCacheShardCount-1)]
}

// key computes a cache key from metadata.
// Uses the fields that determine rule matching: host, dstIP, network, process, ports, specialRules.
func ruleCacheKey(metadata *C.Metadata) uint64 {
	var h uint64 = 14695981039346656037 // FNV offset basis
	const prime uint64 = 1099511628211

	// Port: mix both src and dst for better dispersion
	h = (h ^ uint64(metadata.DstPort)) * prime
	h = (h ^ uint64(metadata.SrcPort)) * prime

	// Network type
	h = (h ^ uint64(metadata.NetWork)) * prime

	// Host (domain)
	if metadata.Host != "" {
		for _, c := range []byte(metadata.Host) {
			h = (h ^ uint64(c)) * prime
		}
	}

	// Destination IP
	if metadata.DstIP.IsValid() {
		b := metadata.DstIP.AsSlice()
		for i := 0; i < len(b); i++ {
			h = (h ^ uint64(b[i])) * prime
		}
	}

	// Process name
	if metadata.Process != "" {
		for _, c := range []byte(metadata.Process) {
			h = (h ^ uint64(c)) * prime
		}
	}

	// SpecialRules: different sub-rule sets must not share cache entries
	if metadata.SpecialRules != "" {
		for _, c := range []byte(metadata.SpecialRules) {
			h = (h ^ uint64(c)) * prime
		}
	}

	return h
}

// Get returns the cached match result, if any.
func (rc *ruleCache) Get(metadata *C.Metadata) (C.Proxy, C.Rule, bool) {
	key := ruleCacheKey(metadata)
	s := rc.getShard(key)
	s.mu.RLock()
	defer s.mu.RUnlock()
	if entry, ok := s.items[key]; ok {
		return entry.proxy, entry.rule, true
	}
	return nil, nil, false
}

// Put stores a match result in the cache.
func (rc *ruleCache) Put(metadata *C.Metadata, proxy C.Proxy, rule C.Rule) {
	key := ruleCacheKey(metadata)
	s := rc.getShard(key)
	s.mu.Lock()
	defer s.mu.Unlock()
	if len(s.items) >= s.size {
		// Eviction: clear the shard when full.
		s.items = make(map[uint64]ruleCacheEntry, s.size)
	}
	s.items[key] = ruleCacheEntry{proxy: proxy, rule: rule}
}

// Invalidate clears the entire cache. Called when rules are updated.
func (rc *ruleCache) Invalidate() {
	for i := range rc.shards {
		s := &rc.shards[i]
		s.mu.Lock()
		s.items = make(map[uint64]ruleCacheEntry, s.size)
		s.mu.Unlock()
	}
}
