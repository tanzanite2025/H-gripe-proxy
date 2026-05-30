import { useState, useEffect } from 'react';

import { Button } from '@/components/tailwind/Button';
import { Card } from '@/components/tailwind/Card';
import { Switch } from '@/components/tailwind/Switch';
import { showNotice } from '@/services/notice-service';
import {
  sessionAffinityGetPredefinedRules,
  type SessionAffinityConfig as SessionAffinityConfigModel,
  type DomainBindingRule,
} from '@/services/session-affinity';

interface Props {
  config: SessionAffinityConfigModel;
  onChange: (config: SessionAffinityConfigModel) => void;
}

/**
 * 会话绑定配置组件
 */
export function SessionAffinityConfig({
  config,
  onChange,
}: Props) {
  const [predefinedRules, setPredefinedRules] = useState<DomainBindingRule[]>([]);

  // 加载预定义规则
  useEffect(() => {
    loadPredefinedRules();
  }, []);

  const loadPredefinedRules = async () => {
    try {
      const rules = await sessionAffinityGetPredefinedRules();
      setPredefinedRules(rules);
    } catch (error) {
      showNotice('error', `加载预定义规则失败: ${error}`);
    }
  };

  const updateConfig = (nextConfig: SessionAffinityConfigModel) => {
    onChange(nextConfig);
  };

  const handleToggleEnabled = (checked: boolean) => {
    updateConfig({ ...config, enabled: checked });
  };

  const handleToggleRule = (index: number, checked: boolean) => {
    const newRules = [...config.domainRules];
    newRules[index] = { ...newRules[index], enabled: checked };
    updateConfig({ ...config, domainRules: newRules });
  };

  const handleLoadPredefinedRules = () => {
    updateConfig({ ...config, domainRules: predefinedRules });
    showNotice('success', '已加载预定义规则');
  };

  return (
    <div className="space-y-4">
      {/* 主开关 */}
      <Card>
        <div className="p-4">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-lg font-semibold">会话绑定</h3>
              <p className="text-sm text-gray-500 mt-1">
                防止 IP 频繁跳动导致账号被封禁
              </p>
            </div>
            <Switch
              checked={config.enabled}
              onCheckedChange={handleToggleEnabled}
            />
          </div>
        </div>
      </Card>

      {/* 域名绑定规则 */}
      <Card>
        <div className="p-4">
          <div className="flex items-center justify-between mb-4">
            <h4 className="font-semibold">域名绑定规则</h4>
            <Button
              size="sm"
              variant="outline"
              onClick={handleLoadPredefinedRules}
            >
              加载预定义规则
            </Button>
          </div>

          <div className="space-y-3">
            {config.domainRules.map((rule, index) => (
              <DomainRuleItem
                key={rule.domainPattern}
                rule={rule}
                enabled={rule.enabled}
                onToggle={(checked) => handleToggleRule(index, checked)}
              />
            ))}

            {config.domainRules.length === 0 && (
              <div className="text-center py-8 text-gray-500">
                暂无规则，点击"加载预定义规则"添加
              </div>
            )}
          </div>
        </div>
      </Card>
    </div>
  );
}

/**
 * 域名规则项组件
 */
function DomainRuleItem({
  rule,
  enabled,
  onToggle,
}: {
  rule: DomainBindingRule;
  enabled: boolean;
  onToggle: (checked: boolean) => void;
}) {
  const getFallbackPolicyText = (policy: string) => {
    switch (policy) {
      case 'Manual':
        return '手动确认';
      case 'AutoRetry':
        return '自动重试';
      case 'AutoSwitch':
        return '自动切换';
      default:
        return policy;
    }
  };

  const getTtlText = (ttl: number) => {
    if (ttl === 0) return '永久';
    if (ttl < 3600) return `${ttl / 60} 分钟`;
    if (ttl < 86400) return `${ttl / 3600} 小时`;
    return `${ttl / 86400} 天`;
  };

  return (
    <div className="flex items-start gap-3 p-3 border rounded-lg hover:bg-gray-50">
      <Switch checked={enabled} onCheckedChange={onToggle} />
      
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <code className="text-sm font-mono bg-gray-100 px-2 py-1 rounded">
            {rule.domainPattern}
          </code>
          <span className="text-xs text-gray-500">
            {getTtlText(rule.ttl)}
          </span>
          <span className="text-xs text-gray-500">
            {getFallbackPolicyText(rule.fallbackPolicy)}
          </span>
        </div>
        <p className="text-sm text-gray-600 mt-1">{rule.description}</p>
      </div>
    </div>
  );
}
