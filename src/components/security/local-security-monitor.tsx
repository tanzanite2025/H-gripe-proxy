/**
 * 本地安全监控组件
 * 
 * 显示本地安全状态和防火墙配置
 */

import {
  AlertCircle,
  CheckCircle2,
  RefreshCw,
  Shield,
  ShieldAlert,
} from 'lucide-react';
import { useState, useEffect, type ChangeEvent } from 'react';

import { Alert } from '@/components/tailwind/Alert';
import { Button } from '@/components/tailwind/Button';
import { Card } from '@/components/tailwind/Card';
import { Chip } from '@/components/tailwind/Chip';
import { FormControlLabel } from '@/components/tailwind/FormControlLabel';
import { Switch } from '@/components/tailwind/Switch';
import { TextField } from '@/components/tailwind/TextField';
import {
  getLocalSecurityConfig,
  updateLocalSecurityConfig,
  getLocalSecurityStatus,
  checkSecurityNow,
  configureFirewall,
  removeFirewall,
  startLeakMonitor,
  stopLeakMonitor,
  isLeakMonitorRunning,
  type LocalSecurityConfig,
  type LeakMonitorStatus,
} from '@/services/local-security';
import { showNotice } from '@/services/notice-service';

export function LocalSecurityMonitor() {
  const [config, setConfig] = useState<LocalSecurityConfig | null>(null);
  const [status, setStatus] = useState<LeakMonitorStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [port, setPort] = useState(10808);
  const [monitorRunning, setMonitorRunning] = useState(false);

  // 加载配置和状态
  const loadData = async () => {
    try {
      const [cfg, st, running] = await Promise.all([
        getLocalSecurityConfig(),
        getLocalSecurityStatus(),
        isLeakMonitorRunning(),
      ]);
      setConfig(cfg);
      setStatus(st);
      setMonitorRunning(running);
    } catch (error) {
      console.error('Failed to load local security data:', error);
      showNotice.error('加载本地安全数据失败');
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  // 更新配置
  const handleConfigChange = async (updates: Partial<LocalSecurityConfig>) => {
    if (!config) return;

    const newConfig = { ...config, ...updates };
    try {
      await updateLocalSecurityConfig(newConfig);
      setConfig(newConfig);
      showNotice.success('配置已更新');
    } catch (error) {
      console.error('Failed to update config:', error);
      showNotice.error('更新配置失败');
    }
  };

  // 立即检查
  const handleCheckNow = async () => {
    setLoading(true);
    try {
      const newStatus = await checkSecurityNow(port);
      setStatus(newStatus);
      showNotice.success('安全检查完成');
    } catch (error) {
      console.error('Failed to check security:', error);
      showNotice.error('安全检查失败');
    } finally {
      setLoading(false);
    }
  };

  // 配置防火墙
  const handleConfigureFirewall = async () => {
    setLoading(true);
    try {
      await configureFirewall(port);
      showNotice.success('防火墙规则已配置');
      await handleCheckNow();
    } catch (error) {
      console.error('Failed to configure firewall:', error);
      showNotice.error(`配置防火墙失败: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  // 删除防火墙规则
  const handleRemoveFirewall = async () => {
    setLoading(true);
    try {
      await removeFirewall(port);
      showNotice.success('防火墙规则已删除');
      await handleCheckNow();
    } catch (error) {
      console.error('Failed to remove firewall:', error);
      showNotice.error(`删除防火墙规则失败: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  // 启动泄漏监控
  const handleStartMonitor = async () => {
    setLoading(true);
    try {
      await startLeakMonitor(port);
      setMonitorRunning(true);
      showNotice.success('泄漏监控已启动');
    } catch (error) {
      console.error('Failed to start leak monitor:', error);
      showNotice.error(`启动泄漏监控失败: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  // 停止泄漏监控
  const handleStopMonitor = async () => {
    setLoading(true);
    try {
      await stopLeakMonitor();
      setMonitorRunning(false);
      showNotice.success('泄漏监控已停止');
    } catch (error) {
      console.error('Failed to stop leak monitor:', error);
      showNotice.error(`停止泄漏监控失败: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  if (!config || !status) {
    return (
      <Card className="p-6">
        <div className="text-sm text-gray-500 dark:text-gray-400">加载中...</div>
      </Card>
    );
  }

  return (
    <Card className="p-6">
      <div className="flex items-start gap-3">
        <div className="rounded-full bg-primary/10 p-2 text-primary">
          <Shield className="h-5 w-5" />
        </div>
        <div>
          <h3 className="text-lg font-semibold text-text-primary">入口隐蔽监控</h3>
          <p className="text-sm text-text-secondary">本地安全和防火墙保护</p>
        </div>
      </div>
      <div className="mt-6 space-y-6">
          {/* 状态指示器 */}
          <div>
            <div className="mb-2 text-sm font-semibold text-text-primary">
              安全状态
            </div>
            <div className="flex flex-wrap gap-2">
              <Chip
                icon={status.localBindingSecure ? <CheckCircle2 className="h-3.5 w-3.5" /> : <AlertCircle className="h-3.5 w-3.5" />}
                label="本地绑定"
                color={status.localBindingSecure ? 'success' : 'error'}
                size="small"
              />
              <Chip
                icon={status.firewallRulesActive ? <CheckCircle2 className="h-3.5 w-3.5" /> : <AlertCircle className="h-3.5 w-3.5" />}
                label="防火墙规则"
                color={status.firewallRulesActive ? 'success' : 'warning'}
                size="small"
              />
              <Chip
                icon={status.externalAccessBlocked ? <CheckCircle2 className="h-3.5 w-3.5" /> : <AlertCircle className="h-3.5 w-3.5" />}
                label="外部访问阻止"
                color={status.externalAccessBlocked ? 'success' : 'error'}
                size="small"
              />
              <Chip
                icon={status.processHidden ? <CheckCircle2 className="h-3.5 w-3.5" /> : <AlertCircle className="h-3.5 w-3.5" />}
                label="进程隐蔽"
                color={status.processHidden ? 'success' : 'default'}
                size="small"
              />
            </div>
          </div>

          {/* 泄漏警告 */}
          {status.leakDetected && (
            <Alert severity="error">
              <div className="space-y-1">
                <div className="flex items-center gap-2 text-sm font-bold">
                  <ShieldAlert className="h-4 w-4" />
                  检测到安全泄漏！
                </div>
              {status.leakType && (
                  <div className="text-xs">{status.leakType}</div>
              )}
              </div>
            </Alert>
          )}

          {/* 配置选项 */}
          <div>
            <div className="mb-2 text-sm font-semibold text-text-primary">
              配置选项
            </div>
            <div className="space-y-2">
              <FormControlLabel
                control={
                  <Switch
                    checked={config.autoFirewall}
                    onChange={(e: ChangeEvent<HTMLInputElement>) =>
                      handleConfigChange({ autoFirewall: e.target.checked })
                    }
                  />
                }
                label="自动配置防火墙"
              />
              <FormControlLabel
                control={
                  <Switch
                    checked={config.leakMonitoring}
                    onChange={(e: ChangeEvent<HTMLInputElement>) =>
                      handleConfigChange({ leakMonitoring: e.target.checked })
                    }
                  />
                }
                label="启用泄漏监控"
              />
              <FormControlLabel
                control={
                  <Switch
                    checked={config.autoSwitchOnConflict}
                    onChange={(e: ChangeEvent<HTMLInputElement>) =>
                      handleConfigChange({
                        autoSwitchOnConflict: e.target.checked,
                      })
                    }
                  />
                }
                label="端口冲突自动切换"
              />
            </div>
          </div>

          {/* 防火墙操作 */}
          <div>
            <div className="mb-2 text-sm font-semibold text-text-primary">
              防火墙管理
            </div>
            <div className="flex flex-wrap items-center gap-3">
              <TextField
                label="端口"
                type="number"
                value={port}
                onChange={(e: ChangeEvent<HTMLInputElement>) => setPort(Number(e.target.value))}
                size="small"
                className="w-[120px]"
              />
              <Button
                variant="contained"
                onClick={handleConfigureFirewall}
                disabled={loading}
                size="small"
              >
                配置防火墙
              </Button>
              <Button
                variant="outlined"
                onClick={handleRemoveFirewall}
                disabled={loading}
                size="small"
              >
                删除规则
              </Button>
            </div>
            <div className="mt-2 text-xs text-text-secondary">
              配置防火墙规则以阻止外部访问，仅允许本地连接
            </div>
          </div>

          {/* 操作按钮 */}
          <div className="flex flex-wrap gap-3">
            <Button
              variant="outlined"
              startIcon={<RefreshCw className="h-4 w-4" />}
              onClick={handleCheckNow}
              disabled={loading}
            >
              立即检查
            </Button>
            {monitorRunning ? (
              <Button
                variant="contained"
                color="error"
                onClick={handleStopMonitor}
                disabled={loading}
              >
                停止监控
              </Button>
            ) : (
              <Button
                variant="contained"
                color="success"
                onClick={handleStartMonitor}
                disabled={loading}
              >
                启动监控
              </Button>
            )}
          </div>

          {/* 最后检查时间 */}
          {status.lastCheckTime > 0 && (
            <div className="text-xs text-text-secondary">
              最后检查: {new Date(status.lastCheckTime * 1000).toLocaleString()}
            </div>
          )}
      </div>
    </Card>
  );
}
