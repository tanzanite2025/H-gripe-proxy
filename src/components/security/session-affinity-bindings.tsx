import { useState } from 'react';

import { Button } from '@/components/tailwind/Button';
import { Card } from '@/components/tailwind/Card';
import type { CoordinatorBindingInfo, CoordinatorStatus } from '@/services/coordinator';
import { showNotice } from '@/services/notice-service';
import {
  sessionAffinityClearBinding,
  sessionAffinityCleanupExpired,
} from '@/services/session-affinity';

/**
 * 会话绑定信息展示组件
 */
export function SessionAffinityBindings({
  status,
  onRefreshStatus,
}: {
  status: CoordinatorStatus;
  onRefreshStatus: () => Promise<CoordinatorStatus | null>;
}) {
  const [loading, setLoading] = useState(false);

  const bindings = status.runtimeState.sessionAffinityBindings;
  const domainRuleBindings = status.runtimeState.stableEgressBackwrite.domainRuleBindings;
  const regularBindings = bindings.filter((binding) => binding.bindingType !== 'domain-rule');

  const loadBindings = async () => {
    await onRefreshStatus();
  };

  const handleClearBinding = async (binding: CoordinatorBindingInfo) => {
    if (binding.bindingType !== 'domain' && binding.bindingType !== 'domain-rule') {
      showNotice('info', '目前只支持清除域名绑定和域名规则回写绑定');
      return;
    }

    try {
      await sessionAffinityClearBinding(binding.key);
      showNotice('success', '绑定已清除');
      loadBindings();
    } catch (error) {
      showNotice('error', `清除失败: ${error}`);
    }
  };

  const handleCleanupExpired = async () => {
    setLoading(true);
    try {
      await sessionAffinityCleanupExpired();
      showNotice('success', '已清理过期绑定');
      loadBindings();
    } catch (error) {
      showNotice('error', `清理失败: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card>
      <div className="p-4">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h4 className="font-semibold">当前绑定</h4>
            <p className="text-sm text-gray-500 mt-1">
              共 {bindings.length} 个活跃绑定，其中 domain-rule 回写 {domainRuleBindings.length} 个
            </p>
          </div>
          <Button
            size="sm"
            variant="outline"
            onClick={handleCleanupExpired}
            loading={loading}
          >
            清理过期
          </Button>
        </div>

        <div className="space-y-4">
          <div className="rounded-lg border border-purple-200 p-4 bg-purple-50/60 dark:bg-purple-950/20 dark:border-purple-800">
            <div className="mb-3">
              <h5 className="font-medium">稳定出口回写（domain-rule）</h5>
              <p className="text-sm text-gray-500 mt-1">
                这里展示稳定组手动选择回写到 `session_affinity` 后形成的域名规则级运行态绑定。
              </p>
            </div>

            <div className="space-y-2">
              {domainRuleBindings.length === 0 ? (
                <div className="text-center py-6 text-gray-500">
                  暂无 domain-rule 回写绑定
                </div>
              ) : (
                domainRuleBindings.map((binding) => (
                  <BindingItem
                    key={`${binding.bindingType}:${binding.key}`}
                    binding={binding}
                    onClear={() => handleClearBinding(binding)}
                  />
                ))
              )}
            </div>
          </div>

          <div className="rounded-lg border border-gray-200 p-4 dark:border-gray-700">
            <div className="mb-3">
              <h5 className="font-medium">普通会话绑定</h5>
              <p className="text-sm text-gray-500 mt-1">
                这里展示常规域名、进程、连接级运行态绑定，不包含稳定出口回写的 domain-rule 记录。
              </p>
            </div>

            <div className="space-y-2">
              {regularBindings.length === 0 ? (
                <div className="text-center py-6 text-gray-500">
                  暂无普通会话绑定
                </div>
              ) : (
                regularBindings.map((binding) => (
                  <BindingItem
                    key={`${binding.bindingType}:${binding.key}`}
                    binding={binding}
                    onClear={() => handleClearBinding(binding)}
                  />
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </Card>
  );
}

/**
 * 绑定项组件
 */
function BindingItem({
  binding,
  onClear,
}: {
  binding: CoordinatorBindingInfo;
  onClear: () => void;
}) {
  const getBindingTypeText = (type: string) => {
    switch (type) {
      case 'domain-rule':
        return '域名规则回写';
      case 'domain':
        return '域名';
      case 'process':
        return '进程';
      case 'connection':
        return '连接';
      default:
        return type;
    }
  };

  const getBindingTypeColor = (type: string) => {
    switch (type) {
      case 'domain-rule':
        return 'bg-purple-100 text-purple-800';
      case 'domain':
        return 'bg-blue-100 text-blue-800';
      case 'process':
        return 'bg-green-100 text-green-800';
      case 'connection':
        return 'bg-purple-100 text-purple-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const getRemainingTimeText = (seconds?: number) => {
    if (seconds === undefined) return '永久';
    if (seconds <= 0) return '已过期';
    
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    
    if (hours > 24) {
      const days = Math.floor(hours / 24);
      return `${days} 天`;
    }
    if (hours > 0) {
      return `${hours} 小时 ${minutes} 分钟`;
    }
    return `${minutes} 分钟`;
  };

  const getTimeAgoText = (unixSeconds: number) => {
    const elapsedSeconds = Math.max(0, Math.floor(Date.now() / 1000) - unixSeconds);

    if (elapsedSeconds < 60) return '刚刚';

    const minutes = Math.floor(elapsedSeconds / 60);
    if (minutes < 60) return `${minutes} 分钟前`;

    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours} 小时前`;

    const days = Math.floor(hours / 24);
    return `${days} 天前`;
  };

  const timeAgo = getTimeAgoText(binding.boundAt);

  return (
    <div className="flex items-center gap-3 p-3 border rounded-lg hover:bg-gray-50">
      <span
        className={`px-2 py-1 text-xs font-medium rounded ${getBindingTypeColor(
          binding.bindingType
        )}`}
      >
        {getBindingTypeText(binding.bindingType)}
      </span>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <code className="text-sm font-mono truncate">{binding.key}</code>
          <span className="text-xs text-gray-500">→</span>
          <span className="text-sm font-medium text-blue-600">
            {binding.nodeId}
          </span>
        </div>
        {binding.sourceGroupName && (
          <div className="text-xs text-purple-600 mt-1">
            来源稳定组：{binding.sourceGroupName}
          </div>
        )}
        {binding.sourceGroupName && (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-2 mt-2 text-xs">
            <div className="rounded border border-purple-200 dark:border-purple-800 px-2 py-1 bg-purple-50/60 dark:bg-purple-950/20">
              <span className="text-gray-500">来源组当前选中节点：</span>
              <span className="ml-1 font-medium text-purple-700 dark:text-purple-300">
                {binding.sourceGroupSelectedNode || '未知'}
              </span>
            </div>
            <div className="rounded border border-blue-200 dark:border-blue-800 px-2 py-1 bg-blue-50/60 dark:bg-blue-950/20">
              <span className="text-gray-500">回写节点：</span>
              <span className="ml-1 font-medium text-blue-700 dark:text-blue-300">
                {binding.nodeId}
              </span>
            </div>
          </div>
        )}
        <div className="flex items-center gap-3 mt-1 text-xs text-gray-500">
          <span>绑定于 {timeAgo}</span>
          <span>•</span>
          <span>剩余 {getRemainingTimeText(binding.remainingSeconds)}</span>
        </div>
      </div>

      <Button
        size="sm"
        variant="ghost"
        onClick={onClear}
        className="text-red-600 hover:text-red-700 hover:bg-red-50"
      >
        清除
      </Button>
    </div>
  );
}
