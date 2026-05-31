package dns

import (
	"context"
	"sync"

	"github.com/metacubex/mihomo/component/resolver"
	"github.com/metacubex/mihomo/log"

	D "github.com/miekg/dns"
)

// PreResolver warms up the DNS cache by resolving commonly used domains.
type PreResolver struct {
	mu      sync.Mutex
	domains []string
	running bool
}

var defaultPreResolver = &PreResolver{
	domains: []string{
		// Common CDNs and services
		"www.google.com",
		"google.com",
		"googleapis.com",
		"googleusercontent.com",
		"gstatic.com",
		"youtube.com",
		"yt3.ggpht.com",
		"github.com",
		"githubusercontent.com",
		"cloudflare.com",
		"cdnjs.cloudflare.com",
		"amazonaws.com",
		"akamaihd.net",
		"edgecastcdn.net",
		"fastly.net",
		"twimg.com",
		"twitter.com",
		"x.com",
		"facebook.com",
		"fbcdn.net",
		"instagram.com",
		"cdninstagram.com",
		"apple.com",
		"icloud.com",
		"microsoft.com",
		"azureedge.net",
		"office.net",
	},
}

// Warmup resolves all preloaded domains in parallel to fill the cache.
func (p *PreResolver) Warmup() {
	p.mu.Lock()
	if p.running {
		p.mu.Unlock()
		return
	}
	p.running = true
	p.mu.Unlock()

	defer func() {
		p.mu.Lock()
		p.running = false
		p.mu.Unlock()
	}()

	if resolver.DefaultResolver == nil {
		return
	}

	log.Infoln("[DNS] Pre-resolving %d common domains", len(p.domains))

	for _, domain := range p.domains {
		go func(d string) {
			ctx, cancel := context.WithTimeout(context.Background(), resolver.DefaultDNSTimeout)
			defer cancel()

			// A record
			m := &D.Msg{}
			m.SetQuestion(D.Fqdn(d), D.TypeA)
			_, _ = resolver.DefaultResolver.ExchangeContext(ctx, m)

			// AAAA record (if ipv6 enabled)
			m6 := &D.Msg{}
			m6.SetQuestion(D.Fqdn(d), D.TypeAAAA)
			ctx2, cancel2 := context.WithTimeout(context.Background(), resolver.DefaultDNSTimeout)
			defer cancel2()
			_, _ = resolver.DefaultResolver.ExchangeContext(ctx2, m6)
		}(domain)
	}
}

// AddDomain adds a domain to the pre-resolve list.
func (p *PreResolver) AddDomain(domain string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	for _, d := range p.domains {
		if d == domain {
			return
		}
	}
	p.domains = append(p.domains, domain)
}

// SetDomains replaces the pre-resolve domain list.
func (p *PreResolver) SetDomains(domains []string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.domains = domains
}

// GetDomains returns the current pre-resolve domain list.
func (p *PreResolver) GetDomains() []string {
	p.mu.Lock()
	defer p.mu.Unlock()
	result := make([]string, len(p.domains))
	copy(result, p.domains)
	return result
}

// GetPreResolver returns the global PreResolver instance.
func GetPreResolver() *PreResolver {
	return defaultPreResolver
}
