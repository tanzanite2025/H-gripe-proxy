import { useState, useEffect } from 'react';

import {
  ipReputationCheckIp,
  getIpTypeText,
  getRiskLevelText,
  getRiskLevelColor,
  type IpReputation,
} from '@/services/ip-reputation';

/**
 * IP 信誉度徽章组件
 */
export function IpReputationBadge({ ip }: { ip: string }) {
  const [reputation, setReputation] = useState<IpReputation | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!ip) {
      return;
    }

    let cancelled = false;

    const checkReputation = async () => {
      setLoading(true);
      setError(null);
      try {
        const rep = await ipReputationCheckIp(ip);
        if (!cancelled) {
          setReputation(rep);
        }
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    void checkReputation();

    return () => {
      cancelled = true;
    };
  }, [ip]);

  if (loading) {
    return (
      <div className="inline-flex items-center gap-1 text-xs text-gray-500">
        <span className="animate-spin">⏳</span>
        <span>检测中...</span>
      </div>
    );
  }

  if (error || !reputation) {
    return null;
  }

  const getIpTypeIcon = (ipType: string) => {
    switch (ipType) {
      case 'Datacenter':
        return '🏢';
      case 'Residential':
        return '🏠';
      case 'Mobile':
        return '📱';
      default:
        return '❓';
    }
  };

  const getIpTypeColor = (ipType: string) => {
    switch (ipType) {
      case 'Datacenter':
        return 'bg-orange-100 text-orange-800';
      case 'Residential':
        return 'bg-green-100 text-green-800';
      case 'Mobile':
        return 'bg-teal-100 text-teal-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  return (
    <div className="inline-flex items-center gap-2">
      {/* IP 类型徽章 */}
      <span
        className={`inline-flex items-center gap-1 px-2 py-1 text-xs font-medium rounded ${getIpTypeColor(
          reputation.ipType
        )}`}
      >
        <span>{getIpTypeIcon(reputation.ipType)}</span>
        <span>{getIpTypeText(reputation.ipType)}</span>
      </span>

      {/* 风险评分 */}
      <span
        className={`inline-flex items-center gap-1 px-2 py-1 text-xs font-medium rounded ${getRiskLevelColor(
          reputation.riskLevel
        )} bg-opacity-10`}
      >
        <span>评分: {reputation.fraudScore}</span>
      </span>

      {/* 风险等级 */}
      <span className={`text-xs font-medium ${getRiskLevelColor(reputation.riskLevel)}`}>
        {getRiskLevelText(reputation.riskLevel)}
      </span>
    </div>
  );
}

/**
 * 简化版 IP 信誉度徽章（仅显示 IP 类型）
 */
export function IpReputationBadgeSimple({ ip }: { ip: string }) {
  const [reputation, setReputation] = useState<IpReputation | null>(null);

  useEffect(() => {
    if (ip) {
      ipReputationCheckIp(ip)
        .then(setReputation)
        .catch(() => {});
    }
  }, [ip]);

  if (!reputation) {
    return null;
  }

  const getIpTypeIcon = (ipType: string) => {
    switch (ipType) {
      case 'Datacenter':
        return '🏢';
      case 'Residential':
        return '🏠';
      case 'Mobile':
        return '📱';
      default:
        return '❓';
    }
  };

  const getIpTypeColor = (ipType: string) => {
    switch (ipType) {
      case 'Datacenter':
        return 'bg-orange-100 text-orange-800';
      case 'Residential':
        return 'bg-green-100 text-green-800';
      case 'Mobile':
        return 'bg-teal-100 text-teal-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  return (
    <span
      className={`inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded ${getIpTypeColor(
        reputation.ipType
      )}`}
      title={`欺诈评分: ${reputation.fraudScore}`}
    >
      <span>{getIpTypeIcon(reputation.ipType)}</span>
      <span>{getIpTypeText(reputation.ipType)}</span>
    </span>
  );
}
