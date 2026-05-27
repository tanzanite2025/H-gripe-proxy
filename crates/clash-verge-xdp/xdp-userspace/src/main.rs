/**
 * XDP 代理命令行工具
 */

use clap::{Parser, Subcommand};
use std::net::Ipv4Addr;
use xdp_userspace::{RouteAction, XdpProxyLoader};

#[derive(Parser)]
#[command(name = "xdp-proxy")]
#[command(about = "XDP-based high-performance proxy", long_about = None)]
struct Cli {
    /// Network interface to attach to
    #[arg(short, long, default_value = "eth0")]
    interface: String,

    /// Path to eBPF program
    #[arg(short, long)]
    ebpf_path: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the XDP proxy
    Start,

    /// Add a route rule
    AddRoute {
        /// Destination IP address
        dest_ip: Ipv4Addr,

        /// Action: pass, proxy, or reject
        #[arg(value_enum)]
        action: ActionArg,

        /// Proxy server IP (required for proxy action)
        #[arg(long)]
        proxy_ip: Option<Ipv4Addr>,

        /// Proxy server port (required for proxy action)
        #[arg(long)]
        proxy_port: Option<u16>,
    },

    /// Remove a route rule
    RemoveRoute {
        /// Destination IP address
        dest_ip: Ipv4Addr,
    },

    /// Show statistics
    Stats,

    /// Show connections
    Connections,

    /// Clear all routes
    ClearRoutes,

    /// Clear all connections
    ClearConnections,
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum ActionArg {
    Pass,
    Proxy,
    Reject,
}

impl From<ActionArg> for RouteAction {
    fn from(arg: ActionArg) -> Self {
        match arg {
            ActionArg::Pass => RouteAction::Pass,
            ActionArg::Proxy => RouteAction::Proxy,
            ActionArg::Reject => RouteAction::Reject,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let cli = Cli::parse();

    // 加载 eBPF 程序
    let ebpf_bytes = if let Some(path) = cli.ebpf_path {
        std::fs::read(path)?
    } else {
        // 使用嵌入的 eBPF 程序
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../xdp-ebpf/target/bpfel-unknown-none/release/xdp-ebpf"
        ))
        .to_vec()
    };

    let mut loader = XdpProxyLoader::new(&ebpf_bytes, &cli.interface)?;

    match cli.command {
        Commands::Start => {
            loader.attach()?;
            println!("XDP proxy started on {}", cli.interface);
            println!("Press Ctrl+C to stop...");

            // 等待信号
            tokio::signal::ctrl_c().await?;
            println!("\nStopping XDP proxy...");
        }

        Commands::AddRoute {
            dest_ip,
            action,
            proxy_ip,
            proxy_port,
        } => {
            loader.attach()?;
            loader.add_route(dest_ip, action.into(), proxy_ip, proxy_port)?;
            println!("Route added successfully");
        }

        Commands::RemoveRoute { dest_ip } => {
            loader.attach()?;
            loader.remove_route(dest_ip)?;
            println!("Route removed successfully");
        }

        Commands::Stats => {
            loader.attach()?;
            let stats = loader.get_stats()?;
            println!("Statistics:");
            println!("  Total packets:    {}", stats.total_packets);
            println!("  Proxied packets:  {}", stats.proxied_packets);
            println!("  Direct packets:   {}", stats.direct_packets);
            println!("  Rejected packets: {}", stats.rejected_packets);
            println!("  Errors:           {}", stats.errors);
        }

        Commands::Connections => {
            loader.attach()?;
            let connections = loader.get_connections()?;
            println!("Active connections: {}", connections.len());
            for (key, state) in connections.iter().take(10) {
                println!(
                    "  {}:{} -> {}:{} (proto: {}) - {} packets, {} bytes",
                    Ipv4Addr::from(key.src_ip),
                    key.src_port,
                    Ipv4Addr::from(key.dst_ip),
                    key.dst_port,
                    key.protocol,
                    state.packets,
                    state.bytes
                );
            }
            if connections.len() > 10 {
                println!("  ... and {} more", connections.len() - 10);
            }
        }

        Commands::ClearRoutes => {
            loader.attach()?;
            loader.clear_routes()?;
            println!("All routes cleared");
        }

        Commands::ClearConnections => {
            loader.attach()?;
            loader.clear_connections()?;
            println!("All connections cleared");
        }
    }

    Ok(())
}
