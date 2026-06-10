use crate::feat;
use serde_yaml_ng::Mapping;

pub async fn apply_global_china_rules(config: Mapping) -> Mapping {
    feat::apply_global_china_rules(config).await
}
