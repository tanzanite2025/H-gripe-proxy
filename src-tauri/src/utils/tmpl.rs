//! Some config file template

/// template for new a profile item
pub const ITEM_LOCAL: &str = "# Profile Template for H-gripe-proxy

proxies: []

proxy-groups: []

rules: []
";

/// enhanced profile
pub const ITEM_MERGE: &str = "# Profile Enhancement Merge Template for H-gripe-proxy

profile:
  store-selected: true
";

pub const ITEM_MERGE_EMPTY: &str = "# Profile Enhancement Merge Template for H-gripe-proxy

";

/// enhanced profile
pub const ITEM_SCRIPT: &str = "// Define main function (script entry)

function main(config, profileName) {
  return config;
}
";

pub const CHINA_RULES_TEMPLATE: &str = "# Built-in china rules for H-gripe-proxy
#
# Keep this file focused on China direct-routing only.
# Do not put Google / OpenAI / overseas-service routing here.

rules:
  # Explicit mainland services that should remain direct.
  - DOMAIN-SUFFIX,bilibili.com,DIRECT
  - DOMAIN-SUFFIX,bilivideo.com,DIRECT
  - DOMAIN-SUFFIX,biliapi.com,DIRECT
  - DOMAIN-SUFFIX,hdslb.com,DIRECT
  - DOMAIN-SUFFIX,12306.cn,DIRECT
  - DOMAIN-SUFFIX,unionpay.com,DIRECT
  - DOMAIN-SUFFIX,95516.com,DIRECT
  - DOMAIN-SUFFIX,icbc.com.cn,DIRECT
  - DOMAIN-SUFFIX,ccb.com,DIRECT
  - DOMAIN-SUFFIX,abchina.com,DIRECT
  - DOMAIN-SUFFIX,boc.cn,DIRECT
  - DOMAIN-SUFFIX,cmbchina.com,DIRECT
  - DOMAIN-SUFFIX,psbc.com,DIRECT

  # Broad mainland direct-routing guardrails.
  - GEOSITE,CN,DIRECT
  - GEOIP,CN,DIRECT,no-resolve
";

/// enhanced profile
pub const ITEM_PROXIES: &str = "# Profile Enhancement Proxies Template for H-gripe-proxy

prepend: []

append: []

delete: []
";

/// enhanced profile
pub const ITEM_GROUPS: &str = "# Profile Enhancement Groups Template for H-gripe-proxy

prepend: []

append: []

delete: []
";
