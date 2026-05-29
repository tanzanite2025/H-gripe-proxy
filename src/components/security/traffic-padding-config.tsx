/**
 * 流量填充配置组件
 * 
 * 配置和监控流量填充功能
 */

import {
  BarChart3,
  Play,
  RefreshCw,
  Signal,
  Square,
} from 'lucide-react';
import { useState, useEffect, type ChangeEvent } from 'react';

import { Button } from '@/components/tailwind/Button';
import { Card } from '@/components/tailwind/Card';
import { Chip } from '@/components/tailwind/Chip';
import { Divider } from '@/components/tailwind/Divider';
import { FormControlLabel } from '@/components/tailwind/FormControlLabel';
import { LinearProgress } from '@/components/tailwind/LinearProgress';
import { Paper } from '@/components/tailwind/Paper';
import { Select, type SelectChangeEvent } from '@/components/tailwind/Select';
import { Switch } from '@/components/tailwind/Switch';
import { TextField } from '@/components/tailwind/TextField';
import { showNotice } from '@/services/notice-service';
import {
  getTrafficPaddingConfig,
  updateTrafficPaddingConfig,
  startTrafficPadding,
  stopTrafficPadding,
  getTrafficPaddingStats,
  resetTrafficPaddingStats,
  isTrafficPaddingRunning,
  formatBytes,
  type TrafficPaddingConfig,
  type PaddingStats,
} from '@/services/traffic-padding';

export function TrafficPaddingConfig() {
  const [config, setConfig] = useState<TrafficPaddingConfig | null>(null);
  const [stats, setStats] = useState<PaddingStats | null>(null);
  const [running, setRunning] = useState(false);
  const [loading, setLoading] = useState(false);

  // 加载配置和状态
  const loadData = async () => {
    try {
      const [cfg, st, isRunning] = await Promise.all([
        getTrafficPaddingConfig(),
        getTrafficPaddingStats(),
        isTrafficPaddingRunning(),
      ]);
      setConfig(cfg);
      setStats(st);
      setRunning(isRunning);
    } catch (error) {
      console.error('Failed to load traffic padding data:', error);
      showNotice.error('加载配置失败');
    }
  };

  useEffect(() => {
    loadData();

    // 定期更新统计
    const interval = setInterval(async () => {
      if (running) {
        try {
          const st = await getTrafficPaddingStats();
          setStats(st);
        } catch (error) {
          console.error('Failed to update stats:', error);
        }
      }
    }, 2000);

    return () => clearInterval(interval);
  }, [running]);

  // 更新配置
  const handleConfigChange = async (updates: Partial<TrafficPaddingConfig>) => {
    if (!config) return;

    const newConfig = { ...config, ...updates };
    try {
      await updateTrafficPaddingConfig(newConfig);
      setConfig(newConfig);
      showNotice.success('配置已更新');
    } catch (error) {
      console.error('Failed to update config:', error);
      showNotice.error('更新配置失败');
    }
  };

  // 启动填充
  const handleStart = async () => {
    setLoading(true);
    try {
      await startTrafficPadding();
      setRunning(true);
      showNotice.success('流量填充已启动');
    } catch (error) {
      console.error('Failed to start padding:', error);
      showNotice.error(`启动失败: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  // 停止填充
  const handleStop = async () => {
    setLoading(true);
    try {
      await stopTrafficPadding();
      setRunning(false);
      showNotice.success('流量填充已停止');
    } catch (error) {
      console.error('Failed to stop padding:', error);
      showNotice.error(`停止失败: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  // 重置统计
  const handleResetStats = async () => {
    try {
      await resetTrafficPaddingStats();
      setStats({
        paddingCount: 0,
        totalPaddingSize: 0,
        bandwidthUsage: 0,
        cpuUsage: 0,
        memoryUsage: 0,
        lastPaddingTime: 0,
      });
      showNotice.success('统计已重置');
    } catch (error) {
      console.error('Failed to reset stats:', error);
      showNotice.error('重置失败');
    }
  };

  if (!config || !stats) {
    return (
      <Card className="p-6">
        <div className="text-sm text-gray-500 dark:text-gray-400">加载中...</div>
      </Card>
    );
  }

  const intensityValue =
    typeof config.intensity === 'string'
      ? config.intensity === 'Low'
        ? 0
        : config.intensity === 'Medium'
        ? 1
        : 2
      : 1;

  return (
    <Card className="p-6">
      <div className="flex items-start justify-between gap-4">
        <div className="flex items-start gap-3">
          <div className="rounded-full bg-primary/10 p-2 text-primary">
            <Signal className="h-5 w-5" />
          </div>
          <div>
            <h3 className="text-lg font-semibold text-text-primary">流量填充</h3>
            <p className="text-sm text-text-secondary">混淆流量模式，增强隐私保护</p>
          </div>
        </div>
        {running ? (
          <Chip label="运行中" color="success" size="small" />
        ) : (
          <Chip label="已停止" color="default" size="small" />
        )}
      </div>
      <div className="mt-6 space-y-6">
          {/* 基本配置 */}
          <div>
            <div className="mb-2 text-sm font-semibold text-text-primary">
              基本配置
            </div>
            <div className="space-y-2">
              <FormControlLabel
                control={
                  <Switch
                    checked={config.enabled}
                    onChange={(e: ChangeEvent<HTMLInputElement>) => handleConfigChange({ enabled: e.target.checked })}
                  />
                }
                label="启用流量填充"
              />
              <FormControlLabel
                control={
                  <Switch
                    checked={config.encrypt}
                    onChange={(e: ChangeEvent<HTMLInputElement>) => handleConfigChange({ encrypt: e.target.checked })}
                    disabled={!config.enabled}
                  />
                }
                label="加密填充数据"
              />
              <FormControlLabel
                control={
                  <Switch
                    checked={config.smartPadding}
                    onChange={(e: ChangeEvent<HTMLInputElement>) =>
                      handleConfigChange({ smartPadding: e.target.checked })
                    }
                    disabled={!config.enabled}
                  />
                }
                label="智能填充（根据流量自动调整）"
              />
            </div>
          </div>

          <Divider />

          {/* 填充强度 */}
          <div>
            <div className="mb-2 text-sm font-semibold text-text-primary">
              填充强度
            </div>
            <input
              type="range"
              min={0}
              max={2}
              step={1}
              value={intensityValue}
              disabled={!config.enabled}
              onChange={(e: ChangeEvent<HTMLInputElement>) => {
                const value = Number(e.target.value);
                const intensity = value === 0 ? 'Low' : value === 1 ? 'Medium' : 'High';
                void handleConfigChange({ intensity });
              }}
              className="w-full accent-primary"
            />
            <div className="mt-2 flex justify-between text-xs text-text-secondary">
              <span>低</span>
              <span>中</span>
              <span>高</span>
            </div>
          </div>

          {/* 填充大小 */}
          <div>
            <div className="mb-2 text-sm font-semibold text-text-primary">
              填充大小范围
            </div>
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
              <TextField
                label="最小（字节）"
                type="number"
                value={config.minSize}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  void handleConfigChange({ minSize: Number(e.target.value) })
                }
                disabled={!config.enabled}
                size="small"
                fullWidth
              />
              <TextField
                label="最大（字节）"
                type="number"
                value={config.maxSize}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  void handleConfigChange({ maxSize: Number(e.target.value) })
                }
                disabled={!config.enabled}
                size="small"
                fullWidth
              />
            </div>
          </div>

          {/* 填充频率 */}
          <div>
            <div className="mb-2 text-sm font-semibold text-text-primary">
              填充频率
            </div>
            <div className="flex flex-wrap items-center gap-3">
              <Select
                value={config.frequency.freqType}
                onChange={(e: SelectChangeEvent) =>
                  handleConfigChange({
                    frequency: {
                      ...config.frequency,
                      freqType: e.target.value as TrafficPaddingConfig['frequency']['freqType'],
                    },
                  })
                }
                disabled={!config.enabled}
                size="small"
                className="w-[140px]"
              >
                <option value="Time">定时</option>
                <option value="Request">按请求</option>
                <option value="Random">随机</option>
              </Select>
              <TextField
                label="间隔（秒）"
                type="number"
                value={config.frequency.interval}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  handleConfigChange({
                    frequency: {
                      ...config.frequency,
                      interval: Number(e.target.value),
                    },
                  })
                }
                disabled={!config.enabled}
                size="small"
                className="w-[150px]"
              />
            </div>
          </div>

          <Divider />

          {/* 统计信息 */}
          <div>
            <div className="mb-3 flex items-center gap-2 text-sm font-semibold text-text-primary">
              <BarChart3 className="h-4 w-4" />
              <span>统计信息</span>
            </div>
            <Paper variant="outlined" className="space-y-4 p-4">
                <div>
                  <div className="text-xs text-text-secondary">
                    填充次数
                  </div>
                  <div className="text-xl font-semibold text-text-primary">{stats.paddingCount}</div>
                </div>
                <div>
                  <div className="text-xs text-text-secondary">
                    填充总大小
                  </div>
                  <div className="text-xl font-semibold text-text-primary">
                    {formatBytes(stats.totalPaddingSize)}
                  </div>
                </div>
                <div>
                  <div className="text-xs text-text-secondary">
                    带宽占用
                  </div>
                  <div className="text-sm text-text-primary">
                    {formatBytes(stats.bandwidthUsage)}/s
                  </div>
                  <LinearProgress
                    variant="determinate"
                    value={Math.min(
                      (stats.bandwidthUsage / config.performanceControl.maxBandwidth) *
                        100,
                      100
                    )}
                    className="mt-2"
                  />
                </div>
                {stats.lastPaddingTime > 0 && (
                  <div>
                    <div className="text-xs text-text-secondary">
                      最后填充时间
                    </div>
                    <div className="text-sm text-text-primary">
                      {new Date(stats.lastPaddingTime * 1000).toLocaleString()}
                    </div>
                  </div>
                )}
            </Paper>
          </div>

          {/* 操作按钮 */}
          <div className="flex flex-wrap gap-3">
            {running ? (
              <Button
                variant="contained"
                color="error"
                startIcon={<Square className="h-4 w-4" />}
                onClick={handleStop}
                disabled={loading}
              >
                停止填充
              </Button>
            ) : (
              <Button
                variant="contained"
                color="success"
                startIcon={<Play className="h-4 w-4" />}
                onClick={handleStart}
                disabled={loading || !config.enabled}
              >
                启动填充
              </Button>
            )}
            <Button
              variant="outlined"
              startIcon={<RefreshCw className="h-4 w-4" />}
              onClick={handleResetStats}
              disabled={loading}
            >
              重置统计
            </Button>
          </div>
      </div>
    </Card>
  );
}
