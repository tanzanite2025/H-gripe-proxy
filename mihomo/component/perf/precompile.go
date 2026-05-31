package perf

import (
	"crypto/sha256"
	"encoding/gob"
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/metacubex/mihomo/log"
)

// ============================================================
// 问题4: 启动优化 — 配置预编译
// ============================================================

// PrecompiledConfig 预编译配置缓存
// 将解析后的配置序列化到磁盘，下次启动时直接加载
type PrecompiledConfig struct {
	ConfigHash string    // 原始配置文件的 SHA256
	CreatedAt  time.Time // 创建时间
	Version    int       // 预编译格式版本
	Data       []byte    // gob 编码的配置数据
}

const (
	precompileVersion = 1
	precompileDir     = "perf-cache"
	precompileFile    = "config.precompiled"
)

// ConfigPrecompiler 配置预编译器
type ConfigPrecompiler struct {
	mu    sync.Mutex
	cacheDir string
}

// NewConfigPrecompiler 创建配置预编译器
func NewConfigPrecompiler(baseDir string) *ConfigPrecompiler {
	cacheDir := filepath.Join(baseDir, precompileDir)
	return &ConfigPrecompiler{cacheDir: cacheDir}
}

// ComputeHash 计算配置文件的 SHA256 哈希
func ComputeHash(configBytes []byte) string {
	h := sha256.Sum256(configBytes)
	return fmt.Sprintf("%x", h[:16]) // 取前 16 字节作为短哈希
}

// Save 保存预编译配置
func (cp *ConfigPrecompiler) Save(hash string, data []byte) error {
	cp.mu.Lock()
	defer cp.mu.Unlock()

	if err := os.MkdirAll(cp.cacheDir, 0755); err != nil {
		return fmt.Errorf("create cache dir: %w", err)
	}

	precompiled := PrecompiledConfig{
		ConfigHash: hash,
		CreatedAt:  time.Now(),
		Version:    precompileVersion,
		Data:       data,
	}

	path := cp.cachePath()
	f, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("create cache file: %w", err)
	}
	defer f.Close()

	enc := gob.NewEncoder(f)
	if err := enc.Encode(precompiled); err != nil {
		os.Remove(path)
		return fmt.Errorf("encode precompiled config: %w", err)
	}

	log.Infoln("[Perf] precompiled config saved (hash=%s, size=%d)", hash[:8], len(data))
	return nil
}

// Load 加载预编译配置
func (cp *ConfigPrecompiler) Load(hash string) ([]byte, bool) {
	cp.mu.Lock()
	defer cp.mu.Unlock()

	path := cp.cachePath()
	f, err := os.Open(path)
	if err != nil {
		return nil, false
	}
	defer f.Close()

	var precompiled PrecompiledConfig
	dec := gob.NewDecoder(f)
	if err := dec.Decode(&precompiled); err != nil {
		log.Warnln("[Perf] failed to decode precompiled config: %s", err)
		return nil, false
	}

	// 验证哈希和版本
	if precompiled.ConfigHash != hash || precompiled.Version != precompileVersion {
		log.Infoln("[Perf] precompiled config mismatch, will recompile")
		return nil, false
	}

	// 检查过期（超过 24 小时）
	if time.Since(precompiled.CreatedAt) > 24*time.Hour {
		log.Infoln("[Perf] precompiled config expired")
		return nil, false
	}

	log.Infoln("[Perf] loaded precompiled config (hash=%s, age=%s)",
		hash[:8], time.Since(precompiled.CreatedAt).Round(time.Second))
	return precompiled.Data, true
}

// Invalidate 使预编译缓存失效
func (cp *ConfigPrecompiler) Invalidate() error {
	cp.mu.Lock()
	defer cp.mu.Unlock()

	path := cp.cachePath()
	if _, err := os.Stat(path); os.IsNotExist(err) {
		return nil
	}
	return os.Remove(path)
}

func (cp *ConfigPrecompiler) cachePath() string {
	return filepath.Join(cp.cacheDir, precompileFile)
}

// ============================================================
// 规则增量加载
// ============================================================

// IncrementalRuleLoader 增量规则加载器
// 跟踪规则提供者的版本，只重新加载变更的部分
type IncrementalRuleLoader struct {
	mu       sync.RWMutex
	versions map[string]string // provider name -> hash
}

// NewIncrementalRuleLoader 创建增量规则加载器
func NewIncrementalRuleLoader() *IncrementalRuleLoader {
	return &IncrementalRuleLoader{
		versions: make(map[string]string),
	}
}

// RecordVersion 记录规则提供者版本
func (irl *IncrementalRuleLoader) RecordVersion(name, hash string) {
	irl.mu.Lock()
	defer irl.mu.Unlock()
	irl.versions[name] = hash
}

// HasChanged 检查规则提供者是否有变更
func (irl *IncrementalRuleLoader) HasChanged(name, hash string) bool {
	irl.mu.RLock()
	defer irl.mu.RUnlock()
	old, exists := irl.versions[name]
	if !exists {
		return true // 首次加载
	}
	return old != hash
}

// GetVersion 获取规则提供者版本
func (irl *IncrementalRuleLoader) GetVersion(name string) (string, bool) {
	irl.mu.RLock()
	defer irl.mu.RUnlock()
	v, ok := irl.versions[name]
	return v, ok
}

// Snapshot 返回所有版本快照
func (irl *IncrementalRuleLoader) Snapshot() map[string]string {
	irl.mu.RLock()
	defer irl.mu.RUnlock()
	snap := make(map[string]string, len(irl.versions))
	for k, v := range irl.versions {
		snap[k] = v
	}
	return snap
}

// ============================================================
// 全局实例
// ============================================================

var (
	DefaultIncrementalLoader = NewIncrementalRuleLoader()
)
