use super::CoreManager;
use anyhow::Result;
use compact_str::CompactString;

impl CoreManager {
    pub async fn get_clash_logs(&self) -> Result<Vec<CompactString>> {
        Ok(Vec::new())
    }
}
