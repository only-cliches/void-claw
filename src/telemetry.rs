/// OpenTelemetry + tracing-subscriber initialisation.
///
/// Call [`init`] once at startup.  The returned [`TelemetryHandle`] must be
/// kept alive until shutdown; call [`TelemetryHandle::shutdown`] after the
/// main event loop exits to flush any in-flight spans.
use anyhow::Result;
use opentelemetry::{KeyValue, trace::TracerProvider as _};
use opentelemetry_sdk::{Resource, runtime, trace::TracerProvider};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::{Config, OtlpProtocol};

// ── Public handle ─────────────────────────────────────────────────────────────

pub struct TelemetryHandle {
    provider: Option<TracerProvider>,
    _log_guard: tracing_appender::non_blocking::WorkerGuard,
}

impl TelemetryHandle {
    /// Flush buffered spans and shut down the exporter.
    pub fn shutdown(self) -> Result<()> {
        if let Some(provider) = self.provider {
            provider.shutdown()?;
        }
        Ok(())
    }
}

// ── Init ─────────────────────────────────────────────────────────────────────

/// Initialise the global tracing subscriber (and optionally OTel export).
///
/// Must be called before any `tracing::*` macros are used.
pub fn init(config: &Config) -> Result<TelemetryHandle> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let (log_writer, log_guard) = build_log_writer(config)?;
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(log_writer)
        .with_ansi(false);

    if let Some(otlp_cfg) = &config.logging.otlp {
        let exporter = build_exporter(otlp_cfg)?;

        let hostname = machine_hostname();
        let resource = Resource::new(vec![
            KeyValue::new("service.name", "agent-zero"),
            KeyValue::new("host.name", hostname),
        ]);

        let provider = TracerProvider::builder()
            .with_batch_exporter(exporter, runtime::Tokio)
            .with_resource(resource)
            .build();

        // Get the tracer *before* boxing the provider, to satisfy the
        // `PreSampledTracer` bound required by `tracing_opentelemetry::layer`.
        let tracer = provider.tracer("agent-zero");
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        opentelemetry::global::set_tracer_provider(provider.clone());

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .with(otel_layer)
            .init();

        Ok(TelemetryHandle {
            provider: Some(provider),
            _log_guard: log_guard,
        })
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();

        Ok(TelemetryHandle {
            provider: None,
            _log_guard: log_guard,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_exporter(otlp: &crate::config::OtlpConfig) -> Result<opentelemetry_otlp::SpanExporter> {
    use opentelemetry_otlp::{SpanExporter, WithExportConfig};
    match otlp.protocol {
        OtlpProtocol::Grpc => Ok(SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&otlp.endpoint)
            .build()?),
        OtlpProtocol::Http => Ok(SpanExporter::builder()
            .with_http()
            .with_endpoint(&otlp.endpoint)
            .build()?),
    }
}

/// Best-effort hostname for the OTel `host.name` resource attribute.
/// Reads `$HOSTNAME` first (always set inside containers), then
/// `$COMPUTERNAME` (Windows), falls back to `/etc/hostname` (Linux/macOS),
/// then "unknown".
pub fn machine_hostname() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("COMPUTERNAME").ok().filter(|s| !s.is_empty()))
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn build_log_writer(
    config: &Config,
) -> Result<(
    tracing_appender::non_blocking::NonBlocking,
    tracing_appender::non_blocking::WorkerGuard,
)> {
    let log_dir = &config.logging.log_dir;
    std::fs::create_dir_all(log_dir)?;
    let appender = tracing_appender::rolling::daily(log_dir, "agent-zero.log");
    Ok(tracing_appender::non_blocking(appender))
}
