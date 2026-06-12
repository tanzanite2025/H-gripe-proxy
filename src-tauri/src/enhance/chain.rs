use super::SeqMap;
use crate::{
    config::{PrfItem, profiles::resolve_profile_file_path},
    utils::help,
};
use serde_yaml_ng::Mapping;
use smartstring::alias::String;
use tokio::fs;

#[derive(Debug, Clone)]
pub struct ChainItem {
    pub uid: String,
    pub data: ChainType,
}

#[derive(Debug, Clone)]
pub enum ChainType {
    Merge(Mapping),
    Script(String),
    Proxies(SeqMap),
    Groups(SeqMap),
}

// Helper trait to allow async conversion
pub trait AsyncChainItemFrom {
    async fn from_async(item: &PrfItem) -> Option<ChainItem>;
}

impl AsyncChainItemFrom for Option<ChainItem> {
    async fn from_async(item: &PrfItem) -> Self {
        let itype = item.itype.as_ref()?.as_str();
        let file = item.file.clone()?;
        let uid = item.uid.clone().unwrap_or_else(|| "".into());
        let path = resolve_profile_file_path(file.as_str()).ok()?;

        if !path.exists() {
            return None;
        }

        match itype {
            "script" => Some(ChainItem {
                uid,
                data: ChainType::Script(fs::read_to_string(path).await.ok()?.into()),
            }),
            "merge" => Some(ChainItem {
                uid,
                data: ChainType::Merge(help::read_mapping(&path).await.ok()?),
            }),
            "proxies" => {
                let seq_map = help::read_seq_map(&path).await.ok()?;
                Some(ChainItem {
                    uid,
                    data: ChainType::Proxies(seq_map),
                })
            }
            "groups" => {
                let seq_map = help::read_seq_map(&path).await.ok()?;
                Some(ChainItem {
                    uid,
                    data: ChainType::Groups(seq_map),
                })
            }
            _ => None,
        }
    }
}
