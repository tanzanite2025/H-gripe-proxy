package main

import (
	"context"
	"encoding/base64"
	"errors"
	"flag"
	"fmt"
	"io"
	"net"
	"os"
	"os/signal"
	"path/filepath"
	"runtime"
	"strings"
	"syscall"

	"github.com/tanzanite2025/mihomo-optimized/common/cmd"
	"github.com/tanzanite2025/mihomo-optimized/component/generator"
	"github.com/tanzanite2025/mihomo-optimized/component/geodata"
	"github.com/tanzanite2025/mihomo-optimized/component/updater"
	"github.com/tanzanite2025/mihomo-optimized/config"
	C "github.com/tanzanite2025/mihomo-optimized/constant"
	"github.com/tanzanite2025/mihomo-optimized/constant/features"
	"github.com/tanzanite2025/mihomo-optimized/hub"
	"github.com/tanzanite2025/mihomo-optimized/hub/executor"
	"github.com/tanzanite2025/mihomo-optimized/log"
	"github.com/tanzanite2025/mihomo-optimized/rules/provider"

	"go.uber.org/automaxprocs/maxprocs"
)

type defaultResolverGuardMode string

const (
	defaultResolverGuardSoft   defaultResolverGuardMode = "soft"
	defaultResolverGuardStrict defaultResolverGuardMode = "strict"

	defaultResolverGuardModeEnv   = "MIHOMO_DEFAULT_RESOLVER_GUARD"
	defaultResolverGuardStrictEnv = "MIHOMO_STRICT_DEFAULT_RESOLVER"
)

var (
	version                bool
	testConfig             bool
	geodataMode            bool
	homeDir                string
	configFile             string
	configString           string
	configBytes            []byte
	externalUI             string
	externalController     string
	externalControllerUnix string
	externalControllerPipe string
	secret                 string
	postUp                 string
	postDown               string
)

func defaultResolverGuardModeFromEnv() defaultResolverGuardMode {
	mode := strings.ToLower(strings.TrimSpace(os.Getenv(defaultResolverGuardModeEnv)))
	if mode == "" {
		mode = strings.ToLower(strings.TrimSpace(os.Getenv(defaultResolverGuardStrictEnv)))
	}

	switch mode {
	case "strict", "debug", "panic", "exit", "1", "true", "yes", "on":
		return defaultResolverGuardStrict
	default:
		return defaultResolverGuardSoft
	}
}

func installDefaultResolverGuard() {
	mode := defaultResolverGuardModeFromEnv()

	net.DefaultResolver.PreferGo = true
	net.DefaultResolver.Dial = func(ctx context.Context, network, address string) (net.Conn, error) {
		buf := make([]byte, 1024)
		for {
			n := runtime.Stack(buf, true)
			if n < len(buf) {
				buf = buf[:n]
				break
			}
			buf = make([]byte, 2*len(buf))
		}

		message := fmt.Sprintf("blocked unexpected net.DefaultResolver lookup: network=%s address=%s", network, address)
		fmt.Fprintf(os.Stderr, "%s\n\n%s", message, buf)
		if mode == defaultResolverGuardStrict {
			os.Exit(2)
		}
		return nil, errors.New(message)
	}
}

func init() {
	flag.StringVar(&homeDir, "d", os.Getenv("CLASH_HOME_DIR"), "set configuration directory")
	flag.StringVar(&configFile, "f", os.Getenv("CLASH_CONFIG_FILE"), "specify configuration file")
	flag.StringVar(&configString, "config", os.Getenv("CLASH_CONFIG_STRING"), "specify base64-encoded configuration string")
	flag.StringVar(&externalUI, "ext-ui", os.Getenv("CLASH_OVERRIDE_EXTERNAL_UI_DIR"), "override external ui directory")
	flag.StringVar(&externalController, "ext-ctl", os.Getenv("CLASH_OVERRIDE_EXTERNAL_CONTROLLER"), "override external controller address")
	flag.StringVar(&externalControllerUnix, "ext-ctl-unix", os.Getenv("CLASH_OVERRIDE_EXTERNAL_CONTROLLER_UNIX"), "override external controller unix address")
	flag.StringVar(&externalControllerPipe, "ext-ctl-pipe", os.Getenv("CLASH_OVERRIDE_EXTERNAL_CONTROLLER_PIPE"), "override external controller pipe address")
	flag.StringVar(&secret, "secret", os.Getenv("CLASH_OVERRIDE_SECRET"), "override secret for RESTful API")
	flag.StringVar(&postUp, "post-up", os.Getenv("CLASH_POST_UP"), "set post-up script")
	flag.StringVar(&postDown, "post-down", os.Getenv("CLASH_POST_DOWN"), "set post-down script")
	flag.BoolVar(&geodataMode, "m", false, "set geodata mode")
	flag.BoolVar(&version, "v", false, "show current version of mihomo")
	flag.BoolVar(&testConfig, "t", false, "test configuration and exit")
}

func main() {
	flag.Parse()

	// Guard against accidental use of Go's system resolver. Release defaults to
	// a controlled error; strict/debug mode keeps the hard fail for leak hunting.
	installDefaultResolverGuard()

	_, _ = maxprocs.Set(maxprocs.Logger(func(string, ...any) {}))

	if len(os.Args) > 1 && os.Args[1] == "convert-ruleset" {
		provider.ConvertMain(os.Args[2:])
		return
	}

	if len(os.Args) > 1 && os.Args[1] == "generate" {
		generator.Main(os.Args[2:])
		return
	}

	if version {
		fmt.Printf("Mihomo Meta %s %s %s with %s %s\n",
			C.Version, runtime.GOOS, runtime.GOARCH, runtime.Version(), C.BuildTime)
		if tags := features.Tags(); len(tags) != 0 {
			fmt.Printf("Use tags: %s\n", strings.Join(tags, ", "))
		}

		return
	}

	if homeDir != "" {
		if !filepath.IsAbs(homeDir) {
			currentDir, _ := os.Getwd()
			homeDir = filepath.Join(currentDir, homeDir)
		}
		C.SetHomeDir(homeDir)
	}

	if geodataMode {
		geodata.SetGeodataMode(true)
	}

	if configString != "" {
		var err error
		configBytes, err = base64.StdEncoding.DecodeString(configString)
		if err != nil {
			log.Fatalln("Initial configuration error: %s", err.Error())
			return
		}
	} else if configFile == "-" {
		var err error
		configBytes, err = io.ReadAll(os.Stdin)
		if err != nil {
			log.Fatalln("Initial configuration error: %s", err.Error())
			return
		}
	} else {
		if configFile != "" {
			if !filepath.IsAbs(configFile) {
				currentDir, _ := os.Getwd()
				configFile = filepath.Join(currentDir, configFile)
			}
		} else {
			configFile = filepath.Join(C.Path.HomeDir(), C.Path.Config())
		}
		C.SetConfig(configFile)

		if err := config.Init(C.Path.HomeDir()); err != nil {
			log.Fatalln("Initial configuration directory error: %s", err.Error())
		}
	}

	if testConfig {
		if len(configBytes) != 0 {
			if _, err := executor.ParseWithBytes(configBytes); err != nil {
				log.Errorln(err.Error())
				fmt.Println("configuration test failed")
				os.Exit(1)
			}
		} else {
			if _, err := executor.Parse(); err != nil {
				log.Errorln(err.Error())
				fmt.Printf("configuration file %s test failed\n", C.Path.Config())
				os.Exit(1)
			}
		}
		fmt.Printf("configuration file %s test is successful\n", C.Path.Config())
		return
	}

	var options []hub.Option
	if externalUI != "" {
		options = append(options, hub.WithExternalUI(externalUI))
	}
	if externalController != "" {
		options = append(options, hub.WithExternalController(externalController))
	}
	if externalControllerUnix != "" {
		options = append(options, hub.WithExternalControllerUnix(externalControllerUnix))
	}
	if externalControllerPipe != "" {
		options = append(options, hub.WithExternalControllerPipe(externalControllerPipe))
	}
	if secret != "" {
		options = append(options, hub.WithSecret(secret))
	}

	if err := hub.Parse(configBytes, options...); err != nil {
		log.Fatalln("Parse config error: %s", err.Error())
	}

	if updater.GeoAutoUpdate() {
		updater.RegisterGeoUpdater()
	}

	if postDown != "" {
		defer func() {
			if _, err := cmd.ExecShell(postDown); err != nil {
				log.Errorln("post-down script error: %s", err.Error())
			}
		}()
	}
	if postUp != "" {
		if _, err := cmd.ExecShell(postUp); err != nil {
			log.Fatalln("post-up script error: %s", err.Error())
		}
	}

	defer executor.Shutdown()

	termSign := make(chan os.Signal, 1)
	hupSign := make(chan os.Signal, 1)
	signal.Notify(termSign, syscall.SIGINT, syscall.SIGTERM)
	signal.Notify(hupSign, syscall.SIGHUP)
	for {
		select {
		case <-termSign:
			return
		case <-hupSign:
			if err := hub.Parse(configBytes, options...); err != nil {
				log.Errorln("Parse config error: %s", err.Error())
			}
		}
	}
}
