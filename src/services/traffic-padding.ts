/**
 * 流量填充服务
 * 
 * 提供流量填充和统计功能
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * 流量填充配置
 */
export interface TrafficPaddingConfig {
  /** 启用填充 */
  enabled: boolean;
  /** 最小填充大小（字节） */
  minSize: number;
  /** 最大填充大小（字节） */
  maxSize: number;
  /** 加密填充数据 */
  encrypt: boolean;
  /** 填充强度 */
  intensity: 'Low' | 'Medium' | 'High' | { Custom: number };
  /** 填充频率 */
  frequency: {
    freqType: 'Time' | 'Request' | 'Random';
    interval: number;
  };
  /** 填充时机 */
  timing: 'Before' | 'After' | 'Random';
  /** 智能填充 */
  smartPadding: boolean;
  /** 性能控制 */
  performanceControl: {
    maxBandwidth: number;
    maxCpuUsage: number;
    maxMemory: number;
    autoDowngrade: boolean;
  };
}

/**
 * 填充统计
 */
export interface PaddingStats {
  /** 填充次数 */
  paddingCount: number;
  /** 填充总大小（字节） */
  totalPaddingSize: number;
  /** 带宽占用（字节/秒） */
  bandwidthUsage: number;
  /** CPU 占用（%） */
  cpuUsage: number;
  /** 内存占用（字节） */
  memoryUsage: number;
  /** 最后填充时间 */
  lastPaddingTime: number;
}

/**
 * 获取流量填充配置
 */
export async function getTrafficPaddingConfig(): Promise<TrafficPaddingConfig> {
  return await invoke<TrafficPaddingConfig>('traffic_padding_get_config');
}

/**
 * 更新流量填充配置
 */
export async function updateTrafficPaddingConfig(
  config: TrafficPaddingConfig
): Promise<void> {
  await invoke('traffic_padding_update_config', { config });
}

/**
 * 启动流量填充
 */
export async function startTrafficPadding(): Promise<void> {
  await invoke('traffic_padding_start');
}

/**
 * 停止流量填充
 */
export async function stopTrafficPadding(): Promise<void> {
  await invoke('traffic_padding_stop');
}

/**
 * 获取流量填充统计
 */
export async function getTrafficPaddingStats(): Promise<PaddingStats> {
  return await invoke<PaddingStats>('traffic_padding_get_stats');
}

/**
 * 重置流量填充统计
 */
export async function resetTrafficPaddingStats(): Promise<void> {
  await invoke('traffic_padding_reset_stats');
}

/**
 * 检查流量填充是否正在运行
 */
export async function isTrafficPaddingRunning(): Promise<boolean> {
  return await invoke<boolean>('traffic_padding_is_running');
}

/**
 * 格式化字节大小
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}
