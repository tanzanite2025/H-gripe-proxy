package dns

import (
	"strings"
	"sync"
	"time"
)

// TrustLevel represents the trustworthiness of a DNS server.
type TrustLevel int

const (
	// TrustUntrusted indicates an untrusted DNS server (plain UDP/TCP, easily tampered)
	TrustUntrusted TrustLevel = iota
	// TrustLow indicates a DNS server with basic encryption but no authentication
	TrustLow
	// TrustMedium indicates a DNS server with encryption and some authentication
	TrustMedium
	// TrustHigh indicates a fully trusted DNS server (DoH/DoT with known provider)
	TrustHigh
	// TrustMaximum indicates a DNS server with DNSSEC validation
	TrustMaximum
)

func (t TrustLevel) String() string {
	switch t {
	case TrustUntrusted:
		return "untrusted"
	case TrustLow:
		return "low"
	case TrustMedium:
		return "medium"
	case TrustHigh:
		return "high"
	case TrustMaximum:
		return "maximum"
	default:
		return "unknown"
	}
}

func (t TrustLevel) MarshalJSON() ([]byte, error) {
	return []byte(`"` + t.String() + `"`), nil
}

// ServerClassification classifies a DNS server by protocol and trust level.
type ServerClassification struct {
	Address     string     `json:"address"`
	Protocol    string     `json:"protocol"`
	TrustLevel  TrustLevel `json:"trustLevel"`
	Encrypted   bool       `json:"encrypted"`
	Description string     `json:"description,omitempty"`
}

// TrustEvaluator evaluates DNS server trustworthiness based on protocol and address.
type TrustEvaluator struct {
	mu             sync.RWMutex
	classifications map[string]ServerClassification
}

var defaultTrustEvaluator = &TrustEvaluator{
	classifications: make(map[string]ServerClassification),
}

// ClassifyServer classifies a DNS server based on its address string.
// Address formats: udp://host:port, tcp://host:port, https://host/path, tls://host:port
func ClassifyServer(address string) ServerClassification {
	addr := strings.ToLower(address)
	c := ServerClassification{Address: address}

	// Determine protocol
	switch {
	case strings.HasPrefix(addr, "https://") || strings.HasPrefix(addr, "h3://"):
		c.Protocol = "DoH"
		c.Encrypted = true
		c.TrustLevel = classifyDoH(addr)
	case strings.HasPrefix(addr, "tls://"):
		c.Protocol = "DoT"
		c.Encrypted = true
		c.TrustLevel = classifyDoT(addr)
	case strings.HasPrefix(addr, "quic://"):
		c.Protocol = "DoQ"
		c.Encrypted = true
		c.TrustLevel = TrustHigh
	case strings.HasPrefix(addr, "tcp://"):
		c.Protocol = "TCP"
		c.Encrypted = false
		c.TrustLevel = TrustLow
	default:
		// Default is UDP
		c.Protocol = "UDP"
		c.Encrypted = false
		c.TrustLevel = TrustUntrusted
	}

	return c
}

func classifyDoH(addr string) TrustLevel {
	// Well-known trusted DoH providers
	trustedProviders := []string{
		"cloudflare.com",  // 1.1.1.1
		"dns.google",      // 8.8.8.8
		"dns.quad9.net",   // 9.9.9.9
		"doh.cleanbrowsing.org",
		"dns.adguard.com",
		"doh.opendns.com",
		"mozilla.cloudflare-dns.com",
	}

	for _, provider := range trustedProviders {
		if strings.Contains(addr, provider) {
			return TrustHigh
		}
	}
	return TrustMedium
}

func classifyDoT(addr string) TrustLevel {
	trustedProviders := []string{
		"1dot1dot1dot1dot1.cloudflare-dns.com",
		"dns.google",
		"quad9.net",
		"dns.adguard.com",
		"dns.cleanbrowsing.org",
	}

	for _, provider := range trustedProviders {
		if strings.Contains(addr, provider) {
			return TrustHigh
		}
	}
	return TrustMedium
}

// EvaluateServers classifies all given DNS servers and stores the results.
func (e *TrustEvaluator) EvaluateServers(servers []string) {
	e.mu.Lock()
	defer e.mu.Unlock()

	for _, server := range servers {
		if _, exists := e.classifications[server]; !exists {
			e.classifications[server] = ClassifyServer(server)
		}
	}
}

// GetClassification returns the classification for a server.
func (e *TrustEvaluator) GetClassification(server string) (ServerClassification, bool) {
	e.mu.RLock()
	defer e.mu.RUnlock()

	c, ok := e.classifications[server]
	return c, ok
}

// GetAllClassifications returns all server classifications.
func (e *TrustEvaluator) GetAllClassifications() []ServerClassification {
	e.mu.RLock()
	defer e.mu.RUnlock()

	result := make([]ServerClassification, 0, len(e.classifications))
	for _, c := range e.classifications {
		result = append(result, c)
	}
	return result
}

// TrustSummary returns a summary of DNS server trust levels.
type TrustSummary struct {
	Total          int                  `json:"total"`
	Encrypted      int                  `json:"encrypted"`
	Unencrypted    int                  `json:"unencrypted"`
	ByTrustLevel   map[string]int       `json:"byTrustLevel"`
	Servers        []ServerClassification `json:"servers"`
	LeakRiskScore  float64              `json:"leakRiskScore"`
	LastEvaluated  time.Time            `json:"lastEvaluated"`
}

// GetTrustSummary returns a summary of all classified DNS servers.
func (e *TrustEvaluator) GetTrustSummary() TrustSummary {
	e.mu.RLock()
	defer e.mu.RUnlock()

	summary := TrustSummary{
		ByTrustLevel:  make(map[string]int),
		Servers:       make([]ServerClassification, 0, len(e.classifications)),
		LastEvaluated: time.Now(),
	}

	var riskSum float64
	for _, c := range e.classifications {
		summary.Total++
		summary.Servers = append(summary.Servers, c)

		if c.Encrypted {
			summary.Encrypted++
		} else {
			summary.Unencrypted++
		}

		level := c.TrustLevel.String()
		summary.ByTrustLevel[level]++

		// Risk score: unencrypted servers contribute more risk
		switch c.TrustLevel {
		case TrustUntrusted:
			riskSum += 1.0
		case TrustLow:
			riskSum += 0.6
		case TrustMedium:
			riskSum += 0.3
		case TrustHigh:
			riskSum += 0.1
		case TrustMaximum:
			riskSum += 0.0
		}
	}

	if summary.Total > 0 {
		summary.LeakRiskScore = riskSum / float64(summary.Total)
	}

	return summary
}

// GetTrustEvaluator returns the global trust evaluator instance.
func GetTrustEvaluator() *TrustEvaluator {
	return defaultTrustEvaluator
}
