package profile

import (
	"github.com/tanzanite2025/mihomo-optimized/common/atomic"
)

// StoreSelected is a global switch for storing selected proxy to cache
var StoreSelected = atomic.NewBool(true)
