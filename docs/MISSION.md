# Mission: Build a Reverse Proxy, Load Balancer, and Dashboard From Scratch

## Purpose

Build an independent Rust reverse proxy and load balancer with a management dashboard. Learn the engineering behind reliable network infrastructure by implementing the core machinery, testing its failure modes, and growing it into a comprehensive, operable system.

This is **not a project to learn or wrap Pingora**. This repository is reference material: borrow useful ideas, study implementation trade-offs, and build our own system.

Success means understanding and owning the design—not merely assembling a feature checklist.

## What “From Scratch” Means

We own:

- Connection and request lifecycles.
- Streaming and backpressure orchestration.
- Routing and backend selection.
- Health management and draining.
- Connection pooling policies.
- Deadlines, error handling, and retry safety.
- Configuration validation and application.
- The management API and dashboard.
- Observability and operational behavior.

We may use established libraries for foundational capabilities:

- Tokio for asynchronous execution and socket I/O.
- rustls for TLS; do not implement cryptography.
- HTTP parsing or protocol libraries where appropriate.
- Serde for configuration serialization.
- Axum for the administrative API.
- tracing and metrics libraries for instrumentation.
- React and TypeScript for the dashboard.

A restricted HTTP/1.1 implementation is a useful learning exercise. It must not be treated as production-safe until framing, malformed input, and interoperability are thoroughly tested. Implementing HTTP/2, an async runtime, or cryptographic primitives is not required to own the proxy architecture.

## Product Vision

```text
                         APPLICATION TRAFFIC

Clients --> TCP / TLS listeners --> HTTP session --> Request policy
                                                         |
                                                         v
                                                       Router
                                                         |
                                                         v
                                                   Load balancer
                                                         |
                                                         v
                                                Upstream connector
                                                         |
                                             +-----------+-----------+
                                             v           v           v
                                          Backend A   Backend B   Backend C

Responses stream back with bounded buffering and backpressure.

                         MANAGEMENT TRAFFIC

Browser --> Dashboard --> Admin API --> Control operations
                                            |
                         +------------------+------------------+
                         v                  v                  v
                  Configuration       Runtime inspection   Drain / enable
                  validation and      and telemetry        operations
                  publication
```

The dashboard manages and observes the proxy. It is never in the application traffic path. Dashboard or admin API failure must not interrupt proxying with the last successfully applied configuration.

Initially, the proxy and admin API run in one process on separate listeners. A distributed control plane is not needed.

## Engineering Principles

1. **Build vertical slices.** Each milestone must produce working behavior that can be exercised end to end.
2. **Correctness before optimization.** Protocol safety, bounded resources, and predictable failures come before throughput tuning.
3. **Streaming by default.** Do not buffer entire bodies unless an explicit feature requires it and imposes a limit.
4. **Backpressure everywhere.** Slow peers and dashboard subscribers must not cause unbounded memory growth.
5. **Failure is part of the specification.** Test disconnects, stalled peers, malformed messages, cancellation, and partial progress.
6. **Separate responsibilities.** Protocol handling, routing, balancing, transport, and management must not become one giant handler.
7. **Keep the hot path independent.** Database access and dashboard delivery must not be required to forward requests.
8. **Prefer explicit policy.** Retry eligibility, forwarding headers, all-backends-unavailable behavior, and drain semantics must be defined.
9. **Grow abstractions from working code.** Do not start with a generic plugin framework or a large workspace.
10. **Measure before optimizing.** Use reproducible workloads and inspect latency, errors, memory, and connection behavior—not just requests per second.

## Architecture and State Boundaries

### Data Plane

Owns live connections and requests:

- TCP and TLS listeners.
- HTTP parsing and session state.
- Request validation and policy.
- Route matching.
- Backend selection.
- Upstream connection acquisition.
- Bidirectional streaming.
- Deadlines, cancellation, and failure handling.

### Control Plane

Owns management decisions:

- Configuration parsing and validation.
- Versioned configuration application.
- Backend administrative state.
- Health-check scheduling and results.
- Readiness, draining, and shutdown coordination.
- Administrative API operations.

### Three Different Kinds of State

| Kind          | Examples                                             | Handling                    |
| ------------- | ---------------------------------------------------- | --------------------------- |
| Configuration | Routes, backend addresses, weights, timeout policies | Persisted and versioned     |
| Runtime state | Health, draining, active connections, pool occupancy | Managed in memory           |
| Telemetry     | Counters, latency distributions, logs, health events | Bounded locally or exported |

Do not store every request in the configuration database. Do not confuse a saved configuration with a successfully applied configuration.

### Configuration Publication

```text
Candidate configuration
          |
          v
Parse, validate, and prepare
          |
          +--> Failure: report errors; retain current configuration
          |
          v
Publish immutable snapshot
          |
          +--> New requests use the new snapshot
          +--> Existing requests retain their original snapshot
```

Record saved, applied, and failed revisions separately. Updates must identify their base revision to prevent silent overwrites by concurrent editors.

Initially, live changes cover routes, pools, weights, and health-check settings. Listener and TLS changes may require restart until their lifecycle is explicitly implemented.

## Build Roadmap

### Milestone 0 — Controlled Test Laboratory

Build three identifiable backend servers and a raw TCP test client.

Backend behaviors must include:

- Identity and request echo.
- Delayed response headers.
- Slow streaming bodies.
- Configurable error responses.
- Mid-response disconnects.
- Accepting a connection without responding.

**Exit criterion:** Tests can deliberately reproduce slow, failed, and malformed interactions without external services.

### Milestone 1 — TCP Proxy and First Dashboard

Build:

- A TCP listener and one fixed upstream.
- Concurrent bidirectional relay.
- Connect timeout and correct half-close behavior.
- Bounded buffering and connection accounting.
- Graceful shutdown with a defined deadline.
- Read-only admin status and statistics endpoints.
- A dashboard showing running state, active connections, and bytes transferred.

**Exit criterion:** Large uploads/downloads and half-closes work; memory is bounded per connection; dashboard failure does not interrupt forwarding.

### Milestone 2 — Single-Backend HTTP/1.1 Reverse Proxy

Start with plain HTTP, one backend, one request per downstream connection, no pooling, and no retries.

Implement:

- Incremental header parsing across arbitrary TCP fragmentation.
- Explicit message framing for requests and responses.
- Content-Length and chunked bodies; close-delimited responses where applicable.
- Special handling for HEAD, informational responses, and bodyless status codes.
- Header size and parsing-time limits.
- Rejection of invalid or ambiguous framing.
- Hop-by-hop header removal, including fields named by Connection.
- Explicit upstream Host and request-target policies.
- Controlled gateway errors.
- Explicit rejection of unsupported upgrades and tunnels.

Dashboard additions: request counts, status codes, and sanitized recent request metadata.

**Exit criterion:** Valid requests behave consistently regardless of fragmentation; malformed or ambiguous messages are rejected safely.

### Milestone 3 — Streaming and Resource Safety

Implement:

- Streaming uploads and downloads with bounded buffers.
- Correct handling of early upstream responses during uploads.
- Cancellation and cleanup when either peer disconnects.
- Read/write idle timeouts and overall request deadlines.
- Request/body limits and admission limits.
- Byte and in-flight request accounting.

**Exit criterion:** Large transfers do not require whole-body buffering; slow or disconnected peers do not leak tasks, memory, or connections.

### Milestone 4 — Configuration and Routing

Implement:

- Startup configuration parsing and validation.
- Exact host matching and longest path-prefix matching.
- Explicit route precedence and unmatched-route behavior.
- A per-request context containing request ID, timing, route, backend, and attempt state as needed.

Keep routing separate from balancing:

```text
Request --> Router chooses pool --> Balancer chooses backend
```

Dashboard addition: read-only route and pool tables.

**Exit criterion:** Overlapping routes resolve deterministically and invalid configurations fail before serving traffic.

### Milestone 5 — Load Balancing

Implement in order:

1. Round-robin.
2. Weighted round-robin.
3. Least outstanding requests.
4. Consistent hashing if a concrete use case justifies it.

Filter eligible backends before applying the selection algorithm. Account for outstanding work using cancellation-safe ownership, such as a selection lease released on drop.

Dashboard additions: backend distribution, weights, and active work.

**Exit criterion:** Distribution is tested under concurrency; accounting returns to zero after completion, errors, and cancellation.

### Milestone 6 — Health and Draining

Implement:

- Active TCP checks, followed by HTTP readiness checks.
- Probe deadlines, interval jitter, and transition thresholds.
- Recovery behavior.
- Explicit all-backends-unavailable policy.
- Administrative draining distinct from health failure.
- Passive failure observations only after active health behavior is understood.

Drain means stop selecting a backend for new work while allowing existing work to finish within a defined deadline. Long-lived streams require explicit treatment.

Dashboard additions: live health, transition history, drain/enable actions, and remaining work.

**Exit criterion:** Failed backends leave service and recovered backends return predictably without rapid flapping.

### Milestone 7 — Keep-Alive and Connection Pooling

Implement:

- Downstream keep-alive.
- Upstream HTTP/1.1 reuse with one in-flight request per connection; no pipelining initially.
- Connection compatibility keys beyond just an IP address.
- Idle expiration and stale connection handling.
- Global and per-backend limits.
- Bounded pool acquisition waits.
- Reuse only after a complete, valid protocol exchange.

Dashboard additions: pool occupancy, reuse counts, and connection acquisition failures.

**Exit criterion:** Cancelled, partially consumed, or malformed exchanges never return contaminated connections to the pool.

### Milestone 8 — Error Model, Deadlines, and Retry Safety

Define distinct errors for client protocol failure, route failure, unavailable backends, connection failure, TLS failure, upstream protocol failure, timeout, cancellation, and internal failure.

Retry policy must consider:

- Method semantics and duplicate-work risk.
- Request body replayability.
- Whether the backend may already have executed the operation.
- Whether the downstream response has started.
- Remaining request deadline.
- Retry budgets and outage amplification.

Start with narrowly scoped retries, not blanket retry-on-error behavior.

**Exit criterion:** Failures have bounded latency; unsafe operations are not silently duplicated; errors after response commitment terminate appropriately.

### Milestone 9 — TLS and Additional Protocols

Implement:

- Downstream TLS termination.
- Independently configured upstream TLS.
- Certificate verification, SNI, ALPN, and handshake deadlines.
- HTTP/2 using a mature protocol library.
- WebSocket upgrade/tunneling and SSE behavior.
- gRPC once HTTP/2 operation is sound.

**Exit criterion:** Invalid upstream certificates fail verification; protocol combinations and long-lived streams have integration coverage.

### Milestone 10 — Comprehensive Management and Hardening

Implement:

- Structured access/error logs and latency histograms.
- Metrics by bounded dimensions such as route, backend, and status class.
- Readiness/liveness endpoints.
- Versioned configuration validation, review, application, and history.
- Atomic configuration publication.
- Administrative authentication and audit events.
- Graceful process draining.
- Fuzzing, interoperability testing, sustained load testing, and failure testing.

**Exit criterion:** Operators can diagnose failures, apply supported configuration changes safely, drain traffic, and restart predictably without depending on undocumented behavior.

### Later — Caching and Advanced Features

Only after the core proxy is reliable, consider:

- Bounded response caching with explicit eligibility and cache keys.
- Freshness, Vary, conditional revalidation, and stampede prevention.
- Stale serving and purging.
- More sophisticated rate limits and traffic policies.
- Zero-downtime binary upgrades.
- Multi-instance management or a distributed control plane.

Authenticated and personalized responses must not be cached by default.

## Dashboard Scope

| Page            | Purpose                                                        |
| --------------- | -------------------------------------------------------------- |
| Overview        | Traffic, error rate, latency, connections, backend summary     |
| Routes          | Matching rules and destination pools                           |
| Upstream pools  | Algorithms, backends, weights, health policies                 |
| Backend details | Health history, failures, active work, drain controls          |
| Traffic         | Bounded, sanitized request metadata with filters               |
| Configuration   | Validation, revision comparison, apply status, history         |
| Events          | Health transitions, config application, administrative actions |

Build read-only visibility before mutation controls. Charts must distinguish unavailable data from zero traffic.

Suggested starting stack:

- Rust + Tokio for the engine.
- Axum for the admin API.
- React + TypeScript + Vite for the dashboard.
- SQLite for configuration revisions and administrative records.
- Server-Sent Events for live health and operational events.
- Prometheus-compatible instrumentation for metrics.

Initially, short historical charts may use bounded in-memory samples, explicitly lost on restart. Add an external metrics store when durable historical queries become necessary.

## Administrative API Direction

Illustrative endpoints, to evolve with working features:

```text
GET  /api/v1/status
GET  /api/v1/stats
GET  /api/v1/routes
GET  /api/v1/pools
GET  /api/v1/pools/:pool/backends

POST /api/v1/config/validate
POST /api/v1/config/revisions
POST /api/v1/config/revisions/:revision/apply

POST /api/v1/pools/:pool/backends/:backend/drain
POST /api/v1/pools/:pool/backends/:backend/enable

GET  /api/v1/events
GET  /metrics
```

Administrative handlers call control operations. They must not directly manipulate routing or connection-pool internals.

## Security Baseline

- Bind the admin API to loopback initially; require authentication before remote exposure.
- Keep administrative and public proxy listeners separate.
- Protect cookie-authenticated mutations against CSRF.
- Restrict cross-origin administrative access.
- Redact credentials, cookies, tokens, and sensitive headers.
- Do not collect request bodies by default.
- Audit configuration changes and privileged operations.
- Bound logs, event history, subscriber queues, and API payload sizes.
- Do not use raw URLs, request IDs, or user IDs as metric labels.
- Define a trusted-proxy policy before accepting forwarded client identity.
- Do not allow slow dashboard subscribers to block the proxy or health checker.

## Testing Strategy

For every milestone:

1. Specify observable behavior and resource limits.
2. Write an integration test for the smallest useful slice.
3. Implement it.
4. Introduce delay, disconnects, malformed input, and cancellation.
5. Inspect logs, counters, task cleanup, and memory behavior.
6. Document the design decision and limitations.
7. Move to the next milestone only when the current behavior is reliable.

Maintain coverage for:

- Fragmentation and message boundaries.
- Partial uploads and early responses.
- Slow clients and stalled backends.
- Connection reuse and stale connections.
- Health transitions and all-backends-down behavior.
- Cancellation-safe accounting.
- Reload during active traffic.
- Admin/dashboard disconnection and failure.

Performance targets should be chosen against specified hardware and workloads. Do not claim production readiness from a single throughput benchmark.

## Code Organization

Begin with a small Rust application and clear modules. A possible later structure is:

```text
my-gateway/
├── Cargo.toml
├── crates/
│   ├── proxy-core/       # transport, HTTP, routing, balancing
│   ├── control/          # validation, revisions, management operations
│   ├── admin-api/        # administrative endpoints and SSE
│   └── gateway/          # executable and service wiring
├── dashboard/
├── test-backends/
└── tests/
```

Split crates when real boundaries justify it, not simply to imitate Pingora. The independent implementation should eventually live in its own project; this mission file does not authorize rewriting the reference repository.

## Using Pingora as Reference

Study targeted solutions after identifying the problem in our own implementation:

- Per-request context and lifecycle extension points.
- Explicit upstream peer descriptions.
- Connector and connection-pool separation.
- Background health services.
- Streaming and cancellation behavior.
- Error classification and retry boundaries.
- Graceful shutdown and operational lifecycle.

Borrow principles and evaluate trade-offs. Do not reproduce its entire module layout or API without a demonstrated need.

## First Deliverable

Build the smallest observable system:

- One fixed-upstream TCP proxy.
- Bounded bidirectional forwarding with half-close support.
- Connect timeout and graceful shutdown.
- Connection and byte counters.
- Read-only status/statistics API.
- A dashboard showing running state, connections, and traffic.
- Integration tests proving forwarding continues when the dashboard disconnects.

Then evolve the data plane and management plane together.

**Mission rule: every stage must work under failure before the next feature is added.**
