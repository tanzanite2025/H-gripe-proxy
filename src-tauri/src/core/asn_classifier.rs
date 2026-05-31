/// ASN 分类器
///
/// 基于 ASN 编号查表 + 组织名称关键词匹配，将 IP 归类为 Datacenter/Residential/Mobile/Education/Unknown

use once_cell::sync::Lazy;
use std::collections::HashMap;

use super::asn_data;

/// ASN 分类结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AsnCategory {
    Datacenter,
    Residential,
    Mobile,
    Education,
    Unknown,
}

/// ASN 分类条目
#[derive(Debug, Clone)]
pub struct AsnInfo {
    pub asn: u32,
    pub name: String,
    pub category: AsnCategory,
}

static ASN_TABLE: Lazy<HashMap<u32, AsnInfo>> = Lazy::new(build_asn_table);

fn build_asn_table() -> HashMap<u32, AsnInfo> {
    let mut map = HashMap::new();

    for &(asn, name) in asn_data::datacenter::datacenter_asns() {
        map.insert(asn, AsnInfo {
            asn,
            name: name.to_string(),
            category: AsnCategory::Datacenter,
        });
    }

    for &(asn, name) in asn_data::mobile::mobile_asns() {
        map.insert(asn, AsnInfo {
            asn,
            name: name.to_string(),
            category: AsnCategory::Mobile,
        });
    }

    for &(asn, name) in asn_data::education::education_asns() {
        map.insert(asn, AsnInfo {
            asn,
            name: name.to_string(),
            category: AsnCategory::Education,
        });
    }

    map
}

/// 通过 ASN 编号查询分类
pub fn classify_by_asn(asn: u32) -> Option<&'static AsnInfo> {
    ASN_TABLE.get(&asn)
}

/// 通过 ASN 组织名称关键词匹配分类
pub fn classify_by_org_name(org_name: &str) -> AsnCategory {
    let lower = org_name.to_lowercase();

    // 优先匹配教育（避免 "University Cloud" 误判为机房）
    for kw in asn_data::education::education_keywords() {
        if lower.contains(&kw.to_lowercase()) {
            return AsnCategory::Education;
        }
    }

    // 其次匹配移动
    for kw in asn_data::mobile::mobile_keywords() {
        if lower.contains(&kw.to_lowercase()) {
            return AsnCategory::Mobile;
        }
    }

    // 最后匹配机房
    for kw in asn_data::datacenter::datacenter_keywords() {
        if lower.contains(&kw.to_lowercase()) {
            return AsnCategory::Datacenter;
        }
    }

    AsnCategory::Unknown
}

/// 综合分类：ASN 编号查表优先，名称关键词兜底
pub fn classify(asn: Option<u32>, org_name: Option<&str>) -> AsnCategory {
    // 1. ASN 编号精确匹配
    if let Some(asn_num) = asn {
        if let Some(info) = classify_by_asn(asn_num) {
            return info.category;
        }
    }

    // 2. 组织名称关键词匹配
    if let Some(name) = org_name {
        let cat = classify_by_org_name(name);
        if cat != AsnCategory::Unknown {
            return cat;
        }
    }

    AsnCategory::Unknown
}

/// 获取 ASN 信息（编号查表优先，名称兜底构造）
pub fn get_asn_info(asn: Option<u32>, org_name: Option<&str>) -> AsnInfo {
    if let Some(asn_num) = asn {
        if let Some(info) = classify_by_asn(asn_num) {
            return info.clone();
        }
    }

    let category = classify(asn, org_name);

    AsnInfo {
        asn: asn.unwrap_or(0),
        name: org_name.unwrap_or("Unknown").to_string(),
        category,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_by_asn() {
        let info = classify_by_asn(16509).unwrap();
        assert_eq!(info.category, AsnCategory::Datacenter);
        assert_eq!(info.name, "Amazon AWS");

        let info = classify_by_asn(9808).unwrap();
        assert_eq!(info.category, AsnCategory::Mobile);
    }

    #[test]
    fn test_classify_by_org_name() {
        assert_eq!(classify_by_org_name("Amazon.com, Inc."), AsnCategory::Datacenter);
        assert_eq!(classify_by_org_name("China Mobile"), AsnCategory::Mobile);
        assert_eq!(classify_by_org_name("Tsinghua University"), AsnCategory::Education);
        assert_eq!(classify_by_org_name("Comcast Cable"), AsnCategory::Unknown);
    }

    #[test]
    fn test_classify_combined() {
        // ASN 查表优先
        assert_eq!(classify(Some(16509), Some("Some ISP")), AsnCategory::Datacenter);
        // 名称兜底
        assert_eq!(classify(None, Some("Alibaba Cloud")), AsnCategory::Datacenter);
        // 未知
        assert_eq!(classify(None, None), AsnCategory::Unknown);
    }
}
