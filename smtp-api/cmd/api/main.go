package main

import (
	"log/slog"
	"net/http"
	"os"
	"strings"
	"time"

	"example.com/smtp-platform/smtp-api/internal/controlplane"
	"example.com/smtp-platform/smtp-api/internal/health"
	"example.com/smtp-platform/smtp-api/internal/observability"
)

func main() {
	slog.SetDefault(slog.New(slog.NewJSONHandler(os.Stderr, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	})))
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health/live", health.Handler)
	mux.HandleFunc("GET /health/ready", health.ReadyHandler(func() bool { return true }))
	environment := strings.ToLower(strings.TrimSpace(os.Getenv("APP_ENVIRONMENT")))
	devMode := os.Getenv("CONTROL_PLANE_DEV_MODE") == "true"
	if devMode && (environment == "production" || environment == "prod") {
		slog.Error("development identity adapter cannot run in production")
		os.Exit(1)
	}
	if devMode && environment == "" {
		slog.Warn("development identity adapter enabled; set APP_ENVIRONMENT explicitly")
	}
	metrics := observability.NewMetrics()
	control := controlplane.Handler{
		Store:   controlplane.NewStore(),
		DevMode: devMode,
		Metrics: metrics,
	}
	mux.Handle("/v1/", control.Routes())
	mux.HandleFunc("GET /metrics", metrics.Handler)

	addr := os.Getenv("API_ADDR")
	if addr == "" {
		addr = ":8080"
	}

	server := &http.Server{
		Addr:              addr,
		Handler:           mux,
		ReadHeaderTimeout: 5 * time.Second,
	}
	slog.Info("API listening", "address", addr)
	if err := server.ListenAndServe(); err != nil {
		slog.Error("API server stopped", "error", err)
		os.Exit(1)
	}
}
