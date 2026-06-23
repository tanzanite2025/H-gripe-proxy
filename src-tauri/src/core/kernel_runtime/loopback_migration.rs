use super::*;

pub async fn mihomo_kernel_isolated_listener_preflight(
    port: Option<u16>,
) -> Result<KernelIsolatedListenerPreflightReport> {
    let requested_host: String = "127.0.0.1".into();
    let requested_port = port.unwrap_or(DEFAULT_ISOLATED_TEST_LISTENER_PORT);
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();
    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))?;
    let runtime_ports = kernel_runtime_ports(config);
    let verge = Config::verge().await.latest_arc();
    let system_proxy_enabled = verge.enable_system_proxy.unwrap_or(false);
    let tun_enabled = verge.enable_tun_mode.unwrap_or(false);
    let conflicts_with_runtime_port = runtime_ports.values().any(|port| *port == requested_port);
    let available = kernel_loopback_port_available(requested_port);
    let mut notes = vec!["loopback-only candidate; preflight does not start a listener".into()];
    if conflicts_with_runtime_port {
        notes.push("candidate port matches an existing Mihomo runtime port".into());
    }
    if !available {
        notes.push("candidate port is unavailable on 127.0.0.1".into());
    }
    let mut blockers =
        vec!["Rust isolated listener remains opt-in only; this preflight must not start forwarding".into()];
    if conflicts_with_runtime_port {
        blockers.push("choose a port that does not overlap Mihomo runtime listeners".into());
    }
    if !available {
        blockers.push("choose an unused loopback port before enabling a test listener".into());
    }
    let mut warnings = Vec::new();
    if system_proxy_enabled {
        warnings.push("system proxy is currently enabled; R3 listener must not become the default proxy".into());
    }
    if tun_enabled {
        warnings.push("TUN is currently enabled; R3 listener must not attach to transparent proxy routing".into());
    }

    Ok(KernelIsolatedListenerPreflightReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "isolated-test-listener-preflight".into(),
        kernel_area: "listener".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        requested_host,
        requested_port,
        can_start_after_opt_in: available && !conflicts_with_runtime_port,
        port_check: KernelIsolatedListenerPortCheck {
            host: "127.0.0.1".into(),
            port: requested_port,
            available,
            conflicts_with_runtime_port,
            notes,
        },
        runtime_ports,
        system_proxy_enabled,
        tun_enabled,
        blockers,
        warnings,
        facts: vec![
            "preflight reads runtime listener configuration and checks loopback port availability".into(),
            "R3 may only use a bounded loopback test path with Mihomo fallback preserved".into(),
        ],
        next_safe_batch: "loopback-test-listener-opt-in".into(),
    })
}

pub async fn mihomo_kernel_loopback_dns_preflight(port: Option<u16>) -> Result<KernelLoopbackDnsPreflightReport> {
    let requested_port = port.unwrap_or(DEFAULT_LOOPBACK_DNS_PREFLIGHT_PORT);
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();
    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))?;
    let runtime_dns_present = config.get("dns").is_some();
    let verge = Config::verge().await.latest_arc();
    let app_dns_settings_enabled = verge.enable_dns_settings.unwrap_or(false);
    let system_proxy_enabled = verge.enable_system_proxy.unwrap_or(false);
    let tun_enabled = verge.enable_tun_mode.unwrap_or(false);
    let udp_available = kernel_loopback_udp_port_available(requested_port);
    let tcp_available = kernel_loopback_port_available(requested_port);
    let mut notes = vec!["loopback DNS candidate; preflight does not bind persistent sockets".into()];
    if !udp_available {
        notes.push("candidate UDP port is unavailable on 127.0.0.1".into());
    }
    if !tcp_available {
        notes.push("candidate TCP port is unavailable on 127.0.0.1".into());
    }

    let mut blockers = vec![
        "loopback DNS remains opt-in only and must not replace default Mihomo DNS".into(),
        "R3 DNS preflight must not patch Mihomo config, TUN, system proxy, or forwarding".into(),
    ];
    if !udp_available {
        blockers.push("choose an unused loopback UDP port before enabling loopback DNS smoke evidence".into());
    }
    if !tcp_available {
        blockers.push("choose an unused loopback TCP port before enabling loopback DNS smoke evidence".into());
    }

    let mut warnings = Vec::new();
    if app_dns_settings_enabled {
        warnings.push("app DNS settings are enabled; loopback DNS must still remain an isolated test path".into());
    }
    if system_proxy_enabled {
        warnings.push("system proxy is enabled; loopback DNS must not become a default proxy dependency".into());
    }
    if tun_enabled {
        warnings.push("TUN is enabled; loopback DNS must not attach to transparent proxy routing".into());
    }

    Ok(KernelLoopbackDnsPreflightReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-dns-preflight".into(),
        kernel_area: "dns".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        requested_host: ISOLATED_TEST_LISTENER_HOST.into(),
        requested_port,
        can_start_after_opt_in: udp_available && tcp_available,
        port_check: KernelLoopbackDnsPortCheck {
            host: ISOLATED_TEST_LISTENER_HOST.into(),
            port: requested_port,
            udp_available,
            tcp_available,
            notes,
        },
        runtime_dns_present,
        app_dns_settings_enabled,
        system_proxy_enabled,
        tun_enabled,
        default_route: false,
        forwards_traffic: false,
        mihomo_fallback: true,
        blockers,
        warnings,
        facts: vec![
            "preflight checks loopback UDP and TCP DNS candidate ports without keeping sockets open".into(),
            "default Mihomo DNS remains production owner until a dedicated opt-in execution batch".into(),
            "loopback DNS must not mutate runtime config, system proxy, TUN, or outbound forwarding".into(),
        ],
        next_safe_batch: "loopback-dns-smoke-evidence".into(),
    })
}

pub async fn mihomo_kernel_loopback_dns_smoke_evidence(
    port: Option<u16>,
) -> Result<KernelLoopbackDnsSmokeEvidenceReport> {
    let requested_port = port.unwrap_or(DEFAULT_LOOPBACK_DNS_PREFLIGHT_PORT);
    let preflight = mihomo_kernel_loopback_dns_preflight(Some(requested_port)).await?;
    let before_runtime_config = kernel_runtime_config_snapshot().await?;
    let before_verge = Config::verge().await.latest_arc();
    let before_system_proxy = before_verge.enable_system_proxy.unwrap_or(false);
    let before_tun = before_verge.enable_tun_mode.unwrap_or(false);
    let mut warnings = preflight.warnings.clone();

    if !preflight.can_start_after_opt_in {
        return Ok(kernel_loopback_dns_smoke_report(
            requested_port,
            false,
            false,
            None,
            true,
            true,
            true,
            preflight.blockers,
            warnings,
        ));
    }

    let server = TokioUdpSocket::bind((ISOLATED_TEST_LISTENER_HOST, requested_port)).await?;
    let server_task = tokio::spawn(async move {
        let mut request = [0_u8; 512];
        let (request_len, peer) = timeout(Duration::from_secs(2), server.recv_from(&mut request)).await??;
        if let Some(response) = build_loopback_dns_smoke_response(&request[..request_len]) {
            server.send_to(&response, peer).await?;
            Ok::<bool, anyhow::Error>(true)
        } else {
            Ok(false)
        }
    });

    let client = TokioUdpSocket::bind((ISOLATED_TEST_LISTENER_HOST, 0)).await?;
    let query = build_loopback_dns_smoke_query(LOOPBACK_DNS_SMOKE_QUERY);
    client
        .send_to(&query, (ISOLATED_TEST_LISTENER_HOST, requested_port))
        .await?;
    let mut response = [0_u8; 512];
    let response_len = timeout(Duration::from_secs(2), client.recv(&mut response)).await??;
    let response_address = parse_loopback_dns_smoke_response(&response[..response_len]);
    let server_responded = server_task.await??;
    let local_response_received = server_responded && response_address.is_some();

    let after_runtime_config = kernel_runtime_config_snapshot().await?;
    let after_verge = Config::verge().await.latest_arc();
    let system_proxy_unchanged = before_system_proxy == after_verge.enable_system_proxy.unwrap_or(false);
    let tun_unchanged = before_tun == after_verge.enable_tun_mode.unwrap_or(false);
    let runtime_config_unchanged = before_runtime_config == after_runtime_config;
    let mut blockers = Vec::new();
    if response_address.as_deref() != Some("127.0.0.1") {
        blockers.push("loopback DNS smoke response did not return 127.0.0.1".into());
    }
    if !system_proxy_unchanged {
        blockers.push("system proxy setting changed during DNS smoke evidence".into());
    }
    if !tun_unchanged {
        blockers.push("TUN setting changed during DNS smoke evidence".into());
    }
    if !runtime_config_unchanged {
        blockers.push("runtime config changed during DNS smoke evidence".into());
    }
    warnings.push(
        "DNS smoke evidence uses a synthetic kernel-smoke.invalid query and must not be used as production DNS".into(),
    );

    Ok(kernel_loopback_dns_smoke_report(
        requested_port,
        true,
        local_response_received,
        response_address,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        blockers,
        warnings,
    ))
}

pub async fn mihomo_kernel_loopback_forwarding_preflight(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> Result<KernelLoopbackForwardingPreflightReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let listener_available = kernel_loopback_port_available(listener_port);
    let target_available = kernel_loopback_port_available(target_port);
    let verge = Config::verge().await.latest_arc();
    let system_proxy_enabled = verge.enable_system_proxy.unwrap_or(false);
    let tun_enabled = verge.enable_tun_mode.unwrap_or(false);
    let mut notes = vec![
        "preflight checks only candidate loopback TCP ports and does not keep sockets open".into(),
        "future smoke target must be a synthetic local responder, not a real outbound adapter".into(),
    ];
    if listener_port == target_port {
        notes.push("listener and target ports must differ for a forwarding smoke path".into());
    }
    if !listener_available {
        notes.push("candidate listener port is unavailable on 127.0.0.1".into());
    }
    if !target_available {
        notes.push("candidate target port is unavailable on 127.0.0.1".into());
    }

    let mut blockers = vec![
        "loopback forwarding remains opt-in only and must not become a system proxy/default route".into(),
        "future smoke evidence must forward only to a synthetic loopback target, never Mihomo/outbound adapters".into(),
        "TUN, transparent proxy, protocol stack replacement, and production forwarding remain blocked".into(),
    ];
    if listener_port == target_port {
        blockers.push("choose different listener and target ports before forwarding smoke evidence".into());
    }
    if !listener_available {
        blockers.push("choose an unused loopback listener TCP port before forwarding smoke evidence".into());
    }
    if !target_available {
        blockers.push("choose an unused loopback target TCP port before forwarding smoke evidence".into());
    }

    let mut warnings = Vec::new();
    if system_proxy_enabled {
        warnings.push("system proxy is enabled; loopback forwarding smoke must not register as a proxy".into());
    }
    if tun_enabled {
        warnings.push("TUN is enabled; loopback forwarding smoke must not attach to transparent proxy routing".into());
    }

    Ok(KernelLoopbackForwardingPreflightReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-forwarding-preflight".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        requested_host: ISOLATED_TEST_LISTENER_HOST.into(),
        listener_port,
        target_port,
        can_start_after_opt_in: listener_port != target_port && listener_available && target_available,
        port_check: KernelLoopbackForwardingPortCheck {
            host: ISOLATED_TEST_LISTENER_HOST.into(),
            listener_port,
            target_port,
            listener_available,
            target_available,
            target_loopback_only: true,
            notes,
        },
        system_proxy_enabled,
        tun_enabled,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_allowed: false,
        mihomo_fallback: true,
        blockers,
        warnings,
        facts: vec![
            "preflight only checks local port readiness and safety gates".into(),
            "forwarding smoke evidence must stay inside 127.0.0.1 listener -> 127.0.0.1 target".into(),
            "real adapter dialing, TUN, system proxy, and default route changes are still forbidden".into(),
        ],
        next_safe_batch: "loopback-forwarding-smoke-evidence".into(),
    })
}

pub async fn mihomo_kernel_loopback_forwarding_smoke_evidence(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> Result<KernelLoopbackForwardingSmokeEvidenceReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let preflight = mihomo_kernel_loopback_forwarding_preflight(Some(listener_port), Some(target_port)).await?;
    let before_runtime_config = kernel_runtime_config_snapshot().await?;
    let before_verge = Config::verge().await.latest_arc();
    let before_system_proxy = before_verge.enable_system_proxy.unwrap_or(false);
    let before_tun = before_verge.enable_tun_mode.unwrap_or(false);
    let mut warnings = preflight.warnings.clone();

    if !preflight.can_start_after_opt_in {
        return Ok(kernel_loopback_forwarding_smoke_report(
            listener_port,
            target_port,
            false,
            false,
            None,
            0,
            0,
            true,
            true,
            true,
            preflight.blockers,
            warnings,
        ));
    }

    let target = TokioTcpListener::bind((ISOLATED_TEST_LISTENER_HOST, target_port)).await?;
    let listener = TokioTcpListener::bind((ISOLATED_TEST_LISTENER_HOST, listener_port)).await?;

    let target_task = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(2), target.accept()).await??;
        let mut request = [0_u8; 512];
        let request_len = timeout(Duration::from_secs(2), stream.read(&mut request)).await??;
        let received = std::str::from_utf8(&request[..request_len])
            .map(|request| request.contains("GET /kernel-forwarding-smoke"))
            .unwrap_or(false);
        stream
            .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .await?;
        stream.shutdown().await?;
        Ok::<bool, anyhow::Error>(received)
    });

    let listener_task = tokio::spawn(async move {
        let (mut inbound, _) = timeout(Duration::from_secs(2), listener.accept()).await??;
        let mut outbound = timeout(
            Duration::from_secs(2),
            TcpStream::connect((ISOLATED_TEST_LISTENER_HOST, target_port)),
        )
        .await??;
        let mut request = [0_u8; 512];
        let request_len = timeout(Duration::from_secs(2), inbound.read(&mut request)).await??;
        outbound.write_all(&request[..request_len]).await?;
        let mut response = [0_u8; 512];
        let response_len = timeout(Duration::from_secs(2), outbound.read(&mut response)).await??;
        inbound.write_all(&response[..response_len]).await?;
        inbound.shutdown().await?;
        Ok::<(u64, u64), anyhow::Error>((request_len as u64, response_len as u64))
    });

    let mut client = timeout(
        Duration::from_secs(2),
        TcpStream::connect((ISOLATED_TEST_LISTENER_HOST, listener_port)),
    )
    .await??;
    client
        .write_all(b"GET /kernel-forwarding-smoke HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .await?;
    let mut response = [0_u8; 512];
    let response_len = timeout(Duration::from_secs(2), client.read(&mut response)).await??;
    let response = std::string::String::from_utf8_lossy(&response[..response_len]);
    let response_status = response.lines().next().map(Into::into);
    let (bytes_from_client, bytes_from_target) = listener_task.await??;
    let target_received = target_task.await??;

    let after_runtime_config = kernel_runtime_config_snapshot().await?;
    let after_verge = Config::verge().await.latest_arc();
    let system_proxy_unchanged = before_system_proxy == after_verge.enable_system_proxy.unwrap_or(false);
    let tun_unchanged = before_tun == after_verge.enable_tun_mode.unwrap_or(false);
    let runtime_config_unchanged = before_runtime_config == after_runtime_config;
    let mut blockers = Vec::new();
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("loopback forwarding smoke response did not return HTTP 204".into());
    }
    if !target_received {
        blockers.push("synthetic target did not receive the forwarding smoke request".into());
    }
    if !system_proxy_unchanged {
        blockers.push("system proxy setting changed during forwarding smoke evidence".into());
    }
    if !tun_unchanged {
        blockers.push("TUN setting changed during forwarding smoke evidence".into());
    }
    if !runtime_config_unchanged {
        blockers.push("runtime config changed during forwarding smoke evidence".into());
    }
    warnings.push(
        "forwarding smoke evidence uses only synthetic loopback endpoints and must not be connected to real adapters"
            .into(),
    );

    Ok(kernel_loopback_forwarding_smoke_report(
        listener_port,
        target_port,
        true,
        target_received,
        response_status,
        bytes_from_client,
        bytes_from_target,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        blockers,
        warnings,
    ))
}

pub async fn mihomo_kernel_loopback_forwarding_rollback_drill(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> Result<KernelLoopbackForwardingRollbackDrillReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let before_runtime_config = kernel_runtime_config_snapshot().await?;
    let before_verge = Config::verge().await.latest_arc();
    let before_system_proxy = before_verge.enable_system_proxy.unwrap_or(false);
    let before_tun = before_verge.enable_tun_mode.unwrap_or(false);

    let smoke = mihomo_kernel_loopback_forwarding_smoke_evidence(Some(listener_port), Some(target_port)).await?;
    let post_preflight = mihomo_kernel_loopback_forwarding_preflight(Some(listener_port), Some(target_port)).await?;
    let ports_released = post_preflight.can_start_after_opt_in;
    let after_runtime_config = kernel_runtime_config_snapshot().await?;
    let after_verge = Config::verge().await.latest_arc();
    let system_proxy_unchanged = before_system_proxy == after_verge.enable_system_proxy.unwrap_or(false);
    let tun_unchanged = before_tun == after_verge.enable_tun_mode.unwrap_or(false);
    let runtime_config_unchanged = before_runtime_config == after_runtime_config;

    let mut blockers = Vec::new();
    if !smoke.passed {
        blockers.push("loopback forwarding smoke evidence did not pass before rollback drill".into());
    }
    if !ports_released {
        blockers.push("loopback forwarding smoke ports were not released after the drill".into());
    }
    if !system_proxy_unchanged {
        blockers.push("system proxy setting changed during forwarding rollback drill".into());
    }
    if !tun_unchanged {
        blockers.push("TUN setting changed during forwarding rollback drill".into());
    }
    if !runtime_config_unchanged {
        blockers.push("runtime config changed during forwarding rollback drill".into());
    }

    let passed = blockers.is_empty();
    Ok(KernelLoopbackForwardingRollbackDrillReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-forwarding-rollback-drill".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: true,
        live_execution_allowed: true,
        listener_port,
        target_port,
        smoke_passed: smoke.passed,
        ports_released,
        post_preflight,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed,
        blockers,
        warnings: vec!["rollback drill remains synthetic loopback-only and does not exercise real adapters".into()],
        facts: vec![
            "drill runs loopback forwarding smoke evidence and immediately re-runs preflight".into(),
            "post-preflight must show listener and target ports are available again".into(),
            "runtime config, system proxy, and TUN settings are compared before and after".into(),
        ],
        next_safe_batch: "loopback-forwarding-leak-check".into(),
    })
}

pub async fn mihomo_kernel_loopback_forwarding_leak_check(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> Result<KernelLoopbackForwardingLeakCheckReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let preflight = mihomo_kernel_loopback_forwarding_preflight(Some(listener_port), Some(target_port)).await?;
    let isolated_status = mihomo_kernel_isolated_test_listener_status().await;
    let listener_port_released = preflight.port_check.listener_available;
    let target_port_released = preflight.port_check.target_available;
    let isolated_test_listener_running = isolated_status.running;
    let mut blockers = Vec::new();
    if !listener_port_released {
        blockers.push("loopback forwarding listener port is still occupied".into());
    }
    if !target_port_released {
        blockers.push("loopback forwarding target port is still occupied".into());
    }
    if isolated_test_listener_running {
        blockers.push("isolated test listener is still running during forwarding leak check".into());
    }
    let passed = blockers.is_empty();

    Ok(KernelLoopbackForwardingLeakCheckReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-forwarding-leak-check".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        listener_port,
        target_port,
        listener_port_released,
        target_port_released,
        isolated_test_listener_running,
        preflight,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed,
        blockers,
        warnings: vec!["leak check is local state evidence only and does not prove platform routing safety".into()],
        facts: vec![
            "checks forwarding smoke listener and target ports are available after rollback drill".into(),
            "checks the isolated test listener persistent state is not running".into(),
            "does not bind persistent sockets, dial adapters, or mutate runtime state".into(),
        ],
        next_safe_batch: "loopback-platform-matrix".into(),
    })
}

pub async fn mihomo_kernel_loopback_platform_matrix(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> Result<KernelLoopbackPlatformMatrixReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let leak_check = mihomo_kernel_loopback_forwarding_leak_check(Some(listener_port), Some(target_port)).await?;
    let current_platform = std::env::consts::OS;
    let current_arch = std::env::consts::ARCH;
    let required_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();
    let current_platform_supported = LOOPBACK_PLATFORM_MATRIX_PLATFORMS.contains(&current_platform);
    let covered_platforms = if current_platform_supported {
        vec![current_platform.into()]
    } else {
        Vec::new()
    };
    let pending_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .filter(|platform| **platform != current_platform)
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();
    let current_platform_passed = current_platform_supported && leak_check.passed;

    let rows = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| {
            let is_current_platform = *platform == current_platform;
            if is_current_platform {
                let mut facts = leak_check.facts.clone();
                facts.push(
                    format!("recorded loopback forwarding leak evidence on {current_platform}/{current_arch}").into(),
                );

                KernelLoopbackPlatformMatrixRow {
                    platform: (*platform).into(),
                    current_platform: true,
                    evidence_status: if leak_check.passed {
                        "observed".into()
                    } else {
                        "blocked".into()
                    },
                    listener_port_released: Some(leak_check.listener_port_released),
                    target_port_released: Some(leak_check.target_port_released),
                    isolated_test_listener_stopped: Some(!leak_check.isolated_test_listener_running),
                    default_route: leak_check.default_route,
                    forwards_traffic: leak_check.forwards_traffic,
                    outbound_adapters_used: leak_check.outbound_adapters_used,
                    mihomo_fallback: leak_check.mihomo_fallback,
                    blockers: leak_check.blockers.clone(),
                    facts,
                }
            } else {
                KernelLoopbackPlatformMatrixRow {
                    platform: (*platform).into(),
                    current_platform: false,
                    evidence_status: "pending".into(),
                    listener_port_released: None,
                    target_port_released: None,
                    isolated_test_listener_stopped: None,
                    default_route: false,
                    forwards_traffic: false,
                    outbound_adapters_used: false,
                    mihomo_fallback: true,
                    blockers: vec![
                        format!("run the loopback platform matrix on {platform} before expanded opt-in").into(),
                    ],
                    facts: vec!["pending platform row is a placeholder and records no runtime evidence".into()],
                }
            }
        })
        .collect::<Vec<KernelLoopbackPlatformMatrixRow>>();

    let mut blockers = vec![
        "R4 expanded opt-in remains blocked until Windows, macOS, and Linux matrix rows are observed".into(),
        "platform-specific rollback drills and hold-window evidence are still required".into(),
    ];
    if !current_platform_supported {
        blockers.push(format!("current platform {current_platform} is not in the required matrix").into());
    }
    if !leak_check.passed {
        blockers.extend(leak_check.blockers.clone());
    }

    let mut warnings = leak_check.warnings.clone();
    warnings.push("platform matrix is read-only evidence and does not allow real adapter/TUN/protocol cutover".into());

    Ok(KernelLoopbackPlatformMatrixReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-platform-matrix".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: current_platform.into(),
        current_arch: current_arch.into(),
        listener_port,
        target_port,
        required_platforms,
        covered_platforms,
        pending_platforms,
        current_platform_passed,
        expanded_opt_in_allowed: false,
        leak_check,
        rows,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: current_platform_passed,
        blockers,
        warnings,
        facts: vec![
            "wraps loopback forwarding leak evidence with a required platform matrix row".into(),
            "records only the current platform; other platform rows stay pending until run there".into(),
            "keeps R4 expanded opt-in blocked until matrix, rollback, and hold-window evidence exist".into(),
        ],
        next_safe_batch: "loopback-hold-window".into(),
    })
}

pub async fn mihomo_kernel_loopback_hold_window(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
) -> Result<KernelLoopbackHoldWindowReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let platform_matrix = mihomo_kernel_loopback_platform_matrix(Some(listener_port), Some(target_port)).await?;
    let observed_at_epoch_ms = current_epoch_ms();
    let hold_started_at_epoch_ms = hold_started_at_epoch_ms.unwrap_or(observed_at_epoch_ms);
    let hold_start_in_future = hold_started_at_epoch_ms > observed_at_epoch_ms;
    let elapsed_hold_seconds = observed_at_epoch_ms
        .saturating_sub(hold_started_at_epoch_ms)
        .saturating_div(1000);
    let current_platform_hold_window_satisfied =
        !hold_start_in_future && elapsed_hold_seconds >= LOOPBACK_HOLD_WINDOW_MIN_SECONDS;

    let rows = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| {
            let is_current_platform = *platform == platform_matrix.current_platform;
            if is_current_platform {
                let mut blockers = Vec::new();
                if !platform_matrix.current_platform_passed {
                    blockers.push("platform matrix evidence must pass before hold-window evidence is usable".into());
                }
                if hold_start_in_future {
                    blockers.push("hold window start timestamp is later than the observation timestamp".into());
                }
                if !current_platform_hold_window_satisfied {
                    blockers.push(format!(
                        "observe at least {LOOPBACK_HOLD_WINDOW_MIN_SECONDS} second(s) before treating hold-window evidence as satisfied"
                    ).into());
                }

                KernelLoopbackHoldWindowRow {
                    platform: (*platform).into(),
                    current_platform: true,
                    evidence_status: if !platform_matrix.current_platform_passed || hold_start_in_future {
                        "blocked".into()
                    } else if current_platform_hold_window_satisfied {
                        "observed".into()
                    } else {
                        "holding".into()
                    },
                    hold_started_at_epoch_ms: Some(hold_started_at_epoch_ms),
                    observed_at_epoch_ms: Some(observed_at_epoch_ms),
                    minimum_hold_seconds: LOOPBACK_HOLD_WINDOW_MIN_SECONDS,
                    elapsed_hold_seconds: Some(elapsed_hold_seconds),
                    hold_window_satisfied: current_platform_hold_window_satisfied,
                    platform_matrix_passed: Some(platform_matrix.current_platform_passed),
                    leak_check_passed: Some(platform_matrix.leak_check.passed),
                    default_route: false,
                    forwards_traffic: false,
                    outbound_adapters_used: false,
                    mihomo_fallback: true,
                    blockers,
                    facts: vec![
                        format!(
                            "recorded loopback hold-window observation on {}/{}",
                            platform_matrix.current_platform, platform_matrix.current_arch
                        )
                        .into(),
                        "hold-window evidence is read-only and does not keep sockets or listeners open".into(),
                    ],
                }
            } else {
                KernelLoopbackHoldWindowRow {
                    platform: (*platform).into(),
                    current_platform: false,
                    evidence_status: "pending".into(),
                    hold_started_at_epoch_ms: None,
                    observed_at_epoch_ms: None,
                    minimum_hold_seconds: LOOPBACK_HOLD_WINDOW_MIN_SECONDS,
                    elapsed_hold_seconds: None,
                    hold_window_satisfied: false,
                    platform_matrix_passed: None,
                    leak_check_passed: None,
                    default_route: false,
                    forwards_traffic: false,
                    outbound_adapters_used: false,
                    mihomo_fallback: true,
                    blockers: vec![
                        format!("run loopback hold-window evidence on {platform} before expanded opt-in").into(),
                    ],
                    facts: vec!["pending hold-window row records no runtime evidence".into()],
                }
            }
        })
        .collect::<Vec<KernelLoopbackHoldWindowRow>>();

    let covered_hold_platforms = rows
        .iter()
        .filter(|row| row.hold_window_satisfied)
        .map(|row| row.platform.clone())
        .collect::<Vec<String>>();
    let pending_hold_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .filter(|platform| !covered_hold_platforms.iter().any(|covered| covered == **platform))
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();

    let mut blockers = vec![
        "R4 expanded opt-in remains blocked until Windows, macOS, and Linux hold-window rows are observed".into(),
        "platform-specific rollback drills are still required before broader opt-in".into(),
    ];
    if !platform_matrix.current_platform_passed {
        blockers.push("current platform matrix evidence is not passing".into());
    }
    if hold_start_in_future {
        blockers.push("hold window start timestamp is later than the observation timestamp".into());
    }
    if !current_platform_hold_window_satisfied {
        blockers.push(
            format!("current platform hold window has not reached {LOOPBACK_HOLD_WINDOW_MIN_SECONDS} second(s)").into(),
        );
    }
    if !pending_hold_platforms.is_empty() {
        blockers.push(
            format!(
                "pending hold-window platform evidence: {}",
                pending_hold_platforms.join(", ")
            )
            .into(),
        );
    }

    let mut warnings = platform_matrix.warnings.clone();
    warnings
        .push("hold-window timestamps are evidence only and do not enable adapter/TUN/protocol/default cutover".into());

    Ok(KernelLoopbackHoldWindowReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-hold-window".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: platform_matrix.current_platform.clone(),
        current_arch: platform_matrix.current_arch.clone(),
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_at_epoch_ms,
        minimum_hold_seconds: LOOPBACK_HOLD_WINDOW_MIN_SECONDS,
        elapsed_hold_seconds,
        required_platforms: platform_matrix.required_platforms.clone(),
        covered_hold_platforms,
        pending_hold_platforms,
        current_platform_passed: platform_matrix.current_platform_passed,
        current_platform_hold_window_satisfied,
        expanded_opt_in_allowed: false,
        platform_matrix,
        rows,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: current_platform_hold_window_satisfied,
        blockers,
        warnings,
        facts: vec![
            "wraps loopback platform matrix evidence with a time-window observation".into(),
            "records the current platform only; other platforms remain pending until run there".into(),
            "keeps expanded opt-in blocked after hold-window evidence until platform rollback evidence exists".into(),
        ],
        next_safe_batch: "loopback-platform-rollback-drills".into(),
    })
}

pub async fn mihomo_kernel_loopback_platform_rollback_drills(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
) -> Result<KernelLoopbackPlatformRollbackDrillsReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let hold_window =
        mihomo_kernel_loopback_hold_window(Some(listener_port), Some(target_port), hold_started_at_epoch_ms).await?;
    let rollback_drill =
        mihomo_kernel_loopback_forwarding_rollback_drill(Some(listener_port), Some(target_port)).await?;
    let current_platform = hold_window.current_platform.clone();
    let current_arch = hold_window.current_arch.clone();
    let current_platform_supported = LOOPBACK_PLATFORM_MATRIX_PLATFORMS.contains(&current_platform.as_str());
    let current_platform_passed = current_platform_supported && rollback_drill.passed;
    let required_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();

    let rows = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| {
            let is_current_platform = *platform == current_platform;
            if is_current_platform {
                let mut facts = rollback_drill.facts.clone();
                facts.push(
                    format!("recorded loopback rollback drill evidence on {current_platform}/{current_arch}").into(),
                );

                KernelLoopbackPlatformRollbackDrillRow {
                    platform: (*platform).into(),
                    current_platform: true,
                    evidence_status: if rollback_drill.passed {
                        "observed".into()
                    } else {
                        "blocked".into()
                    },
                    smoke_passed: Some(rollback_drill.smoke_passed),
                    ports_released: Some(rollback_drill.ports_released),
                    system_proxy_unchanged: Some(rollback_drill.system_proxy_unchanged),
                    tun_unchanged: Some(rollback_drill.tun_unchanged),
                    runtime_config_unchanged: Some(rollback_drill.runtime_config_unchanged),
                    hold_window_satisfied: Some(hold_window.current_platform_hold_window_satisfied),
                    default_route: rollback_drill.default_route,
                    forwards_traffic: rollback_drill.forwards_traffic,
                    outbound_adapters_used: rollback_drill.outbound_adapters_used,
                    mihomo_fallback: rollback_drill.mihomo_fallback,
                    blockers: rollback_drill.blockers.clone(),
                    facts,
                }
            } else {
                KernelLoopbackPlatformRollbackDrillRow {
                    platform: (*platform).into(),
                    current_platform: false,
                    evidence_status: "pending".into(),
                    smoke_passed: None,
                    ports_released: None,
                    system_proxy_unchanged: None,
                    tun_unchanged: None,
                    runtime_config_unchanged: None,
                    hold_window_satisfied: None,
                    default_route: false,
                    forwards_traffic: false,
                    outbound_adapters_used: false,
                    mihomo_fallback: true,
                    blockers: vec![
                        format!("run loopback platform rollback drills on {platform} before expanded opt-in").into(),
                    ],
                    facts: vec!["pending rollback drill row records no runtime evidence".into()],
                }
            }
        })
        .collect::<Vec<KernelLoopbackPlatformRollbackDrillRow>>();

    let covered_rollback_platforms = if current_platform_passed {
        vec![current_platform.clone()]
    } else {
        Vec::new()
    };
    let pending_rollback_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .filter(|platform| !covered_rollback_platforms.iter().any(|covered| covered == **platform))
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();

    let mut blockers = vec![
        "R4 expanded opt-in remains blocked until Windows, macOS, and Linux rollback drill rows are observed".into(),
        "R4 expanded opt-in still requires an explicit decision and dedicated preflight".into(),
    ];
    if !current_platform_supported {
        blockers.push(format!("current platform {current_platform} is not in the required rollback matrix").into());
    }
    if !rollback_drill.passed {
        blockers.extend(rollback_drill.blockers.clone());
    }
    if !hold_window.current_platform_hold_window_satisfied {
        blockers.push("current platform hold-window evidence is not satisfied".into());
    }
    if !pending_rollback_platforms.is_empty() {
        blockers.push(
            format!(
                "pending rollback drill platform evidence: {}",
                pending_rollback_platforms.join(", ")
            )
            .into(),
        );
    }

    let mut warnings = rollback_drill.warnings.clone();
    warnings.extend(hold_window.warnings.clone());
    warnings.push(
        "platform rollback drills are still synthetic loopback-only evidence and do not permit real adapter/TUN/protocol cutover"
            .into(),
    );

    Ok(KernelLoopbackPlatformRollbackDrillsReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-platform-rollback-drills".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: true,
        live_execution_allowed: true,
        current_platform,
        current_arch,
        listener_port,
        target_port,
        required_platforms,
        covered_rollback_platforms,
        pending_rollback_platforms,
        current_platform_passed,
        expanded_opt_in_allowed: false,
        hold_window,
        rollback_drill,
        rows,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: current_platform_passed,
        blockers,
        warnings,
        facts: vec![
            "wraps loopback forwarding rollback drill evidence with required platform rows".into(),
            "records only the current platform; other platform rows stay pending until run there".into(),
            "keeps expanded opt-in blocked until a dedicated R4 preflight and explicit decision".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-preflight".into(),
    })
}

pub async fn mihomo_kernel_isolated_test_listener_status() -> KernelIsolatedTestListenerStatus {
    isolated_test_listener_status(Vec::new())
}

pub async fn mihomo_kernel_start_isolated_test_listener(port: Option<u16>) -> Result<KernelIsolatedTestListenerStatus> {
    if let Some(status) = isolated_test_listener_running_status() {
        return Ok(status);
    }

    let preflight = mihomo_kernel_isolated_listener_preflight(port).await?;
    if !preflight.can_start_after_opt_in {
        bail!(
            "isolated test listener preflight failed: {}",
            preflight
                .blockers
                .iter()
                .map(|blocker| blocker.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        );
    }

    let port = preflight.requested_port;
    let listener = TokioTcpListener::bind((ISOLATED_TEST_LISTENER_HOST, port)).await?;
    let accepted_connections = Arc::new(AtomicU64::new(0));
    let task_counter = accepted_connections.clone();
    let (stop_tx, mut stop_rx) = oneshot::channel();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                accepted = listener.accept() => {
                    let Ok((mut stream, _)) = accepted else {
                        break;
                    };
                    task_counter.fetch_add(1, Ordering::Relaxed);
                    tauri::async_runtime::spawn(async move {
                        let _ = stream.write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n").await;
                        let _ = stream.shutdown().await;
                    });
                }
            }
        }
    });

    let state = KernelIsolatedTestListenerState {
        port,
        started_at_epoch_ms: current_epoch_ms(),
        accepted_connections,
        stop_tx,
    };
    let mut guard = ISOLATED_TEST_LISTENER.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_some() {
        return Ok(isolated_test_listener_status(vec![
            "isolated test listener was already running".into(),
        ]));
    }
    *guard = Some(state);
    Ok(isolated_test_listener_status(Vec::new()))
}

pub async fn mihomo_kernel_stop_isolated_test_listener() -> KernelIsolatedTestListenerStatus {
    let state = ISOLATED_TEST_LISTENER.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(state) = state {
        let _ = state.stop_tx.send(());
        return isolated_test_listener_status(vec!["isolated test listener stopped".into()]);
    }
    isolated_test_listener_status(vec!["isolated test listener was not running".into()])
}

pub async fn mihomo_kernel_isolated_test_listener_smoke_evidence(
    port: Option<u16>,
) -> Result<KernelIsolatedTestListenerSmokeEvidenceReport> {
    let requested_port = port.unwrap_or(DEFAULT_ISOLATED_TEST_LISTENER_PORT);
    let before_status = mihomo_kernel_isolated_test_listener_status().await;
    let before_runtime_config = kernel_runtime_config_snapshot().await?;
    let before_verge = Config::verge().await.latest_arc();
    let before_system_proxy = before_verge.enable_system_proxy.unwrap_or(false);
    let before_tun = before_verge.enable_tun_mode.unwrap_or(false);

    if before_status.running {
        return Ok(kernel_listener_smoke_report(
            requested_port,
            false,
            None,
            before_status.accepted_connections,
            before_status.accepted_connections,
            false,
            false,
            true,
            true,
            true,
            vec!["isolated test listener is already running; smoke evidence did not take lifecycle ownership".into()],
            Vec::new(),
        ));
    }

    let start_status = mihomo_kernel_start_isolated_test_listener(Some(requested_port)).await?;
    let accepted_connections_before = start_status.accepted_connections;
    let mut warnings = start_status.warnings.clone();
    let mut blockers = Vec::new();
    if !start_status.running {
        blockers.push("isolated test listener did not enter running state".into());
    }

    let response_status = if start_status.running {
        match isolated_test_listener_smoke_request(requested_port).await {
            Ok(status) => Some(status),
            Err(err) => {
                blockers.push(format!("local smoke request failed: {err}").into());
                None
            }
        }
    } else {
        None
    };

    let after_request_status = mihomo_kernel_isolated_test_listener_status().await;
    let accepted_connections_after = after_request_status.accepted_connections;
    let status_incremented = accepted_connections_after > accepted_connections_before;
    if !status_incremented {
        blockers.push("accepted connection count did not increase after local request".into());
    }
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("local listener did not return HTTP 204 smoke response".into());
    }

    let stop_status = mihomo_kernel_stop_isolated_test_listener().await;
    warnings.extend(stop_status.warnings);
    let stopped_after_smoke = !stop_status.running;
    if !stopped_after_smoke {
        blockers.push("isolated test listener remained running after stop".into());
    }

    let after_runtime_config = kernel_runtime_config_snapshot().await?;
    let after_verge = Config::verge().await.latest_arc();
    let system_proxy_unchanged = before_system_proxy == after_verge.enable_system_proxy.unwrap_or(false);
    let tun_unchanged = before_tun == after_verge.enable_tun_mode.unwrap_or(false);
    let runtime_config_unchanged = before_runtime_config == after_runtime_config;
    if !system_proxy_unchanged {
        blockers.push("system proxy setting changed during smoke evidence".into());
    }
    if !tun_unchanged {
        blockers.push("TUN setting changed during smoke evidence".into());
    }
    if !runtime_config_unchanged {
        blockers.push("runtime config changed during smoke evidence".into());
    }

    Ok(kernel_listener_smoke_report(
        requested_port,
        true,
        response_status,
        accepted_connections_before,
        accepted_connections_after,
        status_incremented,
        stopped_after_smoke,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        blockers,
        warnings,
    ))
}

fn kernel_runtime_ports(config: &serde_yaml_ng::Mapping) -> BTreeMap<String, u16> {
    let mut ports = BTreeMap::new();
    for key in ["port", "socks-port", "mixed-port", "redir-port", "tproxy-port"] {
        if let Some(port) = kernel_runtime_port(config, key) {
            ports.insert(key.into(), port);
        }
    }
    ports
}

fn kernel_runtime_port(config: &serde_yaml_ng::Mapping, key: &str) -> Option<u16> {
    config
        .get(key)
        .and_then(serde_yaml_ng::Value::as_i64)
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port > 0)
}

fn kernel_loopback_port_available(port: u16) -> bool {
    port > 0 && StdTcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn kernel_loopback_udp_port_available(port: u16) -> bool {
    port > 0 && StdUdpSocket::bind((ISOLATED_TEST_LISTENER_HOST, port)).is_ok()
}

fn build_loopback_dns_smoke_query(domain: &str) -> Vec<u8> {
    let mut query = vec![0xca, 0xfe, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    for label in domain.split('.') {
        query.push(label.len().min(63) as u8);
        query.extend_from_slice(label.as_bytes().get(..label.len().min(63)).unwrap_or_default());
    }
    query.extend_from_slice(&[0x00, 0x00, 0x01, 0x00, 0x01]);
    query
}

fn build_loopback_dns_smoke_response(query: &[u8]) -> Option<Vec<u8>> {
    if query.len() < 12 {
        return None;
    }
    let question_end = skip_dns_question(query, 12)?;
    let mut response = Vec::with_capacity(question_end + 16);
    response.extend_from_slice(&query[0..2]);
    response.extend_from_slice(&[0x81, 0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
    response.extend_from_slice(&query[12..question_end]);
    response.extend_from_slice(&[
        0xc0, 0x0c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 127, 0, 0, 1,
    ]);
    Some(response)
}

fn parse_loopback_dns_smoke_response(response: &[u8]) -> Option<String> {
    if response.len() < 12 {
        return None;
    }
    let question_count = u16::from_be_bytes([response[4], response[5]]);
    let answer_count = u16::from_be_bytes([response[6], response[7]]);
    if answer_count == 0 {
        return None;
    }
    let mut offset = 12;
    for _ in 0..question_count {
        offset = skip_dns_question(response, offset)?;
    }
    for _ in 0..answer_count {
        offset = skip_dns_name(response, offset)?;
        if offset + 10 > response.len() {
            return None;
        }
        let record_type = u16::from_be_bytes([response[offset], response[offset + 1]]);
        let record_class = u16::from_be_bytes([response[offset + 2], response[offset + 3]]);
        let data_len = u16::from_be_bytes([response[offset + 8], response[offset + 9]]) as usize;
        offset += 10;
        if offset + data_len > response.len() {
            return None;
        }
        if record_type == 1 && record_class == 1 && data_len == 4 {
            return Some(
                format!(
                    "{}.{}.{}.{}",
                    response[offset],
                    response[offset + 1],
                    response[offset + 2],
                    response[offset + 3]
                )
                .into(),
            );
        }
        offset += data_len;
    }
    None
}

fn skip_dns_question(packet: &[u8], offset: usize) -> Option<usize> {
    skip_dns_name(packet, offset).and_then(|offset| offset.checked_add(4).filter(|end| *end <= packet.len()))
}

fn skip_dns_name(packet: &[u8], mut offset: usize) -> Option<usize> {
    loop {
        let len = *packet.get(offset)?;
        if len & 0xc0 == 0xc0 {
            return offset.checked_add(2).filter(|end| *end <= packet.len());
        }
        offset += 1;
        if len == 0 {
            return Some(offset);
        }
        offset = offset
            .checked_add(usize::from(len))
            .filter(|next| *next <= packet.len())?;
    }
}

fn isolated_test_listener_running_status() -> Option<KernelIsolatedTestListenerStatus> {
    let guard = ISOLATED_TEST_LISTENER.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().map(|state| KernelIsolatedTestListenerStatus {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-test-listener-opt-in".into(),
        kernel_area: "listener".into(),
        mutates_runtime: true,
        live_execution_allowed: true,
        running: true,
        host: ISOLATED_TEST_LISTENER_HOST.into(),
        port: Some(state.port),
        started_at_epoch_ms: Some(state.started_at_epoch_ms),
        accepted_connections: state.accepted_connections.load(Ordering::Relaxed),
        loopback_only: true,
        default_route: false,
        forwards_traffic: false,
        mihomo_fallback: true,
        blockers: isolated_test_listener_blockers(),
        warnings: Vec::new(),
        facts: isolated_test_listener_facts(),
        next_safe_batch: "listener-smoke-evidence".into(),
    })
}

fn isolated_test_listener_status(warnings: Vec<String>) -> KernelIsolatedTestListenerStatus {
    isolated_test_listener_running_status().unwrap_or_else(|| KernelIsolatedTestListenerStatus {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-test-listener-opt-in".into(),
        kernel_area: "listener".into(),
        mutates_runtime: false,
        live_execution_allowed: true,
        running: false,
        host: ISOLATED_TEST_LISTENER_HOST.into(),
        port: None,
        started_at_epoch_ms: None,
        accepted_connections: 0,
        loopback_only: true,
        default_route: false,
        forwards_traffic: false,
        mihomo_fallback: true,
        blockers: isolated_test_listener_blockers(),
        warnings,
        facts: isolated_test_listener_facts(),
        next_safe_batch: "listener-smoke-evidence".into(),
    })
}

fn isolated_test_listener_blockers() -> Vec<String> {
    vec![
        "listener is loopback-only and must not be installed as the default proxy".into(),
        "listener must not attach to TUN, system proxy, DNS, or outbound forwarding".into(),
        "Mihomo remains the only production forwarding owner".into(),
    ]
}

fn isolated_test_listener_facts() -> Vec<String> {
    vec![
        "accepted connections receive an immediate local 204 response and are not proxied".into(),
        "start requires isolated listener preflight to pass for the selected port".into(),
    ]
}

pub(super) fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

async fn isolated_test_listener_smoke_request(port: u16) -> Result<String> {
    let mut stream = timeout(
        Duration::from_secs(2),
        TcpStream::connect((ISOLATED_TEST_LISTENER_HOST, port)),
    )
    .await??;
    stream
        .write_all(b"GET /kernel-smoke HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .await?;
    let mut response = [0_u8; 128];
    let bytes_read = timeout(Duration::from_secs(2), stream.read(&mut response)).await??;
    let response = std::string::String::from_utf8_lossy(&response[..bytes_read]);
    Ok(response.lines().next().unwrap_or_default().into())
}

async fn kernel_runtime_config_snapshot() -> Result<Option<String>> {
    Config::runtime()
        .await
        .latest_arc()
        .config
        .as_ref()
        .map(serde_yaml_ng::to_string)
        .transpose()
        .map(|snapshot| snapshot.map(Into::into))
        .map_err(Into::into)
}

fn kernel_listener_smoke_report(
    requested_port: u16,
    started_by_smoke: bool,
    response_status: Option<String>,
    accepted_connections_before: u64,
    accepted_connections_after: u64,
    status_incremented: bool,
    stopped_after_smoke: bool,
    system_proxy_unchanged: bool,
    tun_unchanged: bool,
    runtime_config_unchanged: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
) -> KernelIsolatedTestListenerSmokeEvidenceReport {
    let passed = started_by_smoke
        && response_status.as_deref() == Some("HTTP/1.1 204 No Content")
        && status_incremented
        && stopped_after_smoke
        && system_proxy_unchanged
        && tun_unchanged
        && runtime_config_unchanged
        && blockers.is_empty();
    KernelIsolatedTestListenerSmokeEvidenceReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "listener-smoke-evidence".into(),
        kernel_area: "listener".into(),
        mutates_runtime: started_by_smoke,
        live_execution_allowed: true,
        requested_host: ISOLATED_TEST_LISTENER_HOST.into(),
        requested_port,
        started_by_smoke,
        response_status,
        accepted_connections_before,
        accepted_connections_after,
        status_incremented,
        stopped_after_smoke,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        default_route: false,
        forwards_traffic: false,
        mihomo_fallback: true,
        passed,
        blockers,
        warnings,
        facts: vec![
            "smoke evidence starts and stops only the loopback test listener".into(),
            "local smoke request must receive 204 and must not use outbound forwarding".into(),
            "runtime config, system proxy, and TUN settings are compared before and after".into(),
        ],
        next_safe_batch: "loopback-dns-or-forwarding-decision".into(),
    }
}

fn kernel_loopback_dns_smoke_report(
    requested_port: u16,
    udp_bound: bool,
    local_response_received: bool,
    response_address: Option<String>,
    system_proxy_unchanged: bool,
    tun_unchanged: bool,
    runtime_config_unchanged: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
) -> KernelLoopbackDnsSmokeEvidenceReport {
    let passed = udp_bound
        && local_response_received
        && response_address.as_deref() == Some("127.0.0.1")
        && system_proxy_unchanged
        && tun_unchanged
        && runtime_config_unchanged
        && blockers.is_empty();

    KernelLoopbackDnsSmokeEvidenceReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-dns-smoke-evidence".into(),
        kernel_area: "dns".into(),
        mutates_runtime: udp_bound,
        live_execution_allowed: true,
        requested_host: ISOLATED_TEST_LISTENER_HOST.into(),
        requested_port,
        query_name: LOOPBACK_DNS_SMOKE_QUERY.into(),
        udp_bound,
        local_response_received,
        response_address,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        default_route: false,
        forwards_traffic: false,
        mihomo_fallback: true,
        passed,
        blockers,
        warnings,
        facts: vec![
            "smoke evidence binds one temporary UDP socket on 127.0.0.1".into(),
            "synthetic DNS answer returns 127.0.0.1 without replacing default DNS".into(),
            "runtime config, system proxy, and TUN settings are compared before and after".into(),
        ],
        next_safe_batch: "loopback-forwarding-preflight".into(),
    }
}

fn kernel_loopback_forwarding_smoke_report(
    listener_port: u16,
    target_port: u16,
    listener_accepted: bool,
    target_received: bool,
    response_status: Option<String>,
    bytes_from_client: u64,
    bytes_from_target: u64,
    system_proxy_unchanged: bool,
    tun_unchanged: bool,
    runtime_config_unchanged: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
) -> KernelLoopbackForwardingSmokeEvidenceReport {
    let loopback_forwarded =
        listener_accepted && target_received && response_status.as_deref() == Some("HTTP/1.1 204 No Content");
    let passed = loopback_forwarded
        && bytes_from_client > 0
        && bytes_from_target > 0
        && system_proxy_unchanged
        && tun_unchanged
        && runtime_config_unchanged
        && blockers.is_empty();

    KernelLoopbackForwardingSmokeEvidenceReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-forwarding-smoke-evidence".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: listener_accepted,
        live_execution_allowed: true,
        requested_host: ISOLATED_TEST_LISTENER_HOST.into(),
        listener_port,
        target_port,
        request_path: "/kernel-forwarding-smoke".into(),
        listener_accepted,
        target_received,
        response_status,
        bytes_from_client,
        bytes_from_target,
        loopback_forwarded,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed,
        blockers,
        warnings,
        facts: vec![
            "smoke evidence binds temporary listener and target sockets on 127.0.0.1".into(),
            "the target is synthetic and no outbound adapter is dialed".into(),
            "runtime config, system proxy, and TUN settings are compared before and after".into(),
        ],
        next_safe_batch: "loopback-forwarding-rollback-drill".into(),
    }
}
