# Spans

The **Spans** tab groups log lines that look like distributed-tracing spans into a tree under each trace.

## Switching tabs

| Action | Default |
|--------|---------|
| Next / previous tab | `]` / `[` → `view tab next` / `view tab prev` |
| Command | `:view tab logs` · `:view tab spans` · `:view tab toggle` |
| Shortcuts | `:view logs` · `:view spans` |
| Mouse | Click **Logs** / **Spans** in the top tab bar |

## Building the tree

A record becomes a span when both a trace id and a span id are present. Parent links use `parent_span_id` (or equivalent). Recognized fields include:

| Role | Keys (case-insensitive) |
|------|-------------------------|
| Trace id | `trace_id`, `traceId`, `dd.trace_id`, `otel.trace_id`, nested `dd.trace_id` |
| Span id | `span_id`, `spanId`, `dd.span_id`, `otel.span_id`, nested `dd.span_id`, bare `id` when a trace id is also present |
| Parent | `parent_span_id`, `parent_id`, `parentSpanContext.spanId`, `dd.parent_id`, … |
| Name | `span_name`, `name`, `operation`, `operation_name`, `resource`, else `msg` |
| Duration | `duration_ns`, `duration_us`, `duration_ms`, bare `duration` |
| Status | `status`, `status_code`, `otel.status_code`, … |

Only lines that pass the current filters (and are not hidden) are included.

## Formats

JSONL span fields and Node `util.inspect` OpenTelemetry ReadableSpan dumps can appear in the same stream. Brace-balanced `{…}` blocks are assembled into one log row before parsing (JSON first, then OTel inspect, then logfmt/plain).

OTel inspect dumps use fields like `traceId`, top-level `id` (span id), `parentSpanContext.spanId`, `name`, and `duration` in **microseconds**. See `examples/sample-otel.txt`.

## Keys in the Spans tab

Configured under `[keys.spans]` (overrides `[keys]` while the tree is focused):

| Key | Command | Action |
|-----|---------|--------|
| `j` / `k` | `nav down` / `nav up` | Move |
| `Space` | `fold toggle` | Collapse / expand a trace or parent span |
| `Enter` | `view details on` | Jump to the span's log on the Logs tab and open details |

Trace header rows (no underlying span) fold on `Enter` as well.
