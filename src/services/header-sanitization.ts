/**
 * HTTP 头净化服务
 * 
 * 提供 HTTP 头净化和浏览器指纹伪造功能
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * HTTP 头净化配置
 */
export interface HeaderSanitizationConfig {
  /** 启用净化 */
  enabled: boolean;
  /** 清除代理头 */
  removeProxyHeaders: boolean;
  /** 自定义要清除的头 */
  customHeadersToRemove: string[];
  /** 伪造 User-Agent */
  forgeUserAgent: boolean;
  /** 浏览器模板 */
  browserTemplate: 'Chrome' | 'Firefox' | 'Safari' | 'Edge' | 'Custom';
  /** 自定义 User-Agent */
  customUserAgent?: string;
  /** 规范化 Accept 头 */
  normalizeAccept: boolean;
  /** 规范化头部顺序 */
  normalizeHeaderOrder: boolean;
}

/**
 * 浏览器指纹
 */
export interface BrowserFingerprint {
  /** User-Agent */
  userAgent: string;
  /** Accept */
  accept: string;
  /** Accept-Language */
  acceptLanguage: string;
  /** Accept-Encoding */
  acceptEncoding: string;
  /** 头部顺序 */
  headerOrder: string[];
}

/**
 * 获取 HTTP 头净化配置
 */
export async function getHeaderSanitizationConfig(): Promise<HeaderSanitizationConfig> {
  return await invoke<HeaderSanitizationConfig>('header_sanitization_get_config');
}

/**
 * 更新 HTTP 头净化配置
 */
export async function updateHeaderSanitizationConfig(
  config: HeaderSanitizationConfig
): Promise<void> {
  await invoke('header_sanitization_update_config', { config });
}

/**
 * 测试 HTTP 头净化效果
 */
export async function testHeaderSanitization(
  headers: Record<string, string>
): Promise<Record<string, string>> {
  return await invoke<Record<string, string>>('header_sanitization_test', { headers });
}

/**
 * 获取浏览器模板列表
 */
export async function getHeaderSanitizationTemplates(): Promise<string[]> {
  return await invoke<string[]>('header_sanitization_get_templates');
}

/**
 * 获取指定浏览器模板的指纹
 */
export async function getHeaderSanitizationFingerprint(
  template: string
): Promise<BrowserFingerprint> {
  return await invoke<BrowserFingerprint>('header_sanitization_get_fingerprint', {
    template,
  });
}
