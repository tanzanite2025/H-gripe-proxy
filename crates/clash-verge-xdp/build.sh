#!/bin/bash
set -e

echo "Building Clash Verge XDP..."

# 检查工具链
if ! command -v bpf-linker &> /dev/null; then
    echo "Error: bpf-linker not found. Install it with:"
    echo "  cargo install bpf-linker"
    exit 1
fi

# 检查目标
if ! rustup target list | grep -q "bpfel-unknown-none (installed)"; then
    echo "Adding bpfel-unknown-none target..."
    rustup target add bpfel-unknown-none
fi

# 编译 eBPF 程序
echo "Building eBPF program..."
cd xdp-ebpf
cargo build --release --target bpfel-unknown-none
cd ..

# 编译用户态程序
echo "Building userspace program..."
cd xdp-userspace
cargo build --release
cd ..

echo "Build complete!"
echo ""
echo "eBPF program: xdp-ebpf/target/bpfel-unknown-none/release/xdp-ebpf"
echo "Userspace program: xdp-userspace/target/release/xdp-proxy"
echo ""
echo "Run with:"
echo "  sudo ./xdp-userspace/target/release/xdp-proxy --interface eth0 start"
