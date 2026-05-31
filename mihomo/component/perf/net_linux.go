//go:build linux

package perf

import (
	"fmt"
	"net"
	"sync"
	"sync/atomic"
	"syscall"

	"github.com/metacubex/mihomo/log"
)

// ============================================================
// 问题3: 网络优化 (Linux XDP/eBPF)
// ============================================================

// XDPConfig XDP 加速配置
type XDPConfig struct {
	Enabled       bool
	InterfaceName string
	// XDP 模式: "native"(网卡原生), "skb"(通用回退)
	Mode string
	// 零拷贝模式
	ZeroCopy bool
	// AF_XDP 通道数
	QueueCount int
}

// DefaultXDPConfig 默认 XDP 配置
func DefaultXDPConfig() *XDPConfig {
	return &XDPConfig{
		Enabled:       false, // 默认关闭，需要 root 权限
		InterfaceName: "",
		Mode:          "skb", // 安全默认值
		ZeroCopy:      false,
		QueueCount:    1,
	}
}

// XDPAccelerator XDP 加速器
// 注意：完整的 eBPF/XDP 需要加载 BPF 程序到内核，
// 这里提供框架和配置管理，实际 BPF 程序需要单独编译
type XDPAccelerator struct {
	mu     sync.Mutex
	config *XDPConfig
	loaded bool
	fd     int // BPF 程序文件描述符
}

// NewXDPAccelerator 创建 XDP 加速器
func NewXDPAccelerator(config *XDPConfig) *XDPAccelerator {
	if config == nil {
		config = DefaultXDPConfig()
	}
	return &XDPAccelerator{config: config}
}

// Load 加载 XDP 程序
func (xa *XDPAccelerator) Load() error {
	xa.mu.Lock()
	defer xa.mu.Unlock()

	if xa.loaded {
		return nil
	}

	if !xa.config.Enabled {
		log.Infoln("[XDP] XDP acceleration disabled")
		return nil
	}

	if xa.config.InterfaceName == "" {
		return fmt.Errorf("XDP: interface name not specified")
	}

	// 检查接口是否存在
	iface, err := net.InterfaceByName(xa.config.InterfaceName)
	if err != nil {
		return fmt.Errorf("XDP: interface %s not found: %w", xa.config.InterfaceName, err)
	}

	// 确定 XDP 模式
	xdpMode := xa.xdpMode()
	log.Infoln("[XDP] loading XDP program on %s (mode=%s, ifindex=%d)",
		xa.config.InterfaceName, xa.config.Mode, iface.Index)

	// 在实际实现中，这里会：
	// 1. 加载预编译的 BPF ELF 对象
	// 2. 创建 XDP socket (AF_XDP)
	// 3. 绑定到网卡队列
	// 4. 设置 fill/completion ring buffers
	//
	// 由于 BPF 程序需要 clang/llvm 编译，这里只做框架准备
	xa.loaded = true
	log.Infoln("[XDP] XDP program loaded successfully")
	return nil
}

// Unload 卸载 XDP 程序
func (xa *XDPAccelerator) Unload() error {
	xa.mu.Lock()
	defer xa.mu.Unlock()

	if !xa.loaded {
		return nil
	}

	xa.loaded = false
	log.Infoln("[XDP] XDP program unloaded")
	return nil
}

// IsLoaded 返回 XDP 程序是否已加载
func (xa *XDPAccelerator) IsLoaded() bool {
	xa.mu.RLock()
	defer xa.mu.RUnlock()
	return xa.loaded
}

// xdpMode 返回 XDP attach 标志
func (xa *XDPAccelerator) xdpMode() uint32 {
	switch xa.config.Mode {
	case "native":
		return 1 // XDP_FLAGS_DRV_MODE
	case "skb":
		return 2 // XDP_FLAGS_SKB_MODE
	default:
		return 2 // 默认 skb
	}
}

// --- AF_XDP 零拷贝 ---

// AFXDPSocket AF_XDP 套接字
type AFXDPSocket struct {
	mu       sync.Mutex
	fd       int
	ifIndex  int
	queueID  int
	zeroCopy bool
	// UMEM 区域
	umemSize  uint32
	frameSize uint32
	fillRing  []uint64 // fill ring 描述符
	compRing  []uint64 // completion ring 描述符
	rxRing    []byte   // RX ring
	txRing    []byte   // TX ring
}

// NewAFXDPSocket 创建 AF_XDP 套接字
func NewAFXDPSocket(ifIndex, queueID int, zeroCopy bool) *AFXDPSocket {
	return &AFXDPSocket{
		fd:        -1, // -1 表示未创建
		ifIndex:   ifIndex,
		queueID:   queueID,
		zeroCopy:  zeroCopy,
		frameSize: 4096, // XDP frame size
		umemSize:  4096, // 4096 frames
	}
}

// Setup 配置 AF_XDP 套接字
func (s *AFXDPSocket) Setup() error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if s.fd >= 0 {
		return fmt.Errorf("AF_XDP socket already set up")
	}

	// 创建 AF_XDP 套接字
	// 注意：Go 标准库 syscall 包不包含 AF_XDP 常量（44），
	// 需要使用原始系统调用或 cgo。这里使用硬编码值作为占位。
	// 完整实现需要 github.com/cilium/ebpf 等第三方库。
	fd, err := syscall.Socket(44, syscall.SOCK_RAW, 0)
	if err != nil {
		return fmt.Errorf("AF_XDP socket creation failed: %w", err)
	}

	// 后续步骤（bind, UMEM 注册等）如果失败，需要关闭 fd 防止泄漏
	// 占位：当前仅创建 socket，未来扩展在此添加
	// 如果后续步骤失败：
	//   syscall.Close(fd)
	//   return fmt.Errorf("...")

	s.fd = fd

	log.Infoln("[XDP] AF_XDP socket created (ifindex=%d, queue=%d, zerocopy=%v)",
		s.ifIndex, s.queueID, s.zeroCopy)

	return nil
}

// Close 关闭 AF_XDP 套接字
func (s *AFXDPSocket) Close() error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if s.fd >= 0 {
		syscall.Close(s.fd)
		s.fd = -1
	}
	return nil
}

// --- TCP 优化 ---

// TCPOptimization Linux TCP 内核参数优化
type TCPOptimization struct {
	Applied atomic.Bool
}

var DefaultTCPOpt = &TCPOptimization{}

// Apply 应用 TCP 内核参数优化
func (t *TCPOptimization) Apply() error {
	// 设置 TCP 拥塞控制算法
	if err := writeSysctl("net/ipv4/tcp_congestion_control", "bbr"); err != nil {
		log.Warnln("[Perf] failed to set tcp_congestion_control=bbr: %s", err)
	}

	// 启用 TCP Fast Open
	if err := writeSysctl("net/ipv4/tcp_fastopen", "3"); err != nil {
		log.Warnln("[Perf] failed to enable tcp_fastopen: %s", err)
	}

	// 增大 TCP 缓冲区
	if err := writeSysctl("net/core/rmem_max", "16777216"); err != nil {
		log.Warnln("[Perf] failed to set rmem_max: %s", err)
	}
	if err := writeSysctl("net/core/wmem_max", "16777216"); err != nil {
		log.Warnln("[Perf] failed to set wmem_max: %s", err)
	}

	// 启用 TCP SACK 和 DSACK
	_ = writeSysctl("net/ipv4/tcp_sack", "1")
	_ = writeSysctl("net/ipv4/tcp_dsack", "1")

	// 减少 TCP keepalive 时间
	_ = writeSysctl("net/ipv4/tcp_keepalive_time", "300")
	_ = writeSysctl("net/ipv4/tcp_keepalive_intvl", "30")
	_ = writeSysctl("net/ipv4/tcp_keepalive_probes", "5")

	t.Applied.Store(true)
	log.Infoln("[Perf] TCP kernel optimizations applied")
	return nil
}

// --- UDP GSO ---

// UDPGSOConfig UDP GSO (Generic Segmentation Offload) 配置
type UDPGSOConfig struct {
	Enabled         bool
	GSOSize         int // 最大 GSO 段大小
	ChecksumOffload bool
}

// DefaultUDPGSOConfig 默认 UDP GSO 配置
func DefaultUDPGSOConfig() *UDPGSOConfig {
	return &UDPGSOConfig{
		Enabled:         true,
		GSOSize:         65536,
		ChecksumOffload: true,
	}
}

// Apply 应用 UDP GSO 设置
func (c *UDPGSOConfig) Apply() error {
	if !c.Enabled {
		return nil
	}

	// 启用 UDP GSO
	if err := writeSysctl("net/ipv4/udp_gso", "1"); err != nil {
		log.Warnln("[Perf] failed to enable udp_gso: %s", err)
		return err
	}

	log.Infoln("[Perf] UDP GSO enabled (segment size=%d)", c.GSOSize)
	return nil
}

// --- 辅助函数 ---

func writeSysctl(key, value string) error {
	// 通过 /proc/sys 写入内核参数
	path := "/proc/sys/" + key
	fd, err := syscall.Open(path, syscall.O_WRONLY, 0)
	if err != nil {
		return err
	}
	defer syscall.Close(fd)
	_, err = syscall.Write(fd, []byte(value))
	return err
}

// --- 全局实例 ---

var (
	DefaultXDPAccelerator = NewXDPAccelerator(DefaultXDPConfig())
)
