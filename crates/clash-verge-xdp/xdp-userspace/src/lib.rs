/**
 * XDP 代理用户态库
 */

use aya::{
    maps::{HashMap, PerCpuArray},
    programs::{Xdp, XdpFlags},
    Ebpf,
};
use std::net::Ipv4Addr;

/// 路由动作
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteAction {
    Pass = 0,
    Proxy = 1,
    Reject = 2,
}

/// 路由表项
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RouteEntry {
    pub action: u32,
    pub proxy_ip: u32,
    pub proxy_port: u16,
    pub _padding: u16,
}

/// 连接跟踪键
#[repr(C)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ConnKey {
    pub src_ip: u32,
    pub dst_ip: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub _padding: [u8; 3],
}

/// 连接状态
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ConnState {
    pub proxy_ip: u32,
    pub proxy_port: u16,
    pub established: u8,
    pub _padding: u8,
    pub packets: u64,
    pub bytes: u64,
}

/// 统计信息
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    pub total_packets: u64,
    pub proxied_packets: u64,
    pub direct_packets: u64,
    pub rejected_packets: u64,
    pub errors: u64,
}

/// XDP 代理加载器
pub struct XdpProxyLoader {
    ebpf: Ebpf,
    interface: String,
}

impl XdpProxyLoader {
    /// 创建新的加载器
    pub fn new(ebpf_bytes: &[u8], interface: &str) -> Result<Self, anyhow::Error> {
        let ebpf = Ebpf::load(ebpf_bytes)?;

        Ok(Self {
            ebpf,
            interface: interface.to_string(),
        })
    }

    /// 附加到网卡
    pub fn attach(&mut self) -> Result<(), anyhow::Error> {
        let program: &mut Xdp = self
            .ebpf
            .program_mut("xdp_proxy")
            .ok_or_else(|| anyhow::anyhow!("Program not found"))?
            .try_into()?;

        program.load()?;
        program.attach(&self.interface, XdpFlags::SKB_MODE)?;

        log::info!("XDP program attached to {}", self.interface);
        Ok(())
    }

    /// 分离
    pub fn detach(&mut self) -> Result<(), anyhow::Error> {
        let program: &mut Xdp = self
            .ebpf
            .program_mut("xdp_proxy")
            .ok_or_else(|| anyhow::anyhow!("Program not found"))?
            .try_into()?;

        program.unload()?;
        log::info!("XDP program detached from {}", self.interface);
        Ok(())
    }

    /// 添加路由规则
    pub fn add_route(
        &mut self,
        dest_ip: Ipv4Addr,
        action: RouteAction,
        proxy_ip: Option<Ipv4Addr>,
        proxy_port: Option<u16>,
    ) -> Result<(), anyhow::Error> {
        let mut route_table: HashMap<_, u32, RouteEntry> =
            HashMap::try_from(self.ebpf.map_mut("ROUTE_TABLE")?)?;

        let entry = RouteEntry {
            action: action as u32,
            proxy_ip: proxy_ip.map(|ip| u32::from(ip)).unwrap_or(0),
            proxy_port: proxy_port.unwrap_or(0),
            _padding: 0,
        };

        let dest_ip_u32 = u32::from(dest_ip);
        route_table.insert(dest_ip_u32, entry, 0)?;

        log::info!(
            "Added route: {} -> {:?} (proxy: {:?}:{})",
            dest_ip,
            action,
            proxy_ip,
            proxy_port.unwrap_or(0)
        );

        Ok(())
    }

    /// 删除路由规则
    pub fn remove_route(&mut self, dest_ip: Ipv4Addr) -> Result<(), anyhow::Error> {
        let mut route_table: HashMap<_, u32, RouteEntry> =
            HashMap::try_from(self.ebpf.map_mut("ROUTE_TABLE")?)?;

        let dest_ip_u32 = u32::from(dest_ip);
        route_table.remove(&dest_ip_u32)?;

        log::info!("Removed route: {}", dest_ip);
        Ok(())
    }

    /// 获取统计信息
    pub fn get_stats(&mut self) -> Result<Stats, anyhow::Error> {
        let stats_map: PerCpuArray<_, Stats> =
            PerCpuArray::try_from(self.ebpf.map_mut("STATS")?)?;

        let mut total_stats = Stats::default();

        // 聚合所有 CPU 的统计
        if let Ok(per_cpu_stats) = stats_map.get(&0, 0) {
            for cpu_stats in per_cpu_stats {
                total_stats.total_packets += cpu_stats.total_packets;
                total_stats.proxied_packets += cpu_stats.proxied_packets;
                total_stats.direct_packets += cpu_stats.direct_packets;
                total_stats.rejected_packets += cpu_stats.rejected_packets;
                total_stats.errors += cpu_stats.errors;
            }
        }

        Ok(total_stats)
    }

    /// 获取连接跟踪信息
    pub fn get_connections(&mut self) -> Result<Vec<(ConnKey, ConnState)>, anyhow::Error> {
        let conn_track: HashMap<_, ConnKey, ConnState> =
            HashMap::try_from(self.ebpf.map_mut("CONN_TRACK")?)?;

        let mut connections = Vec::new();

        for item in conn_track.iter() {
            if let Ok((key, value)) = item {
                connections.push((key, value));
            }
        }

        Ok(connections)
    }

    /// 清除所有路由规则
    pub fn clear_routes(&mut self) -> Result<(), anyhow::Error> {
        let mut route_table: HashMap<_, u32, RouteEntry> =
            HashMap::try_from(self.ebpf.map_mut("ROUTE_TABLE")?)?;

        // 删除所有条目
        let keys: Vec<u32> = route_table
            .iter()
            .filter_map(|item| item.ok().map(|(k, _)| k))
            .collect();

        for key in keys {
            route_table.remove(&key)?;
        }

        log::info!("Cleared all routes");
        Ok(())
    }

    /// 清除连接跟踪表
    pub fn clear_connections(&mut self) -> Result<(), anyhow::Error> {
        let mut conn_track: HashMap<_, ConnKey, ConnState> =
            HashMap::try_from(self.ebpf.map_mut("CONN_TRACK")?)?;

        // 删除所有条目
        let keys: Vec<ConnKey> = conn_track
            .iter()
            .filter_map(|item| item.ok().map(|(k, _)| k))
            .collect();

        for key in keys {
            conn_track.remove(&key)?;
        }

        log::info!("Cleared all connections");
        Ok(())
    }
}

impl Drop for XdpProxyLoader {
    fn drop(&mut self) {
        if let Err(e) = self.detach() {
            log::error!("Failed to detach XDP program: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_entry_size() {
        assert_eq!(std::mem::size_of::<RouteEntry>(), 12);
    }

    #[test]
    fn test_conn_key_size() {
        assert_eq!(std::mem::size_of::<ConnKey>(), 16);
    }

    #[test]
    fn test_conn_state_size() {
        assert_eq!(std::mem::size_of::<ConnState>(), 24);
    }
}
