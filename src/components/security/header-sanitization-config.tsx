/**
 * HTTP 头净化配置组件
 * 
 * 配置和测试 HTTP 头净化功能
 */

import { FlaskConical, RefreshCw, Shield } from 'lucide-react';
import {
  useCallback,
  useEffect,
  useState,
  type ChangeEvent,
} from 'react';

import { Button } from '@/components/tailwind/Button';
import { Card } from '@/components/tailwind/Card';
import { Chip } from '@/components/tailwind/Chip';
import { Divider } from '@/components/tailwind/Divider';
import { FormControlLabel } from '@/components/tailwind/FormControlLabel';
import { Paper } from '@/components/tailwind/Paper';
import { Select, type SelectChangeEvent } from '@/components/tailwind/Select';
import { Switch } from '@/components/tailwind/Switch';
import { TextField } from '@/components/tailwind/TextField';
import {
  getHeaderSanitizationConfig,
  updateHeaderSanitizationConfig,
  testHeaderSanitization,
  getHeaderSanitizationFingerprint,
  type HeaderSanitizationConfig,
  type BrowserFingerprint,
} from '@/services/header-sanitization';
import { showNotice } from '@/services/notice-service';

export function HeaderSanitizationConfig() {
  const [config, setConfig] = useState<HeaderSanitizationConfig | null>(null);
  const [fingerprint, setFingerprint] = useState<BrowserFingerprint | null>(null);
  const [testHeaders, setTestHeaders] = useState<Record<string, string>>({
    'User-Agent': 'Old User Agent',
    'X-Forwarded-For': '1.2.3.4',
    'Via': 'proxy-server',
    'Accept': 'text/html',
  });
  const [testResult, setTestResult] = useState<Record<string, string> | null>(null);
  const [loading, setLoading] = useState(false);

  // 加载配置
  const loadFingerprint = useCallback(async (template: string) => {
    try {
      const fp = await getHeaderSanitizationFingerprint(template);
      setFingerprint(fp);
    } catch (error) {
      console.error('Failed to load fingerprint:', error);
    }
  }, []);

  const loadConfig = useCallback(async () => {
    try {
      const cfg = await getHeaderSanitizationConfig();
      setConfig(cfg);
      await loadFingerprint(cfg.browserTemplate);
    } catch (error) {
      console.error('Failed to load header sanitization config:', error);
      showNotice.error('加载配置失败');
    }
  }, [loadFingerprint]);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  // 更新配置
  const handleConfigChange = async (updates: Partial<HeaderSanitizationConfig>) => {
    if (!config) return;

    const newConfig = { ...config, ...updates };
    try {
      await updateHeaderSanitizationConfig(newConfig);
      setConfig(newConfig);
      
      // 如果浏览器模板改变，重新加载指纹
      if (updates.browserTemplate) {
        await loadFingerprint(updates.browserTemplate);
      }
      
      showNotice.success('配置已更新');
    } catch (error) {
      console.error('Failed to update config:', error);
      showNotice.error('更新配置失败');
    }
  };

  // 测试净化效果
  const handleTest = async () => {
    setLoading(true);
    try {
      const result = await testHeaderSanitization(testHeaders);
      setTestResult(result);
      showNotice.success('测试完成');
    } catch (error) {
      console.error('Failed to test sanitization:', error);
      showNotice.error('测试失败');
    } finally {
      setLoading(false);
    }
  };

  // 重置测试
  const handleResetTest = () => {
    setTestHeaders({
      'User-Agent': 'Old User Agent',
      'X-Forwarded-For': '1.2.3.4',
      'Via': 'proxy-server',
      'Accept': 'text/html',
    });
    setTestResult(null);
  };

  if (!config) {
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
          <h3 className="text-lg font-semibold text-text-primary">HTTP 头净化</h3>
          <p className="text-sm text-text-secondary">清除代理特征，伪造真实浏览器指纹</p>
        </div>
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
                label="启用 HTTP 头净化"
              />
              <FormControlLabel
                control={
                  <Switch
                    checked={config.removeProxyHeaders}
                    onChange={(e: ChangeEvent<HTMLInputElement>) =>
                      handleConfigChange({ removeProxyHeaders: e.target.checked })
                    }
                    disabled={!config.enabled}
                  />
                }
                label="清除代理特征头"
              />
              <FormControlLabel
                control={
                  <Switch
                    checked={config.forgeUserAgent}
                    onChange={(e: ChangeEvent<HTMLInputElement>) =>
                      handleConfigChange({ forgeUserAgent: e.target.checked })
                    }
                    disabled={!config.enabled}
                  />
                }
                label="伪造 User-Agent"
              />
              <FormControlLabel
                control={
                  <Switch
                    checked={config.normalizeHeaderOrder}
                    onChange={(e: ChangeEvent<HTMLInputElement>) =>
                      handleConfigChange({ normalizeHeaderOrder: e.target.checked })
                    }
                    disabled={!config.enabled}
                  />
                }
                label="规范化头部顺序"
              />
            </div>
          </div>

          <Divider />

          {/* 浏览器模板 */}
          <div>
            <div className="mb-2 text-sm font-semibold text-text-primary">
              浏览器模板
            </div>
            <Select
              value={config.browserTemplate}
              onChange={(e: SelectChangeEvent) =>
                handleConfigChange({
                  browserTemplate: e.target.value as HeaderSanitizationConfig['browserTemplate'],
                })
              }
              disabled={!config.enabled || !config.forgeUserAgent}
              fullWidth
              size="small"
            >
              <option value="Chrome">Chrome</option>
              <option value="Firefox">Firefox</option>
              <option value="Safari">Safari</option>
              <option value="Edge">Edge</option>
              <option value="Custom">自定义</option>
            </Select>
          </div>

          {/* 浏览器指纹预览 */}
          {fingerprint && (
            <div>
              <div className="mb-2 text-sm font-semibold text-text-primary">
                浏览器指纹预览
              </div>
              <Paper variant="outlined" className="space-y-3 p-4">
                <div>
                  <div className="text-xs text-text-secondary">
                      User-Agent:
                  </div>
                  <div className="break-all text-sm text-text-primary">
                      {fingerprint.userAgent}
                  </div>
                </div>
                <div>
                  <div className="text-xs text-text-secondary">
                      Accept:
                  </div>
                  <div className="break-all text-sm text-text-primary">
                      {fingerprint.accept}
                  </div>
                </div>
                <div>
                  <div className="text-xs text-text-secondary">
                      Accept-Language:
                  </div>
                  <div className="text-sm text-text-primary">{fingerprint.acceptLanguage}</div>
                </div>
              </Paper>
            </div>
          )}

          <Divider />

          {/* 测试区域 */}
          <div>
            <div className="mb-2 text-sm font-semibold text-text-primary">
              测试净化效果
            </div>
            <div className="space-y-4">
              <div>
                <div className="mb-2 text-xs text-text-secondary">
                  测试头部（JSON 格式）
                </div>
                <TextField
                  multiline
                  rows={4}
                  value={JSON.stringify(testHeaders, null, 2)}
                  onChange={(e: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
                    try {
                      setTestHeaders(JSON.parse(e.target.value));
                    } catch (ignore) {
                      // 忽略解析错误
                    }
                  }}
                  fullWidth
                  size="small"
                  className="font-mono"
                />
              </div>

              <div className="flex flex-wrap gap-3">
                <Button
                  variant="contained"
                  startIcon={<FlaskConical className="h-4 w-4" />}
                  onClick={handleTest}
                  disabled={loading || !config.enabled}
                >
                  测试净化
                </Button>
                <Button
                  variant="outlined"
                  startIcon={<RefreshCw className="h-4 w-4" />}
                  onClick={handleResetTest}
                  disabled={loading}
                >
                  重置
                </Button>
              </div>

              {/* 测试结果 */}
              {testResult && (
                <div>
                  <div className="mb-2 text-xs text-text-secondary">
                    净化后的头部
                  </div>
                  <Paper variant="outlined" className="space-y-2 bg-green-50 p-4 dark:bg-green-900/10">
                      {Object.entries(testResult).map(([key, value]) => (
                        <div key={key} className="flex flex-wrap items-start gap-2">
                          <Chip label={key} size="small" />
                          <span className="break-all text-sm text-text-primary">
                            {value}
                          </span>
                        </div>
                      ))}
                  </Paper>
                </div>
              )}
            </div>
          </div>
      </div>
    </Card>
  );
}
