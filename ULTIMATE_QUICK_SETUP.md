# 究极功能快速配置指南

## 5 分钟快速部署

### 步骤 1: 生成加密密钥

```bash
# 在应用中生成密钥
# 安全配置 → 内生欺骗陷阱 → 生成新密钥

# 设置环境变量（Windows PowerShell）
$env:CLASH_VERGE_SECURE_KEY="your-generated-key"

# 设置环境变量（Linux/macOS）
export CLASH_VERGE_SECURE_KEY="your-generated-key"
```

### 步骤 2: 启动安全监控

```
安全配置 → 内生欺骗陷阱 → 启用安全监控
```

系统将自动开始：
- ✅ 反调试检测
- ✅ 内存蜜罐监控
- ✅ 可疑工具检测

### 步骤 3: 配置反主动探测

```
安全配置 → 反主动探测
```

1. 启用反探测
2. 生成新密钥（自动）
3. 设置时间窗口：300 秒
4. 添加白名单 IP（可选）
5. 保存配置

### 步骤 4: 选择 TLS 指纹

```
安全配置 → TLS 指纹伪装
```

推荐选择：
- 🌐 **Chrome 120** - 日常使用
- 🦊 **Firefox 121** - 注重隐私
- 🎮 **Genshin Impact** - 伪装成游戏

### 步骤 5: 配置多路径路由

```
多路径路由 → 基础配置
```

1. 启用多路径路由
2. 选择策略：**加权（推荐）**
3. 启用会话保持
4. 保存配置

### 步骤 6: 添加节点池

```
多路径路由 → 节点池管理 → 添加节点池
```

**推荐配置**：

#### 流媒体专用池
```yaml
名称: 流媒体专用
类型: Streaming
节点:
  - 名称: HK-Stream-1
    服务器: hk1.example.com
    端口: 443
    协议: vmess
    权重: 100
```

#### 游戏专用池
```yaml
名称: 游戏专用
类型: Gaming
节点:
  - 名称: JP-Game-1
    服务器: jp1.example.com
    端口: 443
    协议: trojan
    权重: 100
```

#### 下载专用池
```yaml
名称: 下载专用
类型: Download
节点:
  - 名称: US-Download-1
    服务器: us1.example.com
    端口: 443
    权重: 50
  - 名称: US-Download-2
    服务器: us2.example.com
    端口: 443
    权重: 50
```

---

## XDP 代理配置（Linux 专用）

### 前置要求

```bash
# 检查内核版本（需要 5.10+）
uname -r

# 安装工具链
cargo install bpf-linker
rustup target add bpfel-unknown-none
```

### 编译和启动

```bash
# 编译
cd crates/clash-verge-xdp
./build.sh

# 启动（需要 root）
sudo ./xdp-userspace/target/release/xdp-proxy \
  --interface eth0 \
  start
```

### 添加路由规则

```bash
# 直连
sudo ./xdp-proxy add-route 8.8.8.8 pass

# 代理
sudo ./xdp-proxy add-route 1.1.1.1 proxy \
  --proxy-ip 10.0.0.1 \
  --proxy-port 1080

# 拒绝
sudo ./xdp-proxy add-route 192.168.1.1 reject
```

---

## 推荐配置方案

### 方案 1: 日常使用（平衡）

```yaml
安全监控: ✅ 启用
反探测: ✅ 启用
  时间窗口: 300 秒
  严格模式: ❌ 关闭
TLS 指纹: Chrome 120
多路径: ✅ 启用
  策略: 加权
  会话保持: ✅ 启用
XDP 代理: ❌ 关闭（非 Linux）
```

### 方案 2: 高安全（严格）

```yaml
安全监控: ✅ 启用
反探测: ✅ 启用
  时间窗口: 60 秒
  严格模式: ✅ 启用
  白名单: 添加客户端 IP
TLS 指纹: Safari
多路径: ✅ 启用
  策略: 延迟优先
  会话保持: ✅ 启用
XDP 代理: ✅ 启用（Linux）
配置欺骗: ✅ 部署假配置
```

### 方案 3: 极致性能（Linux）

```yaml
安全监控: ✅ 启用
反探测: ✅ 启用
TLS 指纹: Chrome 120
多路径: ✅ 启用
  策略: 最少连接
XDP 代理: ✅ 启用
  模式: Native
  接口: eth0
```

---

## 重要安全规则

### ⚠️ 必须单节点的服务

| 服务 | 域名模式 | 原因 |
|------|---------|------|
| Netflix | `*.netflix.com` | IP 变化会被封号 |
| YouTube | `*.youtube.com` | IP 变化会被封号 |
| Hulu | `*.hulu.com` | IP 变化会被封号 |
| Disney+ | `*.disneyplus.com` | IP 变化会被封号 |
| Steam | `*.steampowered.com` | 避免延迟波动 |
| Epic Games | `*.epicgames.com` | 避免延迟波动 |
| Twitter | `*.twitter.com` | 建议单节点 |
| Facebook | `*.facebook.com` | 建议单节点 |

### ✅ 可以多路径的服务

| 服务 | 域名模式 | 优势 |
|------|---------|------|
| GitHub | `*.github.com` | 提高下载速度 |
| 通用下载 | - | 提高下载速度 |

---

## 故障排除

### 问题 1: 安全监控无法启动

**解决**：
1. 检查权限
2. 查看日志
3. 重启应用

### 问题 2: XDP 代理无法加载

**解决**：
```bash
# 检查内核版本
uname -r  # 需要 5.10+

# 检查权限
sudo -v

# 使用 SKB 模式（兼容性好）
sudo ./xdp-proxy --interface eth0 start
```

### 问题 3: 多路径导致封号

**解决**：
1. 检查会话绑定规则
2. 确认流媒体服务使用单节点
3. 查看预定义规则是否生效

### 问题 4: 假配置被频繁访问

**解决**：
1. 检查是否有扫描软件
2. 检查杀毒软件设置
3. 更换假配置路径

---

## 性能监控

### 查看 XDP 统计

```bash
sudo ./xdp-proxy stats
```

输出：
```
Statistics:
  Total packets:    1000000
  Proxied packets:  500000
  Direct packets:   450000
  Rejected packets: 50000
  Errors:           0
```

### 查看安全状态

```
安全配置 → 内生欺骗陷阱
```

查看：
- 🟢 安全状态
- 🟢 调试器检测
- 🟢 内存扫描检测

---

## 测试验证

### 测试反调试

```bash
# Windows
x64dbg.exe clash-verge.exe

# Linux
gdb -p $(pgrep clash-verge)
```

**预期**：程序检测到调试器并触发自毁

### 测试配置欺骗

1. 部署假配置
2. 用编辑器打开假配置
3. 点击"检查访问"

**预期**：显示"假配置文件被访问"

### 测试多路径

1. 配置多个节点
2. 访问 GitHub
3. 查看连接分布

**预期**：流量分散到多个节点

---

## 最佳实践

### ✅ 推荐

1. 定期更换加密密钥（每月）
2. 定期更换 TLS 指纹（每周）
3. 监控假配置访问（每天）
4. 使用白名单（高风险环境）
5. 启用会话保持（避免封号）

### ❌ 避免

1. 不要在调试模式下运行
2. 不要同时运行扫描工具
3. 不要分享握手暗号
4. 不要禁用安全监控
5. 不要忽略安全警告
6. 不要对流媒体使用多路径

---

## 获取帮助

如果遇到问题：

1. 查看日志文件
2. 检查安全状态
3. 阅读完整文档：`ULTIMATE_FEATURES_COMPLETE.md`
4. 提交 Issue

---

**祝你使用愉快！** 🛡️🚀⚡🌐
