package main

import "testing"

func TestDefaultResolverGuardModeDefaultsToSoft(t *testing.T) {
	t.Setenv(defaultResolverGuardModeEnv, "")
	t.Setenv(defaultResolverGuardStrictEnv, "")

	if mode := defaultResolverGuardModeFromEnv(); mode != defaultResolverGuardSoft {
		t.Fatalf("expected default resolver guard to default to soft mode, got %q", mode)
	}
}

func TestDefaultResolverGuardModeStrictValues(t *testing.T) {
	for _, value := range []string{"strict", "debug", "panic", "exit", "1", "true", "yes", "on"} {
		t.Run(value, func(t *testing.T) {
			t.Setenv(defaultResolverGuardModeEnv, value)
			t.Setenv(defaultResolverGuardStrictEnv, "")

			if mode := defaultResolverGuardModeFromEnv(); mode != defaultResolverGuardStrict {
				t.Fatalf("expected %q to enable strict mode, got %q", value, mode)
			}
		})
	}
}

func TestDefaultResolverGuardLegacyStrictEnv(t *testing.T) {
	t.Setenv(defaultResolverGuardModeEnv, "")
	t.Setenv(defaultResolverGuardStrictEnv, "1")

	if mode := defaultResolverGuardModeFromEnv(); mode != defaultResolverGuardStrict {
		t.Fatalf("expected legacy strict env to enable strict mode, got %q", mode)
	}
}
