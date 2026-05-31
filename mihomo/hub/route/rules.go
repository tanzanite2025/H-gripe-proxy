package route

import (
	"strconv"
	"time"

	R "github.com/tanzanite2025/mihomo-optimized/rules"
	RC "github.com/tanzanite2025/mihomo-optimized/rules/common"
	RW "github.com/tanzanite2025/mihomo-optimized/rules/wrapper"

	"github.com/tanzanite2025/mihomo-optimized/constant"
	"github.com/tanzanite2025/mihomo-optimized/tunnel"

	"github.com/metacubex/chi"
	"github.com/metacubex/chi/render"
	"github.com/metacubex/http"
)

func ruleRouter() http.Handler {
	r := chi.NewRouter()
	r.Get("/", getRules)
	if !embedMode {
		r.Patch("/disable", disableRules)
		r.Delete("/{index}", deleteRule)
		r.Post("/", createRule)
		r.Get("/sub", getSubRules)
		r.Delete("/sub/{name}", deleteSubRuleBySource)
	}
	return r
}

type Rule struct {
	Index   int    `json:"index"`
	Type    string `json:"type"`
	Payload string `json:"payload"`
	Proxy   string `json:"proxy"`
	Size    int    `json:"size"`
	Source  string `json:"source"`

	// Extra contains information from RuleWrapper
	Extra *RuleExtra `json:"extra,omitempty"`
}

type RuleExtra struct {
	Disabled  bool      `json:"disabled"`
	Deleted   bool      `json:"deleted"`
	HitCount  uint64    `json:"hitCount"`
	HitAt     time.Time `json:"hitAt"`
	MissCount uint64    `json:"missCount"`
	MissAt    time.Time `json:"missAt"`
}

type CreateRuleRequest struct {
	Type      string `json:"type"`
	Payload   string `json:"payload"`
	Proxy     string `json:"proxy"`
	NoResolve bool   `json:"noResolve"`
	Source    string `json:"source,omitempty"`
	SubRule   string `json:"subRule,omitempty"`  // if set, insert into named sub-rule list instead of global rules
	Position  string `json:"position,omitempty"` // "prepend" or "append" (default)
}

func getRules(w http.ResponseWriter, r *http.Request) {
	rawRules := tunnel.Rules()
	total := len(rawRules)

	rules := make([]Rule, 0, total)
	for index, rule := range rawRules {
		rr := Rule{
			Index:   index,
			Type:    rule.RuleType().String(),
			Payload: rule.Payload(),
			Proxy:   rule.Adapter(),
			Size:    -1,
			Source:  "profile",
		}
		if ruleWrapper, ok := rule.(constant.RuleWrapper); ok {
			rr.Source = ruleWrapper.Source()
			rr.Extra = &RuleExtra{
				Disabled:  ruleWrapper.IsDisabled(),
				Deleted:   ruleWrapper.IsDeleted(),
				HitCount:  ruleWrapper.HitCount(),
				HitAt:     ruleWrapper.HitAt(),
				MissCount: ruleWrapper.MissCount(),
				MissAt:    ruleWrapper.MissAt(),
			}
			rule = ruleWrapper.Unwrap()
		}
		if rule.RuleType() == constant.GEOIP || rule.RuleType() == constant.GEOSITE {
			rr.Size = rule.(constant.RuleGroup).GetRecodeSize()
		}
		rules = append(rules, rr)
	}

	q := r.URL.Query()

	// by default, filter out soft-deleted rules unless showDeleted=true
	if q.Get("showDeleted") != "true" {
		filtered := make([]Rule, 0, len(rules))
		for _, rr := range rules {
			if rr.Extra == nil || !rr.Extra.Deleted {
				filtered = append(filtered, rr)
			}
		}
		rules = filtered
	}

	if filterType := q.Get("type"); filterType != "" {
		filtered := make([]Rule, 0, len(rules))
		for _, rr := range rules {
			if rr.Type == filterType {
				filtered = append(filtered, rr)
			}
		}
		rules = filtered
	}
	if filterProxy := q.Get("proxy"); filterProxy != "" {
		filtered := make([]Rule, 0, len(rules))
		for _, rr := range rules {
			if rr.Proxy == filterProxy {
				filtered = append(filtered, rr)
			}
		}
		rules = filtered
	}
	if filterSource := q.Get("source"); filterSource != "" {
		filtered := make([]Rule, 0, len(rules))
		for _, rr := range rules {
			if rr.Source == filterSource {
				filtered = append(filtered, rr)
			}
		}
		rules = filtered
	}

	filteredTotal := len(rules)
	page, _ := strconv.Atoi(q.Get("page"))
	pageSize, _ := strconv.Atoi(q.Get("pageSize"))
	if page < 1 {
		page = 1
	}
	if pageSize < 1 {
		render.JSON(w, r, render.M{
			"rules": rules,
			"total": filteredTotal,
		})
		return
	}

	start := (page - 1) * pageSize
	if start > filteredTotal {
		start = filteredTotal
	}
	end := start + pageSize
	if end > filteredTotal {
		end = filteredTotal
	}

	render.JSON(w, r, render.M{
		"rules":    rules[start:end],
		"total":    filteredTotal,
		"page":     page,
		"pageSize": pageSize,
	})
}

func disableRules(w http.ResponseWriter, r *http.Request) {
	var payload map[int]bool
	if err := render.DecodeJSON(r.Body, &payload); err != nil {
		render.Status(r, http.StatusBadRequest)
		render.JSON(w, r, ErrBadRequest)
		return
	}

	if len(payload) != 0 {
		rules := tunnel.Rules()
		for index, disabled := range payload {
			if index < 0 || index >= len(rules) {
				continue
			}
			rule := rules[index]
			if ruleWrapper, ok := rule.(constant.RuleWrapper); ok {
				ruleWrapper.SetDisabled(disabled)
			}
		}
	}

	render.NoContent(w, r)
}

func deleteRule(w http.ResponseWriter, r *http.Request) {
	indexStr := chi.URLParam(r, "index")
	index, err := strconv.Atoi(indexStr)
	if err != nil {
		render.Status(r, http.StatusBadRequest)
		render.JSON(w, r, ErrBadRequest)
		return
	}

	rules := tunnel.Rules()
	if index < 0 || index >= len(rules) {
		render.Status(r, http.StatusNotFound)
		render.JSON(w, r, render.M{"error": "rule not found"})
		return
	}

	rule := rules[index]
	if ruleWrapper, ok := rule.(constant.RuleWrapper); ok {
		ruleWrapper.SetDeleted(true)
		render.NoContent(w, r)
	} else {
		render.Status(r, http.StatusBadRequest)
		render.JSON(w, r, render.M{"error": "rule cannot be deleted (not wrapped)"})
	}
}

func createRule(w http.ResponseWriter, r *http.Request) {
	var req CreateRuleRequest
	if err := render.DecodeJSON(r.Body, &req); err != nil {
		render.Status(r, http.StatusBadRequest)
		render.JSON(w, r, ErrBadRequest)
		return
	}

	if req.Type == "" || req.Proxy == "" {
		render.Status(r, http.StatusBadRequest)
		render.JSON(w, r, render.M{"error": "type and proxy are required"})
		return
	}

	tp, payload, target, params := RC.ParseRulePayload(req.Type+","+req.Payload+","+req.Proxy, true)
	if target == "" {
		render.Status(r, http.StatusBadRequest)
		render.JSON(w, r, render.M{"error": "invalid rule format"})
		return
	}

	// When inserting into a sub-rule list, pass the current subRules map
	// so that SUB-RULE type rules can resolve their targets.
	var subRulesMap map[string][]constant.Rule
	if req.SubRule != "" {
		subRulesMap = tunnel.SubRules()
	}
	parsed, parseErr := R.ParseRule(tp, payload, target, params, subRulesMap)
	if parseErr != nil {
		render.Status(r, http.StatusBadRequest)
		render.JSON(w, r, render.M{"error": parseErr.Error()})
		return
	}

	source := "runtime"
	if req.Source != "" {
		source = req.Source
	}
	wrapped := RW.NewRuleWrapper(parsed, source)

	position := tunnel.PositionAppend
	if req.Position == "prepend" {
		position = tunnel.PositionPrepend
	}

	var idx int
	if req.SubRule != "" {
		idx = tunnel.InsertSubRule(req.SubRule, wrapped, position)
	} else {
		idx = tunnel.InsertRule(wrapped, position)
	}

	render.JSON(w, r, render.M{
		"index":   idx,
		"subRule": req.SubRule,
	})
}

func getSubRules(w http.ResponseWriter, r *http.Request) {
	rawSubRules := tunnel.SubRules()
	result := make(map[string][]Rule, len(rawSubRules))
	for name, rawRules := range rawSubRules {
		rules := make([]Rule, 0, len(rawRules))
		for index, rule := range rawRules {
			rr := Rule{
				Index:   index,
				Type:    rule.RuleType().String(),
				Payload: rule.Payload(),
				Proxy:   rule.Adapter(),
				Size:    -1,
				Source:  "profile",
			}
			if ruleWrapper, ok := rule.(constant.RuleWrapper); ok {
				rr.Source = ruleWrapper.Source()
				rr.Extra = &RuleExtra{
					Disabled: ruleWrapper.IsDisabled(),
					Deleted:  ruleWrapper.IsDeleted(),
				}
			}
			rules = append(rules, rr)
		}
		result[name] = rules
	}
	render.JSON(w, r, result)
}

type DeleteSubRuleRequest struct {
	SourcePrefix string `json:"sourcePrefix"`
}

func deleteSubRuleBySource(w http.ResponseWriter, r *http.Request) {
	name := chi.URLParam(r, "name")
	if name == "" {
		render.Status(r, http.StatusBadRequest)
		render.JSON(w, r, ErrBadRequest)
		return
	}

	var req DeleteSubRuleRequest
	if err := render.DecodeJSON(r.Body, &req); err != nil {
		render.Status(r, http.StatusBadRequest)
		render.JSON(w, r, ErrBadRequest)
		return
	}

	sourcePrefix := req.SourcePrefix
	if sourcePrefix == "" {
		sourcePrefix = "security:"
	}

	count := tunnel.DeleteSubRuleBySource(name, sourcePrefix)
	render.JSON(w, r, render.M{
		"deleted": count,
	})
}
