import { useState, useEffect } from 'react';

import { Button } from '@/components/tailwind/Button';
import { Card } from '@/components/tailwind/Card';
import { Switch } from '@/components/tailwind/Switch';
import {
  ipReputationGetConfig,
  ipReputationUpdateConfig,
  ipReputationGetPredefinedRules,
  ipReputationClearCache,
  ipReputationGetCacheStats,
  type IpReputationConfig,
  type RiskRoutingRule,
} from '@/services/ip-reputation';
import { showNotice } from '@/services/notice-service';

/**
 * IP 信誉度配置组件
 */
export function IpReputationConfig() {
  const [config, setConfig] = useState<IpReputationConfig | null>(null);
  const [predefinedRules, setPredefinedRules] = useState<RiskRoutingRule[]>([]);
  const [cacheStats, setCacheStats] = useState<[number, number]>([0, 0]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    loadConfig();
    loadPredefinedRules();
    loadCacheStats();

    // 每 10 秒更新缓存统计
    const interval = setInterval(loadCacheStats, 10000);
    return () => clearInterval(interval);
  }, []);

  const loadConfig = async () => {
    try {
      const cfg = await ipReputationGetConfig();
      setConfig(cfg);
    } catch (error) {
      showNotice.error(`加载配置失败: ${error}`);
    }
  };

  const loadPredefinedRules = async () => {
    try {
      const rules = await ipReputationGetPredefinedRules();
      setPredefinedRules(rules);
    } catch (error) {
      showNotice.error(`加载预定义规则失败: ${error}`);
    }
  };

  const loadCacheStats = async () => {
    try {
      const stats = await ipReputationGetCacheStats();
      setCacheStats(stats);
    } catch (error) {
      console.error('加载缓存统计失败:', error);
    }
  };

  const handleSave = async () => {
    if (!config) return;

    setLoading(true);
    try {
      await ipReputationUpdateConfig(config);
      showNotice.success('配置已保存');
    } catch (error) {
      showNotice.error(`保存失败: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleToggleEnabled = (checked: boolean) => {
    if (!config) return;
    setConfig({ ...config, enabled: checked });
  };

  const handleToggleRule = (index: number, checked: boolean) => {
    if (!config) return;
    const newRules = [...config.routingRules];
    newRules[index] = { ...newRules[index], enabled: checked };
    setConfig({ ...config, routingRules: newRules });
  };

  const handleLoadPredefinedRules = () => {
    if (!config) return;
    setConfig({ ...config, routingRules: predefinedRules });
    showNotice.success('已加载预定义规则');
  };

  const handleClearCache = async () => {
    try {
      await ipReputationClearCache();
      showNotice.success('缓存已清除');
      loadCacheStats();
    } catch (error) {
      showNotice.error(`清除缓存失败: ${error}`);
    }
  };

  if (!config) {
    return <div className="p-4">加载中...</div>;
  }

  const [totalCache, expiredCache] = cacheStats;

  return (
    <div className="space-y-4">
      {/* 主开关 */}
      <Card>
        <div className="p-4">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-lg font-semibold">IP 信誉度检测</h3>
              <p className="text-sm text-gray-500 mt-1">
                根据 IP 信誉度选择合适的节点，避免使用高风险 IP
              </p>
            </div>
            <Switch checked={config.enabled} onCheckedChange={handleToggleEnabled} />
          </div>
        </div>
      </Card>

      {/* 缓存统计 */}
      <Card>
        <div className="p-4">
          <div className="flex items-center justify-between mb-3">
            <h4 className="font-semibold">缓存统计</h4>
            <Button size="sm" variant="outline" onClick={handleClearCache}>
              清除缓存
            </Button>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="bg-gray-50 p-3 rounded-lg">
              <div className="text-2xl font-bold text-blue-600">{totalCache}</div>
              <div className="text-sm text-gray-600">缓存条目</div>
            </div>
            <div className="bg-gray-50 p-3 rounded-lg">
              <div className="text-2xl font-bold text-orange-600">{expiredCache}</div>
              <div className="text-sm text-gray-600">已过期</div>
            </div>
          </div>
        </div>
      </Card>

      {/* 风控路由规则 */}
      <Card>
        <div className="p-4">
          <div className="flex items-center justify-between mb-4">
            <h4 className="font-semibold">风控路由规则</h4>
            <Button
              size="sm"
              variant="outline"
              onClick={handleLoadPredefinedRules}
            >
              加载预定义规则
            </Button>
          </div>

          <div className="space-y-3">
            {config.routingRules.map((rule, index) => (
              <RiskRoutingRuleItem
                key={`${rule.domainPatterns.join('|')}::${rule.maxFraudScore}::${rule.fallbackPolicy}`}
                rule={rule}
                enabled={rule.enabled}
                onToggle={(checked) => handleToggleRule(index, checked)}
              />
            ))}

            {config.routingRules.length === 0 && (
              <div className="text-center py-8 text-gray-500">
                暂无规则，点击"加载预定义规则"添加
              </div>
            )}
          </div>
        </div>
      </Card>

      {/* 保存按钮 */}
      <div className="flex justify-end">
        <Button onClick={handleSave} loading={loading} disabled={loading}>
          保存配置
        </Button>
      </div>
    </div>
  );
}

/**
 * 风控路由规则项组件
 */
function RiskRoutingRuleItem({
  rule,
  enabled,
  onToggle,
}: {
  rule: RiskRoutingRule;
  enabled: boolean;
  onToggle: (checked: boolean) => void;
}) {
  const getIpTypeText = (ipType?: string) => {
    switch (ipType) {
      case 'Datacenter':
        return '机房 IP';
      case 'Residential':
        return '住宅 IP';
      case 'Mobile':
        return '移动 IP';
      default:
        return '任意';
    }
  };

  const getFallbackPolicyText = (policy: string) => {
    switch (policy) {
      case 'Block':
        return '阻止连接';
      case 'Warn':
        return '警告但允许';
      case 'Allow':
        return '允许';
      default:
        return policy;
    }
  };

  const getFallbackPolicyColor = (policy: string) => {
    switch (policy) {
      case 'Block':
        return 'text-red-600';
      case 'Warn':
        return 'text-yellow-600';
      case 'Allow':
        return 'text-green-600';
      default:
        return 'text-gray-600';
    }
  };

  const getRiskScoreColor = (score: number) => {
    if (score <= 30) return 'text-green-600';
    if (score <= 60) return 'text-yellow-600';
    if (score <= 85) return 'text-orange-600';
    return 'text-red-600';
  };

  return (
    <div className="flex items-start gap-3 p-3 border rounded-lg hover:bg-gray-50">
      <Switch checked={enabled} onCheckedChange={onToggle} />

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          {rule.domainPatterns.map((pattern) => (
            <code
              key={pattern}
              className="text-sm font-mono bg-gray-100 px-2 py-1 rounded"
            >
              {pattern}
            </code>
          ))}
        </div>

        <p className="text-sm text-gray-600 mt-2">{rule.description}</p>

        <div className="flex items-center gap-4 mt-2 text-xs">
          <span className="text-gray-500">
            IP 类型: <span className="font-medium">{getIpTypeText(rule.requiredIpType)}</span>
          </span>
          <span className="text-gray-500">
            最大评分:{' '}
            <span className={`font-medium ${getRiskScoreColor(rule.maxFraudScore)}`}>
              {rule.maxFraudScore}
            </span>
          </span>
          <span className="text-gray-500">
            故障转移:{' '}
            <span className={`font-medium ${getFallbackPolicyColor(rule.fallbackPolicy)}`}>
              {getFallbackPolicyText(rule.fallbackPolicy)}
            </span>
          </span>
        </div>
      </div>
    </div>
  );
}
