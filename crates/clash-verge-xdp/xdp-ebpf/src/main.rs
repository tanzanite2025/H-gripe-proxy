#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::{HashMap, PerCpuArray},
    programs::XdpContext,
};
use aya_log_ebpf::info;

/// 路由动作
#[repr(u32)]
#[derive(Clone, Copy)]
pub enum RouteAction {
    Pass = 0,    // 直连
    Proxy = 1,   // 代理
    Reject = 2,  // 拒绝
}

/// 路由表项
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RouteEntry {
    pub action: u32,        // RouteAction
    pub proxy_ip: u32,      // 代理服务器 IP
    pub proxy_port: u16,    // 代理服务器端口
    pub _padding: u16,
}

/// 连接跟踪键
#[repr(C)]
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
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
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy)]
pub struct Stats {
    pub total_packets: u64,
    pub proxied_packets: u64,
    pub direct_packets: u64,
    pub rejected_packets: u64,
    pub errors: u64,
}

// 路由表：目标 IP -> 路由规则
#[map]
static ROUTE_TABLE: HashMap<u32, RouteEntry> = HashMap::with_max_entries(10000, 0);

// 连接跟踪表：连接五元组 -> 连接状态
#[map]
static CONN_TRACK: HashMap<ConnKey, ConnState> = HashMap::with_max_entries(100000, 0);

// 统计计数器（Per-CPU）
#[map]
static STATS: PerCpuArray<Stats> = PerCpuArray::with_max_entries(1, 0);

/// 以太网头部
#[repr(C)]
struct EthHdr {
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    ether_type: u16,
}

impl EthHdr {
    const LEN: usize = 14;
}

/// IP 头部（简化版）
#[repr(C)]
struct IpHdr {
    version_ihl: u8,
    tos: u8,
    tot_len: u16,
    id: u16,
    frag_off: u16,
    ttl: u8,
    protocol: u8,
    check: u16,
    saddr: u32,
    daddr: u32,
}

impl IpHdr {
    const LEN: usize = 20;
    
    fn ihl(&self) -> usize {
        ((self.version_ihl & 0x0F) * 4) as usize
    }
}

/// TCP 头部（简化版）
#[repr(C)]
struct TcpHdr {
    source: u16,
    dest: u16,
    seq: u32,
    ack_seq: u32,
    _flags: u16,
    window: u16,
    check: u16,
    urg_ptr: u16,
}

/// UDP 头部
#[repr(C)]
struct UdpHdr {
    source: u16,
    dest: u16,
    len: u16,
    check: u16,
}

const ETH_P_IP: u16 = 0x0800;
const IPPROTO_TCP: u8 = 6;
const IPPROTO_UDP: u8 = 17;

/// 从上下文中读取指针
#[inline(always)]
unsafe fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = core::mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *const T)
}

/// 从上下文中读取可变指针
#[inline(always)]
unsafe fn ptr_at_mut<T>(ctx: &XdpContext, offset: usize) -> Result<*mut T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = core::mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *mut T)
}

/// 更新统计信息
#[inline(always)]
fn update_stats<F>(f: F)
where
    F: FnOnce(&mut Stats),
{
    unsafe {
        if let Some(stats) = STATS.get_ptr_mut(0) {
            f(&mut *stats);
        }
    }
}

/// XDP 主程序
#[xdp]
pub fn xdp_proxy(ctx: XdpContext) -> u32 {
    match try_xdp_proxy(ctx) {
        Ok(ret) => ret,
        Err(_) => {
            update_stats(|s| s.errors += 1);
            xdp_action::XDP_ABORTED
        }
    }
}

fn try_xdp_proxy(ctx: XdpContext) -> Result<u32, ()> {
    // 更新总包数
    update_stats(|s| s.total_packets += 1);

    // 1. 解析以太网头
    let ethhdr = unsafe { ptr_at::<EthHdr>(&ctx, 0)? };
    let eth_type = unsafe { (*ethhdr).ether_type };

    // 只处理 IPv4
    if u16::from_be(eth_type) != ETH_P_IP {
        return Ok(xdp_action::XDP_PASS);
    }

    // 2. 解析 IP 头
    let iphdr = unsafe { ptr_at::<IpHdr>(&ctx, EthHdr::LEN)? };
    let ip_hdr_len = unsafe { (*iphdr).ihl() };
    let protocol = unsafe { (*iphdr).protocol };
    let saddr = unsafe { u32::from_be((*iphdr).saddr) };
    let daddr = unsafe { u32::from_be((*iphdr).daddr) };

    // 3. 解析传输层端口
    let (src_port, dst_port) = match protocol {
        IPPROTO_TCP => {
            let tcphdr = unsafe { ptr_at::<TcpHdr>(&ctx, EthHdr::LEN + ip_hdr_len)? };
            let src = unsafe { u16::from_be((*tcphdr).source) };
            let dst = unsafe { u16::from_be((*tcphdr).dest) };
            (src, dst)
        }
        IPPROTO_UDP => {
            let udphdr = unsafe { ptr_at::<UdpHdr>(&ctx, EthHdr::LEN + ip_hdr_len)? };
            let src = unsafe { u16::from_be((*udphdr).source) };
            let dst = unsafe { u16::from_be((*udphdr).dest) };
            (src, dst)
        }
        _ => {
            // 不支持的协议，直接放行
            return Ok(xdp_action::XDP_PASS);
        }
    };

    // 4. 构建连接键
    let conn_key = ConnKey {
        src_ip: saddr,
        dst_ip: daddr,
        src_port,
        dst_port,
        protocol,
        _padding: [0; 3],
    };

    // 5. 查找连接跟踪表
    if let Some(conn_state) = unsafe { CONN_TRACK.get(&conn_key) } {
        // 已有连接，更新统计
        let mut state = *conn_state;
        state.packets += 1;
        state.bytes += unsafe { u16::from_be((*iphdr).tot_len) as u64 };
        
        unsafe {
            CONN_TRACK.insert(&conn_key, &state, 0).ok();
        }

        // 根据连接状态决定动作
        if state.proxy_ip != 0 {
            update_stats(|s| s.proxied_packets += 1);
            // TODO: 实现代理转发逻辑
            return Ok(xdp_action::XDP_PASS);
        } else {
            update_stats(|s| s.direct_packets += 1);
            return Ok(xdp_action::XDP_PASS);
        }
    }

    // 6. 新连接，查找路由表
    if let Some(route) = unsafe { ROUTE_TABLE.get(&daddr) } {
        let action = route.action;

        match action {
            0 => {
                // Pass: 直连
                update_stats(|s| s.direct_packets += 1);
                
                // 创建连接跟踪
                let conn_state = ConnState {
                    proxy_ip: 0,
                    proxy_port: 0,
                    established: 1,
                    _padding: 0,
                    packets: 1,
                    bytes: unsafe { u16::from_be((*iphdr).tot_len) as u64 },
                };
                
                unsafe {
                    CONN_TRACK.insert(&conn_key, &conn_state, 0).ok();
                }

                Ok(xdp_action::XDP_PASS)
            }
            1 => {
                // Proxy: 代理
                update_stats(|s| s.proxied_packets += 1);
                
                // 创建连接跟踪
                let conn_state = ConnState {
                    proxy_ip: route.proxy_ip,
                    proxy_port: route.proxy_port,
                    established: 1,
                    _padding: 0,
                    packets: 1,
                    bytes: unsafe { u16::from_be((*iphdr).tot_len) as u64 },
                };
                
                unsafe {
                    CONN_TRACK.insert(&conn_key, &conn_state, 0).ok();
                }

                // TODO: 实现代理转发逻辑
                info!(&ctx, "Proxying packet to {}:{}", route.proxy_ip, route.proxy_port);
                Ok(xdp_action::XDP_PASS)
            }
            2 => {
                // Reject: 拒绝
                update_stats(|s| s.rejected_packets += 1);
                Ok(xdp_action::XDP_DROP)
            }
            _ => Ok(xdp_action::XDP_PASS),
        }
    } else {
        // 没有匹配的路由，默认直连
        update_stats(|s| s.direct_packets += 1);
        Ok(xdp_action::XDP_PASS)
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
