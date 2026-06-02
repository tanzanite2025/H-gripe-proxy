package route

import (
	"runtime"

	"github.com/tanzanite2025/mihomo-optimized/component/obfuscation"
	"github.com/tanzanite2025/mihomo-optimized/component/perf"
	"github.com/tanzanite2025/mihomo-optimized/tunnel"

	"github.com/metacubex/chi"
	"github.com/metacubex/chi/render"
	"github.com/metacubex/http"
)

func engineRouter() http.Handler {
	r := chi.NewRouter()
	r.Get("/stats", getEngineStats)
	r.Get("/connections/top", getTopConnections)
	r.Get("/buffer-pool", getBufferPoolStats)
	r.Get("/rule-traffic", getRuleTraffic)
	r.Get("/egress", getEgressStatus)
	r.Get("/obfuscation/tls", getTLSFingerprintStats)
	r.Post("/obfuscation/tls/rotate", forceTLSRotation)
	r.Get("/obfuscation/stats", getObfuscationStats)
	r.Post("/obfuscation/stats/reset", resetObfuscationStats)
	r.Get("/perf/stats", getPerfStats)
	r.Get("/perf/hot-reload", getHotReloadStatus)
	r.Get("/perf/xdp", getXDPStatus)
	return r
}

func getEngineStats(w http.ResponseWriter, r *http.Request) {
	stats := map[string]any{
		"activeConnections": tunnel.DefaultConnManager.ActiveCount(),
		"trackedConns":      tunnel.DefaultConnTrafficStats.ActiveCount(),
	}
	render.JSON(w, r, stats)
}

func getTopConnections(w http.ResponseWriter, r *http.Request) {
	top := tunnel.DefaultConnTrafficStats.GetTopByBandwidth(10)
	render.JSON(w, r, top)
}

func getBufferPoolStats(w http.ResponseWriter, r *http.Request) {
	stats := tunnel.DefaultBufferPool.Stats()
	render.JSON(w, r, stats)
}

func getRuleTraffic(w http.ResponseWriter, r *http.Request) {
	snapshot := tunnel.DefaultRuleTrafficStats.Snapshot()
	render.JSON(w, r, snapshot)
}

func getEgressStatus(w http.ResponseWriter, r *http.Request) {
	render.JSON(w, r, tunnel.DefaultEgressMonitor.Snapshot())
}

func getTLSFingerprintStats(w http.ResponseWriter, r *http.Request) {
	stats := map[string]any{
		"currentFingerprint": obfuscation.DefaultTLSRotator.CurrentFingerprint(),
		"rotationCount":      obfuscation.DefaultTLSRotator.RotationCount(),
		"usageSnapshot":      obfuscation.DefaultTLSFingerprintStats.Snapshot(),
	}
	render.JSON(w, r, stats)
}

func forceTLSRotation(w http.ResponseWriter, r *http.Request) {
	newFingerprint := obfuscation.DefaultTLSRotator.ForceRotation()
	obfuscation.DefaultObfStats.OnTLSRotation()
	result := map[string]any{
		"newFingerprint": newFingerprint,
	}
	render.JSON(w, r, result)
}

func getObfuscationStats(w http.ResponseWriter, r *http.Request) {
	stats := map[string]any{
		"obfuscation": obfuscation.DefaultObfStats.Snapshot(),
		"tls": map[string]any{
			"currentFingerprint": obfuscation.DefaultTLSRotator.CurrentFingerprint(),
			"rotationCount":      obfuscation.DefaultTLSRotator.RotationCount(),
			"usageSnapshot":      obfuscation.DefaultTLSFingerprintStats.Snapshot(),
		},
	}
	render.JSON(w, r, stats)
}

func resetObfuscationStats(w http.ResponseWriter, r *http.Request) {
	obfuscation.DefaultObfStats.Reset()
	render.JSON(w, r, map[string]any{"status": "ok"})
}

func getPerfStats(w http.ResponseWriter, r *http.Request) {
	var m runtime.MemStats
	runtime.ReadMemStats(&m)
	stats := map[string]any{
		"goroutines":     runtime.NumGoroutine(),
		"gogc":           perf.DefaultGCOpt.TargetGOGC,
		"memLimit":       perf.DefaultGCOpt.MemoryLimit,
		"heapAlloc":      m.HeapAlloc,
		"heapSys":        m.HeapSys,
		"heapInUse":      m.HeapInuse,
		"stackInUse":     m.StackInuse,
		"numGC":          m.NumGC,
		"gcPauseTotal":   m.PauseTotalNs,
		"protectedConns": perf.DefaultHotReloader.ProtectedCount(),
		"ruleVersion":    perf.DefaultHotReloader.RuleVersion(),
	}
	render.JSON(w, r, stats)
}

func getHotReloadStatus(w http.ResponseWriter, r *http.Request) {
	status := map[string]any{
		"ruleVersion":    perf.DefaultHotReloader.RuleVersion(),
		"protectedConns": perf.DefaultHotReloader.ProtectedCount(),
		"xdpLoaded":      perf.DefaultXDPAccelerator.IsLoaded(),
	}
	render.JSON(w, r, status)
}

func getXDPStatus(w http.ResponseWriter, r *http.Request) {
	status := map[string]any{
		"loaded":  perf.DefaultXDPAccelerator.IsLoaded(),
		"enabled": perf.DefaultXDPAccelerator != nil,
	}
	render.JSON(w, r, status)
}
