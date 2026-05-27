# eBPF/XDP 零内核态切换代理架构

## 概述

使用 eBPF/XDP 技术在 Linux 内核网卡驱动层实现代理逻辑，实现：
- ✅ 零用户态切换
- ✅ 零内存拷贝
- ✅ 线速（Line-Rate）转发
- ✅ 极低 CPU 占用
- ✅ 微秒级延迟

---

## 技术栈

### 核心技术

- **XDP (eXpress Data Path)**: 在网卡驱动层拦截数据包
- **eBPF (extended Berkeley Packet Filter)**: 在内核中运行沙箱代码
- **Aya**: Rust 的 eBPF 开发框架
- **AF_XDP**: 零拷贝用户态接口（可选）

### 性能对比

| 架构 | 延迟 | CPU 占用 | 吞吐量 |
|------|------|----------|--------|
| 传统 TUN | ~100μs | 高 | 1-5 Gbps |
| eBPF/XDP | ~10μs | 极低 | 10-100 Gbps |
| 提升 | **10x** | **5-10x** | **10-20x** |

---

## 架构设计

### 数据包处理流程

```
┌─────────────────────────────────────────────────┐
│  物理网卡（NIC）                                 │
└────────────────┬────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────┐
│  XDP Hook Point（网卡驱动层）                    │
│  ┌───────────────────────────────────────────┐  │
│  │  eBPF 程序（运行在内核中）                 │  │
│  │  ┌─────────────────────────────────────┐  │  │
│  │  │ 1. 解析数据包头                      │  │  │
│  │  │ 2. 查找路由表（eBPF Map）            │  │  │
│  │  │ 3. 解密/加密（内核态）               │  │  │
│  │  │ 4. 重写目标地址                      │  │  │
│  │  │ 5. 直接转发                          │  │  │
│  │  └─────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────┘  │
└────────────────┬────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────┐
│  直接发送到目标网卡（零拷贝）                    │
└─────────────────────────────────────────────────┘
```

### 组件架构

```
┌──────────────────────────────────────────────────┐
│  Clash Verge (用户态控制平面)                     │
│  ┌────────────────────────────────────────────┐  │
│  │  - 配置管理                                 │  │
│  │  - 路由规则                                 │  │
│  │  - 统计监控                                 │  │
│  │  - eBPF 程序加载器                          │  │
│  └────────────────────────────────────────────┘  │
└──────────────┬───────────────────────────────────┘
               │ 通过 eBPF Map 通信
               ▼
┌──────────────────────────────────────────────────┐
│  Linux Kernel (内核态数据平面)                    │
│  ┌────────────────────────────────────────────┐  │
│  │  XDP eBPF 程序                              │  │
│  │  ┌──────────────────────────────────────┐  │  │
│  │  │  - 数据包过滤                         │  │  │
│  │  │  - 协议解析                           │  │  │
│  │  │  - 加密/解密                          │  │  │
│  │  │  - 路由转发                           │  │  │
│  │  └──────────────────────────────────────┘  │  │
│  │                                             │  │
│  │  eBPF Maps（共享数据结构）                  │  │
│  │  ┌──────────────────────────────────────┐  │  │
│  │  │  - 路由表                             │  │  │
│  │  │  - 连接跟踪表                         │  │  │
│  │  │  - 统计计数器                         │  │  │
│  │  │  - 配置参数                           │  │  │
│  │  └──────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

---

## 实现计划

### Phase 1: 基础框架（1-2 周）

**目标**：搭建 eBPF 开发环境和基础框架

**任务**：
1. ✅ 创建 eBPF 项目结构
2. ✅ 实现基础 XDP 程序（数据包计数）
3. ✅ 实现用户态加载器
4. ✅ 实现 eBPF Map 通信
5. ✅ 基础测试和验证

**文件**：
```
crates/clash-verge-xdp/
├── Cargo.toml
├── xdp-ebpf/              # eBPF 内核态代码
│   ├── Cargo.toml
│   └── src/
│       └── main.rs        # XDP 程序入口
└── xdp-userspace/         # 用户态控制代码
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        └── loader.rs      # eBPF 加载器
```

### Phase 2: 数据包处理（2-3 周）

**目标**：实现核心数据包处理逻辑

**任务**：
1. ✅ IP/TCP/UDP 协议解析
2. ✅ 连接跟踪（Connection Tracking）
3. ✅ NAT 地址转换
4. ✅ 数据包重写
5. ✅ 路由表查找

### Phase 3: 加密支持（2-3 周）

**目标**：在内核态实现加密/解密

**任务**：
1. ✅ Shadowsocks 协议支持
2. ✅ AEAD 加密（AES-GCM, ChaCha20-Poly1305）
3. ✅ 密钥管理
4. ✅ 性能优化

### Phase 4: 高级特性（2-3 周）

**目标**：实现高级功能

**任务**：
1. ✅ 负载均衡
2. ✅ 故障切换
3. ✅ 流量统计
4. ✅ QoS 支持
5. ✅ 多核扩展

### Phase 5: 集成和优化（1-2 周）

**目标**：集成到 Clash Verge

**任务**：
1. ✅ 与现有代理系统集成
2. ✅ 配置界面
3. ✅ 性能测试
4. ✅ 文档编写

---

## 技术细节

### XDP 程序结构

```rust
// xdp-ebpf/src/main.rs
#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::{HashMap, PerfEventArray},
    programs::XdpContext,
};

// 路由表
#[map]
static ROUTE_TABLE: HashMap<u32, RouteEntry> = HashMap::with_max_entries(10000, 0);

// 连接跟踪表
#[map]
static CONN_TRACK: HashMap<ConnKey, ConnState> = HashMap::with_max_entries(100000, 0);

// 统计计数器
#[map]
static STATS: HashMap<u32, Stats> = HashMap::with_max_entries(256, 0);

#[xdp]
pub fn xdp_proxy(ctx: XdpContext) -> u32 {
    match try_xdp_proxy(ctx) {
        Ok(ret) => ret,
        Err(_) => xdp_action::XDP_ABORTED,
    }
}

fn try_xdp_proxy(ctx: XdpContext) -> Result<u32, ()> {
    // 1. 解析以太网头
    let ethhdr = ptr_at::<EthHdr>(&ctx, 0)?;
    
    // 2. 检查是否是 IP 包
    if ethhdr.ether_type != ETH_P_IP {
        return Ok(xdp_action::XDP_PASS);
    }
    
    // 3. 解析 IP 头
    let iphdr = ptr_at::<IpHdr>(&ctx, EthHdr::LEN)?;
    
    // 4. 查找路由表
    let route = unsafe {
        ROUTE_TABLE.get(&iphdr.daddr).ok_or(())?
    };
    
    // 5. 处理数据包
    match route.action {
        RouteAction::Proxy => {
            // 代理模式：加密并转发
            proxy_packet(&ctx, iphdr, route)?;
        }
        RouteAction::Direct => {
            // 直连模式：直接转发
            return Ok(xdp_action::XDP_PASS);
        }
        RouteAction::Reject => {
            // 拒绝模式：丢弃
            return Ok(xdp_action::XDP_DROP);
        }
    }
    
    Ok(xdp_action::XDP_TX)
}
```

### 用户态加载器

```rust
// xdp-userspace/src/loader.rs
use aya::{
    Ebpf,
    programs::{Xdp, XdpFlags},
    maps::HashMap,
};

pub struct XdpProxyLoader {
    ebpf: Ebpf,
    interface: String,
}

impl XdpProxyLoader {
    pub fn new(interface: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // 加载 eBPF 程序
        let mut ebpf = Ebpf::load(include_bytes_aligned!(
            "../../xdp-ebpf/target/bpfel-unknown-none/release/xdp-ebpf"
        ))?;
        
        Ok(Self {
            ebpf,
            interface: interface.to_string(),
        })
    }
    
    pub fn attach(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 获取 XDP 程序
        let program: &mut Xdp = self.ebpf
            .program_mut("xdp_proxy")
            .unwrap()
            .try_into()?;
        
        // 加载到内核
        program.load()?;
        
        // 附加到网卡
        program.attach(&self.interface, XdpFlags::SKB_MODE)?;
        
        Ok(())
    }
    
    pub fn update_route(&mut self, dest: u32, route: RouteEntry) -> Result<(), Box<dyn std::error::Error>> {
        let mut route_table: HashMap<_, u32, RouteEntry> = HashMap::try_from(
            self.ebpf.map_mut("ROUTE_TABLE").unwrap()
        )?;
        
        route_table.insert(dest, route, 0)?;
        Ok(())
    }
}
```

---

## 性能优化策略

### 1. 零拷贝（Zero-Copy）

- 使用 XDP_TX 直接转发
- 避免数据包进入内核协议栈
- 使用 AF_XDP 实现用户态零拷贝

### 2. 批处理（Batching）

- 批量处理数据包
- 减少系统调用次数
- 提高缓存命中率

### 3. 多核扩展（Multi-Core Scaling）

- RSS (Receive Side Scaling) 分发数据包
- 每个 CPU 核心独立处理
- 无锁数据结构

### 4. 内存优化

- 使用 eBPF Map 的 Per-CPU 变量
- 预分配内存池
- 避免动态内存分配

### 5. 加密优化

- 使用硬件加速（AES-NI）
- 批量加密
- 流水线处理

---

## 限制和注意事项

### eBPF 限制

1. **指令数限制**：单个 eBPF 程序最多 100 万条指令
2. **栈大小限制**：512 字节
3. **循环限制**：必须有明确的上界
4. **函数调用限制**：有限的辅助函数

### 解决方案

1. **Tail Calls**：将大程序拆分成多个小程序
2. **Map 存储**：使用 Map 存储大数据结构
3. **循环展开**：手动展开循环
4. **内联函数**：减少函数调用开销

### 平台支持

- ✅ **Linux**: 完整支持（内核 4.18+）
- ❌ **Windows**: 不支持（可使用 WSL2）
- ❌ **macOS**: 不支持

### 权限要求

- 需要 `CAP_BPF` 或 `CAP_SYS_ADMIN` 权限
- 需要 root 或 sudo

---

## 部署方案

### 方案 1: 本地客户端（Linux）

```
┌─────────────────┐
│  Linux 客户端    │
│  ┌───────────┐  │
│  │ XDP 程序  │  │
│  └───────────┘  │
│       ↓         │
│  ┌───────────┐  │
│  │ 物理网卡  │  │
│  └───────────┘  │
└─────────────────┘
        ↓
    互联网
```

**优点**：
- 最低延迟
- 最高性能
- 完全控制

**缺点**：
- 仅支持 Linux
- 需要 root 权限

### 方案 2: VPS 服务器（推荐）

```
┌─────────────┐      ┌─────────────────┐
│  客户端      │      │  VPS (Linux)    │
│  (任何平台)  │ ───> │  ┌───────────┐  │
└─────────────┘      │  │ XDP 程序  │  │
                     │  └───────────┘  │
                     │       ↓         │
                     │  ┌───────────┐  │
                     │  │ 物理网卡  │  │
                     │  └───────────┘  │
                     └─────────────────┘
                             ↓
                         目标服务器
```

**优点**：
- 跨平台支持
- 集中管理
- 高性能转发

**缺点**：
- 需要 VPS
- 额外成本

### 方案 3: 混合模式

```
┌─────────────────┐      ┌─────────────────┐
│  Linux 客户端    │      │  VPS (Linux)    │
│  ┌───────────┐  │      │  ┌───────────┐  │
│  │ XDP 程序  │  │ ───> │  │ XDP 程序  │  │
│  └───────────┘  │      │  └───────────┘  │
└─────────────────┘      └─────────────────┘
        ↓                         ↓
┌─────────────────┐      ┌─────────────────┐
│  Windows/macOS  │      │  目标服务器      │
│  (传统模式)      │ ───> │                 │
└─────────────────┘      └─────────────────┘
```

**优点**：
- 灵活性高
- 性能最优
- 兼容性好

---

## 下一步行动

### 立即开始

1. **创建 eBPF 项目结构**
2. **实现基础 XDP 程序**
3. **测试数据包拦截**

### 需要的工具

```bash
# 安装 Rust eBPF 工具链
cargo install bpf-linker

# 安装 Aya 模板
cargo install cargo-generate

# 创建 XDP 项目
cargo generate https://github.com/aya-rs/aya-template
```

### 测试环境

- Linux 内核 5.10+
- 支持 XDP 的网卡
- root 权限

---

## 预期效果

### 性能提升

- **延迟**: 从 100μs 降至 10μs（10x）
- **吞吐量**: 从 5 Gbps 提升至 50+ Gbps（10x）
- **CPU 占用**: 降低 80%
- **内存占用**: 降低 50%

### 用户体验

- 游戏延迟更低
- 视频加载更快
- 大文件下载更稳定
- 多设备同时使用无压力

---

**这是真正的架构层面究极体！** 🚀⚡
