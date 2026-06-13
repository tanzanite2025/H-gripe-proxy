<h1 align="center">
  <img src="Meta.png" alt="Meta Kennel" width="200">
  <br>Tanzanite Mihomo Optimized Kernel<br>
</h1>

<h3 align="center">Casey's independently maintained MIHOMO kernel for Clash Verge Optimized.</h3>

## Features

- Local HTTP/HTTPS/SOCKS server with authentication support
- VMess, VLESS, Shadowsocks, Trojan, Snell, TUIC, Hysteria protocol support
- Built-in DNS server that aims to minimize DNS pollution attack impact, supports DoH/DoT upstream and fake IP.
- Rules based off domains, GEOIP, IPCIDR or Process to forward packets to different nodes
- Remote groups allow users to implement powerful rules. Supports automatic fallback, load balancing or auto select node
  based off latency
- Remote providers, allowing users to get node lists remotely instead of hard-coding in config
- Netfilter TCP redirecting. Deploy Mihomo on your Internet gateway with `iptables`.
- Comprehensive HTTP RESTful API controller

## Dashboard

This kernel does not download a dashboard by default. Desktop builds should
package their UI locally; standalone users can configure `external-ui` or
`external-ui-url` explicitly when they need a remote dashboard bundle.

## Configration example

Configuration example is located at [/docs/config.yaml](docs/config.yaml).

## Docs

Project-specific configuration docs are kept in [docs/config.yaml](docs/config.yaml).

## For development

Requirements:
[Go 1.20 or newer](https://go.dev/dl/)

Build mihomo:

```shell
git clone https://github.com/tanzanite2025/clash-verge-optimized.git
cd clash-verge-optimized/mihomo && go mod download
go build
```

Set go proxy if a connection to GitHub is not possible:

```shell
go env -w GOPROXY=https://goproxy.io,direct
```

Build with gvisor tun stack:

```shell
go build -tags with_gvisor
```

### IPTABLES configuration

Work on Linux OS which supported `iptables`

```yaml
# Enable the TPROXY listener
tproxy-port: 9898

iptables:
  enable: true # default is false
  inbound-interface: eth0 # detect the inbound interface, default is 'lo'
```

## Debugging

Check [wiki](https://wiki.metacubex.one/api/#debug) to get an instruction on using debug
API.

## Credits

- [Dreamacro/clash](https://github.com/Dreamacro/clash)
- [SagerNet/sing-box](https://github.com/SagerNet/sing-box)
- [riobard/go-shadowsocks2](https://github.com/riobard/go-shadowsocks2)
- [v2ray/v2ray-core](https://github.com/v2ray/v2ray-core)
- [WireGuard/wireguard-go](https://github.com/WireGuard/wireguard-go)
- [yaling888/clash-plus-pro](https://github.com/yaling888/clash)

## License

This software is released under the GPL-3.0 license.
