# DNS Config 组件重构计划

## 📊 当前状态

**文件：** `src/components/setting/components/clash/dns-config.tsx`  
**大小：** 1111 行  
**复杂度：** 高  
**优先级：** 中

---

## 🎯 重构目标

将 1111 行的大型 DNS 配置组件拆分为职责清晰、易于维护的模块化结构。

**目标：**
- 主组件 < 200 行
- 每个子组件 < 200 行
- 每个 hook < 150 行
- 工具函数 < 50 行/函数

---

## 📁 拆分方案

```
dns-config/
├── index.tsx                      # 主组件 (~180行)
├── components/
│   ├── dns-general-fields.tsx     # 通用字段 (~200行)
│   ├── dns-nameserver-fields.tsx  # 域名服务器字段 (~200行)
│   ├── dns-fallback-fields.tsx    # 回退过滤字段 (~150行)
│   └── dns-hosts-fields.tsx       # Hosts 字段 (~50行)
├── hooks/
│   ├── use-dns-config.ts          # 配置管理 (~150行)
│   └── use-dns-form.ts            # 表单管理 (~100行)
└── utils/
    └── dns-helpers.ts             # 工具函数 (~150行)
```

---

## 🔧 详细拆分计划

### 1. 工具函数 (`utils/dns-helpers.ts`)

**提取的函数：**
```typescript
// 解析和格式化
export const parseNameserverPolicy(str: string): NameserverPolicy
export const formatNameserverPolicy(policy: unknown): string
export const formatHosts(hosts: unknown): string
export const parseHosts(str: string): NameserverPolicy
export const parseList(str: string): string[]

// 默认配置
export const DEFAULT_DNS_CONFIG = { ... }

// 配置生成
export const generateDnsConfig(values: DnsFormValues): any
export const generateHostsConfig(hostsStr: string): any
```

**收益：**
- 纯函数，易于测试
- 可复用
- 逻辑清晰

---

### 2. 配置管理 Hook (`hooks/use-dns-config.ts`)

**职责：**
- 加载 DNS 配置
- 保存 DNS 配置
- 验证 DNS 配置
- 应用 DNS 配置

**核心代码：**
```typescript
export const useDnsConfig = () => {
  const { clash, mutateClash } = useClash()

  const loadConfig = async () => {
    const exists = await invoke('check_dns_config_exists')
    if (exists) {
      const content = await invoke('get_dns_config_content')
      return yaml.load(content)
    }
    return null
  }

  const saveConfig = async (config: any) => {
    await invoke('save_dns_config', { dnsConfig: config })
    
    // 验证配置
    const validation = await invoke('validate_dns_config')
    if (validation.status !== 'valid') {
      throw new Error(validation.message)
    }

    // 如果DNS开关打开，应用配置
    if (clash?.dns?.enable) {
      await invoke('apply_dns_config', { apply: true })
      mutateClash()
    }
  }

  return { loadConfig, saveConfig }
}
```

---

### 3. 表单管理 Hook (`hooks/use-dns-form.ts`)

**职责：**
- 管理表单状态
- 表单值与配置对象互转
- YAML 与表单值互转

**核心代码：**
```typescript
export const useDnsForm = () => {
  const [values, setValues] = useState<DnsFormValues>(DEFAULT_VALUES)
  const [yamlContent, setYamlContent] = useState('')
  const [visualization, setVisualization] = useState(true)

  // 从配置对象更新表单值
  const updateValuesFromConfig = (config: any) => {
    setValues({
      enable: config.dns?.enable ?? DEFAULT_DNS_CONFIG.enable,
      listen: config.dns?.listen ?? DEFAULT_DNS_CONFIG.listen,
      // ... 其他字段
    })
  }

  // 从表单值生成配置对象
  const generateConfigFromValues = () => {
    return {
      dns: generateDnsConfig(values),
      hosts: generateHostsConfig(values.hosts),
    }
  }

  // 从 YAML 更新表单值
  const updateValuesFromYaml = () => {
    const config = yaml.load(yamlContent)
    updateValuesFromConfig(config)
  }

  // 从表单值更新 YAML
  const updateYamlFromValues = () => {
    const config = generateConfigFromValues()
    setYamlContent(yaml.dump(config))
  }

  return {
    values,
    setValues,
    yamlContent,
    setYamlContent,
    visualization,
    setVisualization,
    updateValuesFromConfig,
    generateConfigFromValues,
    updateValuesFromYaml,
    updateYamlFromValues,
  }
}
```

---

### 4. 通用字段组件 (`components/dns-general-fields.tsx`)

**包含字段：**
- enable (开关)
- listen (监听地址)
- enhanced-mode (增强模式)
- fake-ip-range (Fake IP 范围)
- fake-ip-filter-mode (Fake IP 过滤模式)
- ipv6 (IPv6 支持)
- prefer-h3 (优先 HTTP/3)
- respect-rules (遵守规则)
- use-hosts (使用 Hosts)
- use-system-hosts (使用系统 Hosts)
- direct-nameserver-follow-policy (Direct 域名服务器遵循策略)

**核心代码：**
```typescript
export const DnsGeneralFields = ({ values, onChange }) => {
  return (
    <>
      <Typography variant="subtitle1">通用设置</Typography>
      
      <Item>
        <ListItemText primary="启用 DNS" />
        <Switch checked={values.enable} onChange={onChange('enable')} />
      </Item>

      <Item>
        <ListItemText primary="监听地址" />
        <TextField
          value={values.listen}
          onChange={onChange('listen')}
          placeholder=":53"
        />
      </Item>

      {/* ... 其他字段 */}
    </>
  )
}
```

---

### 5. 域名服务器字段组件 (`components/dns-nameserver-fields.tsx`)

**包含字段：**
- default-nameserver (默认域名服务器)
- nameserver (域名服务器)
- fallback (回退域名服务器)
- proxy-server-nameserver (代理服务器域名服务器)
- direct-nameserver (直连域名服务器)
- fake-ip-filter (Fake IP 过滤)
- nameserver-policy (域名服务器策略)

**核心代码：**
```typescript
export const DnsNameserverFields = ({ values, onChange }) => {
  return (
    <>
      <Item>
        <ListItemText
          primary="默认域名服务器"
          secondary="用于解析 DNS 服务器的域名"
        />
        <TextField
          fullWidth
          multiline
          value={values.defaultNameserver}
          onChange={onChange('defaultNameserver')}
          placeholder="system, 223.6.6.6, 8.8.8.8"
        />
      </Item>

      {/* ... 其他字段 */}
    </>
  )
}
```

---

### 6. 回退过滤字段组件 (`components/dns-fallback-fields.tsx`)

**包含字段：**
- fallback-geoip (GeoIP 过滤)
- fallback-geoip-code (GeoIP 代码)
- fallback-ipcidr (IP CIDR 过滤)
- fallback-domain (域名过滤)

**核心代码：**
```typescript
export const DnsFallbackFields = ({ values, onChange }) => {
  return (
    <>
      <Typography variant="subtitle2">回退过滤</Typography>

      <Item>
        <ListItemText
          primary="GeoIP 过滤"
          secondary="根据 GeoIP 过滤回退结果"
        />
        <Switch
          checked={values.fallbackGeoip}
          onChange={onChange('fallbackGeoip')}
        />
      </Item>

      {/* ... 其他字段 */}
    </>
  )
}
```

---

### 7. Hosts 字段组件 (`components/dns-hosts-fields.tsx`)

**包含字段：**
- hosts (Hosts 映射)

**核心代码：**
```typescript
export const DnsHostsFields = ({ values, onChange }) => {
  return (
    <>
      <Typography variant="subtitle1">Hosts 配置</Typography>

      <Item>
        <ListItemText
          primary="Hosts 映射"
          secondary="自定义域名到 IP 的映射"
        />
        <TextField
          fullWidth
          multiline
          value={values.hosts}
          onChange={onChange('hosts')}
          placeholder="*.clash.dev=127.0.0.1, test.com=1.1.1.1"
        />
      </Item>
    </>
  )
}
```

---

### 8. 主组件 (`index.tsx`)

**职责：**
- 组合所有子组件和 hooks
- 处理对话框打开/关闭
- 处理保存操作
- 切换可视化/YAML 模式

**核心代码：**
```typescript
export function DnsViewer({ ref }: { ref?: Ref<DialogRef> }) {
  const { t } = useTranslation()
  const themeMode = useThemeMode()
  const [open, setOpen] = useState(false)

  // 配置管理
  const { loadConfig, saveConfig } = useDnsConfig()

  // 表单管理
  const {
    values,
    setValues,
    yamlContent,
    setYamlContent,
    visualization,
    setVisualization,
    updateValuesFromConfig,
    generateConfigFromValues,
    updateValuesFromYaml,
    updateYamlFromValues,
  } = useDnsForm()

  // 初始化
  const initDnsConfig = async () => {
    const config = await loadConfig()
    if (config) {
      updateValuesFromConfig(config)
      setYamlContent(yaml.dump(config))
    }
  }

  // 保存
  const onSave = async () => {
    const config = visualization
      ? generateConfigFromValues()
      : yaml.load(yamlContent)

    await saveConfig(config)
    setOpen(false)
    showNotice.success('保存成功')
  }

  // 处理字段变化
  const handleChange = (field: string) => (event: any) => {
    const value = event.target.type === 'checkbox'
      ? event.target.checked
      : event.target.value

    setValues(prev => ({ ...prev, [field]: value }))
  }

  return (
    <BaseDialog
      open={open}
      title="DNS 配置"
      onClose={() => setOpen(false)}
      onOk={onSave}
    >
      {visualization ? (
        <List>
          <DnsGeneralFields values={values} onChange={handleChange} />
          <DnsNameserverFields values={values} onChange={handleChange} />
          <DnsFallbackFields values={values} onChange={handleChange} />
          <DnsHostsFields values={values} onChange={handleChange} />
        </List>
      ) : (
        <MonacoEditor
          value={yamlContent}
          onChange={setYamlContent}
        />
      )}
    </BaseDialog>
  )
}
```

---

## 📈 预期收益

### 代码质量

| 指标 | 重构前 | 重构后 | 改善 |
|------|--------|--------|------|
| 主组件行数 | 1111 | ~180 | ↓ 84% |
| 最大文件行数 | 1111 | ~200 | ↓ 82% |
| 平均文件行数 | 1111 | ~150 | ↓ 86% |
| 文件数量 | 1 | 8 | +700% |

### 可维护性

- ✅ 每个文件职责单一
- ✅ 易于理解和修改
- ✅ 易于测试
- ✅ 易于复用

---

## 🚀 实施步骤

### 步骤 1：提取工具函数（30分钟）
1. 创建 `utils/dns-helpers.ts`
2. 提取所有解析和格式化函数
3. 提取默认配置
4. 运行类型检查

### 步骤 2：提取配置管理 Hook（20分钟）
1. 创建 `hooks/use-dns-config.ts`
2. 提取加载、保存、验证逻辑
3. 运行类型检查

### 步骤 3：提取表单管理 Hook（30分钟）
1. 创建 `hooks/use-dns-form.ts`
2. 提取表单状态管理
3. 提取值转换逻辑
4. 运行类型检查

### 步骤 4：拆分字段组件（60分钟）
1. 创建 `components/dns-general-fields.tsx`
2. 创建 `components/dns-nameserver-fields.tsx`
3. 创建 `components/dns-fallback-fields.tsx`
4. 创建 `components/dns-hosts-fields.tsx`
5. 运行类型检查

### 步骤 5：重构主组件（30分钟）
1. 创建 `index.tsx`
2. 组合所有子组件和 hooks
3. 运行类型检查

### 步骤 6：测试验证（20分钟）
1. 运行 `pnpm run typecheck`
2. 运行 `pnpm run build`
3. 手动测试所有功能

**总计：** ~3 小时

---

## ⚠️ 注意事项

1. **保持功能不变** - 重构不改变任何功能
2. **逐步验证** - 每一步都运行类型检查
3. **备份原文件** - 使用 `.backup` 后缀
4. **更新导入** - 确保所有导入路径正确

---

## 📚 参考

- `COMPONENT_REFACTOR_GUIDE.md` - 组件重构指南
- `CURRENT_PROXY_CARD_REFACTOR_COMPLETE.md` - 类似重构案例
- `ENHANCED_CANVAS_TRAFFIC_GRAPH_REFACTOR_COMPLETE.md` - 类似重构案例
- `GROUPS_EDITOR_VIEWER_REFACTOR_COMPLETE.md` - 类似重构案例

---

**文档创建时间：** 2026-05-27  
**文档版本：** v1.0  
**状态：** 计划中  
**优先级：** 中
