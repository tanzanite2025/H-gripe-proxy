//go:build !linux

package perf

import (
	"fmt"

	"github.com/tanzanite2025/mihomo-optimized/log"
)

// XDPConfig XDP 加速配置（非 Linux 平台存根）
type XDPConfig struct {
	Enabled       bool
	InterfaceName string
	Mode          string
	ZeroCopy      bool
	QueueCount    int
}

func DefaultXDPConfig() *XDPConfig {
	return &XDPConfig{Enabled: false}
}

// XDPAccelerator XDP 加速器（非 Linux 存根）
type XDPAccelerator struct {
	config *XDPConfig
}

func NewXDPAccelerator(config *XDPConfig) *XDPAccelerator {
	if config == nil {
		config = DefaultXDPConfig()
	}
	return &XDPAccelerator{config: config}
}

func (xa *XDPAccelerator) Load() error {
	if xa.config.Enabled {
		return fmt.Errorf("XDP is only supported on Linux")
	}
	return nil
}

func (xa *XDPAccelerator) Unload() error { return nil }
func (xa *XDPAccelerator) IsLoaded() bool { return false }

// TCPOptimization TCP 优化（非 Linux 存根）
type TCPOptimization struct {
	Applied bool
}

var DefaultTCPOpt = &TCPOptimization{}

func (t *TCPOptimization) Apply() error {
	log.Infoln("[Perf] TCP kernel optimizations skipped (not Linux)")
	return nil
}

// UDPGSOConfig UDP GSO 配置（非 Linux 存根）
type UDPGSOConfig struct {
	Enabled         bool
	GSOSize         int
	ChecksumOffload bool
}

func DefaultUDPGSOConfig() *UDPGSOConfig {
	return &UDPGSOConfig{Enabled: false}
}

func (c *UDPGSOConfig) Apply() error {
	log.Infoln("[Perf] UDP GSO skipped (not Linux)")
	return nil
}

var DefaultXDPAccelerator = NewXDPAccelerator(nil)
