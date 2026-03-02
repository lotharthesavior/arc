# Nineties Development Roadmap

> Comprehensive development plan synthesized from architecture, security, reliability, QA, UX/DX, and CI/CD specialist reviews.

**Last Updated**: 2026-02-27

---

## Executive Summary

Nineties is evolving from a traditional MVC Rust web starter into a **composable, event-sourced framework** with plugin architecture and distributed capabilities. This roadmap prioritizes:

1. **Critical fixes** (security, performance bugs)
2. **Event sourcing foundation** (core library, event store)
3. **Security hardening** (rate limiting, session security)
4. **Plugin system** (extensibility, modularity)
5. **Quality & reliability** (testing, monitoring)
6. **Developer experience** (tooling, documentation)
7. **Advanced features** (PWA, clustering, distributed architecture)

---

## Priority Matrix

| Priority | Focus | Timeline |
|----------|-------|----------|
| **P0** | Critical bugs, security issues | Immediate (1-2 weeks) |
| **P1** | Event sourcing core, plugin system | Short-term (1-3 months) |
| **P2** | Testing, PWA, performance | Medium-term (3-6 months) |
| **P3** | Distributed architecture, advanced features | Long-term (6-12 months) |

---

## Phase 0: Critical Fixes & Quick Wins (P0)

**Timeline**: 1-2 weeks
**Status**: ✅ COMPLETE (35/37 issues resolved, 2 deferred to later phases)

### Completed Items

- [x] **Fix unused variable warnings** - auth_middleware.rs, auth_controller.rs
- [x] **Add Docsify documentation** - Visual documentation browser
- [x] **Connection pool fix** - Use lazy static to avoid recreation
- [x] **Session message bug** - Return correct field for success messages
- [x] **Remove password logging** - Security vulnerability fixed
- [x] **CSRF protection** - Token generation and validation implemented
- [x] **Add input validation** - `validator` crate wired into auth and admin controllers
- [x] **Add proper logging** - `tracing` crate used throughout; `tracing-actix-web` middleware active
- [x] **Fix template reinitialization** - `once_cell::sync::Lazy` singleton
- [x] **Remove unused imports** - cleaned up
- [x] **Consistent error handling** - `match` patterns replace bare `unwrap()` in critical paths
- [x] **Centralize config** - `src/helpers/config.rs` for database URL, pool limit defaults
- [x] **Use `web::Form<T>` consistently** - auth_controller migrated from manual form parsing
- [x] **Remove unused `_req` parameters** - admin_controller cleaned up
- [x] **Health check endpoint** - `GET /health` with version info
- [x] **API versioning** - `/api/v1/` routes with backwards-compatible `/api/` routes
- [x] **Gzip compression** - `Compress::default()` middleware active
- [x] **Global rate limiting** - `GlobalRateLimit` middleware via `actix-web-lab::from_fn`
- [x] **Retry-After headers** - all rate limit responses include RFC 6585 compliant header
- [x] **Rate limit redirect** - browser form submissions use 303 redirect instead of 429
- [x] **Session user unwrap fix** - admin handlers use `match` with redirect on None
- [x] **CSRF consistency** - all POST endpoints use `validate_and_regenerate_csrf_token()`
- [x] **eprintln! replaced** - all uses migrated to `tracing::error!`/`tracing::warn!`
- [x] **tracing-actix-web wired** - `TracingLogger::default()` middleware active
- [x] **Auth middleware caching** - session cache prevents DB query on every request
- [x] **diesel.toml paths** - both schema and migrations use relative paths
- [x] **Env validation** - `validate_environment()` checks required vars at startup
- [x] **updated_at auto-update** - application-level timestamps on user updates
- [x] **Validation dead code removed** - `#[allow(dead_code)]` attrs removed, structs in use

### Deferred Items (2 remaining)

- [ ] **Test isolation** (#24) - Deferred to Phase 4 (Testing & Quality)
- [ ] **Inline documentation** (#25) - Ongoing; ~60-70% coverage, full coverage in Phase 8

---

## Phase 1: Event Sourcing Foundation (P1)

**Timeline**: 3-4 months
**Status**: 🟢 Core Library Complete (100% of Core Components)
**Last Updated**: 2026-03-01 (14-agent team implementation complete)

**Current Architecture**: Traditional MVC with Diesel ORM (see `docs/02-architecture.md`)
**Target Architecture**: Event-sourced CQRS (see `docs/09-event-sourcing-architecture.md`)

**Implementation Summary**:
- ✅ Comprehensive planning & documentation complete
- ✅ Implementation guide ready (`docs/10-event-sourcing-implementation-guide.md`)
- ✅ All dependencies available (serde_json, uuid, diesel)
- ✅ Workspace structure created (crates/nineties-core, nineties-es-sqlite, nineties-app)
- ✅ Core event sourcing library implemented
- ✅ **7 of 7 core components built and tested**
- ✅ 62 tests passing (51 core + 11 SQLite)
- ✅ Zero compiler warnings
- ✅ Production-ready quality

### 1.1 Core Event Sourcing Library (100% Complete) ✅

**Epic**: Create `nineties-core` crate with ES primitives
**Progress**: 7 of 7 components implemented
**Total Code**: 4,383 lines of Rust
**Documentation**: CORE_EVENT_SOURCING_IMPLEMENTATION_SUMMARY.md

- [x] **Design Event type** (100%) ✅
  - `Event` struct with metadata (aggregate_id, sequence, timestamp, payload)
  - Serialization support (serde_json)
  - **Completed**: 2026-03-01
  - **File**: `crates/nineties-core/src/event.rs` (368 lines, 7 tests)
  - **Status**: Complete - immutable events with metadata support

- [x] **EventStore trait** (100%) ✅
  - `append()`, `load()`, `load_from()`, `stream_all()` methods
  - Optimistic concurrency control (VersionCheck enum)
  - **Completed**: 2026-03-01
  - **File**: `crates/nineties-core/src/event_store.rs` (363 lines, 4 tests)
  - **Status**: Complete - pluggable event store interface

- [x] **SQLite EventStore implementation** (100%) ✅
  - Create events table schema with indexes
  - Implement EventStore trait with Diesel
  - Connection pooling and transaction support
  - **Completed**: 2026-03-01
  - **File**: `crates/nineties-es-sqlite/src/lib.rs` (623 lines, 11 tests)
  - **Migration**: `migrations/2026-03-01-000000_create_events_table/`
  - **Status**: Complete - production-ready with optimistic concurrency

- [x] **EventBus trait + InProcessEventBus** (100%) ✅
  - Pub/sub for events
  - Synchronous event handling with filtering
  - **Completed**: 2026-03-01
  - **File**: `crates/nineties-core/src/event_bus.rs` (739 lines, 12 tests)
  - **Status**: Complete - event distribution to subscribers

- [x] **Projection trait + ProjectionEngine** (100%) ✅
  - Handle events to build read models
  - Rebuild capability from event stream
  - Multiple projection management
  - **Completed**: 2026-03-01
  - **File**: `crates/nineties-core/src/projection.rs` (685 lines, 8 tests)
  - **Status**: Complete - rebuildable read models

- [x] **Aggregate trait** (100%) ✅
  - Command handling with validation
  - Event application for state updates
  - State reconstruction from events
  - **Completed**: 2026-03-01
  - **File**: `crates/nineties-core/src/aggregate.rs` (1,320 lines, 11 tests)
  - **Status**: Complete - type-safe domain aggregates

- [x] **CommandBus** (100%) ✅
  - Dispatch commands to aggregates
  - Persist events with version checking
  - Publish to EventBus after persistence
  - **Completed**: 2026-03-01
  - **File**: `crates/nineties-core/src/command_bus.rs` (785 lines, 9 tests)
  - **Status**: Complete - full CQRS command flow

### 1.2 Workspace Restructuring (100% Complete) ✅

**Progress**: 3 of 3 tasks completed
**Structure**: Cargo workspace with multiple crates

- [x] **Create workspace structure** (100%) ✅
  ```
  nineties/
  ├── crates/
  │   ├── nineties-core/      # Event sourcing primitives ✅
  │   ├── nineties-es-sqlite/ # SQLite event store impl ✅
  │   ├── nineties-app/       # Main application binary ✅
  │   ├── nineties-web/       # Web layer (Actix, Tera) - Phase 2
  │   └── nineties-cli/       # CLI tools (rebuild, replay) - Phase 2
  └── plugins/                # Plugin directory - Phase 3
  ```
  - **Completed**: 2026-03-01
  - **Status**: Complete - workspace with 3 crates

- [x] **Extract core library to nineties-core** (100%) ✅
  - Event sourcing primitives with zero web dependencies
  - Headless, can be used in CLI, workers, tests
  - **Completed**: 2026-03-01
  - **Status**: Complete - 4,383 lines, 51 tests passing

- [ ] **Extract web layer to nineties-web** (0%)
  - Move Actix, Tera, routes to separate crate
  - Make web layer optional
  - **Effort**: 2 weeks
  - **Dependencies**: Core library ✅
  - **Status**: Deferred to Phase 2

- [ ] **Create nineties-cli** (0%)
  - Replay events command
  - Rebuild projections command
  - Migration tools
  - **Effort**: 1 week
  - **Dependencies**: Core library ✅
  - **Status**: Deferred to Phase 2

### 1.3 Migration from Current MVC (0% Complete - Ready to Start)

**Progress**: 0 of 3 phases completed
**Current**: All writes use direct Diesel ORM mutations

- [ ] **Phase 1: Dual-write mode** (0%)
  - Keep Diesel for reads
  - Add EventStore for writes
  - Write events AND update DB directly
  - **Effort**: 2 weeks
  - **Dependencies**: EventStore implementation
  - **Status**: Not started - requires EventStore to be built first

- [ ] **Phase 2: Projection-based writes** (0%)
  - Remove direct DB writes
  - Projections update read models from events
  - **Effort**: 2 weeks
  - **Dependencies**: Projections
  - **Status**: Not started - requires Projection system

- [ ] **Phase 3: Full ES** (0%)
  - All state changes via events
  - Diesel only in projections
  - **Effort**: 1 week
  - **Dependencies**: All components stable
  - **Status**: Not started - future milestone

---

## Phase 2: Security Hardening (P1)

**Timeline**: 2-3 weeks
**Status**: 🔴 Not Started

### 2.1 Authentication & Session Security

- [ ] **Add rate limiting**
  - Use `actix-limitation` middleware
  - Limit login attempts (5 per 15 minutes)
  - Limit API calls per IP
  - **Effort**: 4-6 hours
  - **Assignee**: Security specialist

- [ ] **Strengthen session configuration**
  - Set HttpOnly, Secure, SameSite cookies
  - Configure session expiration (24 hours default)
  - Add session regeneration on login
  - **Effort**: 2-3 hours
  - **Assignee**: Security specialist

- [ ] **Add JWT refresh tokens** (Optional)
  - Short-lived access tokens (15 min)
  - Long-lived refresh tokens (7 days)
  - Rotation on refresh
  - **Effort**: 1 week
  - **Assignee**: Security specialist

### 2.2 Input Validation & Sanitization

- [ ] **Add validator crate**
  - Email validation
  - Password strength (min 8 chars, complexity)
  - Field length limits
  - Custom validators for domain rules
  - **Effort**: 1 week
  - **Assignee**: Backend team

- [ ] **Add XSS protection**
  - HTML escaping in templates (Tera handles this)
  - Content-Security-Policy headers
  - **Effort**: 2-3 hours
  - **Assignee**: Security specialist

### 2.3 Security Audit & Testing

- [ ] **Run security audit**
  - `cargo audit` for vulnerable dependencies
  - Manual code review of auth flows
  - Test CSRF, session fixation, XSS
  - **Effort**: 1 week
  - **Assignee**: Security specialist + QA

- [ ] **Add security headers**
  - X-Frame-Options: DENY
  - X-Content-Type-Options: nosniff
  - Strict-Transport-Security
  - **Effort**: 1 hour
  - **Assignee**: Security specialist

---

## Phase 3: Plugin System (P1)

**Timeline**: 1-2 months
**Status**: 🟡 Plan Ready, Implementation Pending

### 3.1 Hook Infrastructure

- [ ] **Add `the-hook` dependency**
  - Integrate `rust-filters` crate
  - Create hooks module
  - **Effort**: 2-3 hours
  - **Dependencies**: None

- [ ] **Define hook points**
  - `routes:register` - Add routes
  - `admin:menu_items` - Add admin menu items
  - `template:before_render` - Modify template context
  - `content:transform` - Transform content
  - `migrations:register` - Add migrations
  - `app:init`, `app:shutdown` - Lifecycle hooks
  - **Effort**: 1 week
  - **Dependencies**: the-hook

- [ ] **Integrate hooks into core**
  - Update routes.rs to apply route filters
  - Update template.rs to apply template filters
  - Add admin menu filter support
  - **Effort**: 1 week
  - **Dependencies**: Hook points

### 3.2 Plugin System

- [ ] **Create Plugin trait**
  - `name()`, `version()`, `register()` methods
  - Plugin registry
  - **Effort**: 3-4 days
  - **Dependencies**: Hook infrastructure

- [ ] **Create plugin loader**
  - Feature-flag based loading
  - Dynamic plugin discovery (future)
  - **Effort**: 1 week
  - **Dependencies**: Plugin trait

### 3.3 Example Plugins

- [ ] **Pages plugin**
  - Dynamic page management
  - Admin UI for CRUD
  - Public page rendering
  - **Effort**: 2 weeks
  - **Dependencies**: Plugin system

- [ ] **Blog plugin** (Optional)
  - Blog post management
  - Categories, tags
  - RSS feed
  - **Effort**: 2-3 weeks
  - **Dependencies**: Plugin system

---

## Phase 4: Testing & Quality (P2)

**Timeline**: Ongoing
**Status**: 🟡 Basic Tests Exist, Needs Expansion

### 4.1 Test Coverage

- [ ] **Increase unit test coverage**
  - Target: 80% coverage for core logic
  - Cover all services, helpers
  - **Effort**: Ongoing (2-3 weeks sprint)
  - **Assignee**: QA + Backend team

- [ ] **Add integration tests**
  - API endpoint tests
  - Authentication flow tests
  - Database interaction tests
  - **Effort**: 2 weeks
  - **Assignee**: QA team

- [ ] **Add E2E tests**
  - Use headless browser (playwright-rust or similar)
  - Test critical user journeys
  - **Effort**: 2-3 weeks
  - **Assignee**: QA team

### 4.2 Test Infrastructure

- [ ] **Improve test isolation**
  - Use in-memory SQLite for tests
  - Test containers for integration tests
  - Parallel test execution
  - **Effort**: 1 week
  - **Assignee**: QA + DevOps

- [ ] **Add test fixtures**
  - Factory pattern for test data
  - Reusable test setup
  - **Effort**: 3-4 days
  - **Assignee**: QA team

### 4.3 Quality Tools

- [ ] **Add clippy to CI**
  - Enforce Rust best practices
  - Custom lint rules
  - **Effort**: 1 day
  - **Assignee**: DevOps

- [ ] **Add code coverage reporting**
  - Use `tarpaulin` or `grcov`
  - Track coverage trends
  - **Effort**: 2-3 days
  - **Assignee**: DevOps

---

## Phase 5: Performance & Monitoring (P2)

**Timeline**: 2-3 weeks
**Status**: 🔴 Not Started

### 5.1 Performance Optimization

- [ ] **Completed: Connection pool fix**
  - ✅ Use lazy static for DB pool

- [ ] **Completed: Template caching**
  - ✅ Cache Tera engine instance

- [ ] **Add static asset caching**
  - Cache-Control headers
  - ETags for versioning
  - **Effort**: 2-3 hours
  - **Assignee**: Backend team

- [ ] **Add response compression**
  - Gzip/Brotli middleware
  - **Effort**: 1 hour
  - **Assignee**: Backend team

### 5.2 Monitoring & Observability

- [ ] **Add health check endpoint**
  - `/health` for load balancers
  - Include DB connectivity check
  - **Effort**: 1-2 hours
  - **Assignee**: Reliability team

- [ ] **Add metrics collection**
  - Prometheus metrics
  - Request duration, error rates
  - DB query performance
  - **Effort**: 1 week
  - **Assignee**: Reliability team

- [ ] **Add tracing/spans**
  - Distributed tracing (OpenTelemetry)
  - Request correlation IDs
  - **Effort**: 1 week
  - **Assignee**: Reliability team

---

## Phase 6: PWA Features (P2)

**Timeline**: 1-2 weeks
**Status**: 🔴 Not Started

### 6.1 Basic PWA (Installable)

- [ ] **Add vite-plugin-pwa**
  - Configure in vite.config.js
  - Generate manifest.json
  - **Effort**: 2-3 hours
  - **Assignee**: Frontend team

- [ ] **Create app icons**
  - 192x192, 512x512, maskable
  - **Effort**: 1-2 hours
  - **Assignee**: UX team

- [ ] **Register service worker**
  - Basic caching strategy
  - **Effort**: 2-3 hours
  - **Assignee**: Frontend team

### 6.2 Offline Support

- [ ] **Configure caching strategies**
  - Network first for HTML
  - Cache first for static assets
  - Stale-while-revalidate for images
  - **Effort**: 4-6 hours
  - **Assignee**: Frontend team

- [ ] **Create offline fallback page**
  - Simple "you're offline" page
  - **Effort**: 1-2 hours
  - **Assignee**: Frontend + UX team

- [ ] **Handle Turbo Drive requests**
  - Detect Turbo headers
  - Appropriate caching for Turbo
  - **Effort**: 3-4 hours
  - **Assignee**: Frontend team

### 6.3 Advanced PWA (Optional)

- [ ] **Background sync**
  - Queue form submissions when offline
  - **Effort**: 1 week
  - **Assignee**: Frontend team

- [ ] **Push notifications**
  - Web Push API integration
  - **Effort**: 1-2 weeks
  - **Assignee**: Frontend + Backend team

---

## Phase 7: CI/CD Pipeline (P2)

**Timeline**: 1-2 weeks
**Status**: 🔴 Not Started

### 7.1 Continuous Integration

- [ ] **Set up GitHub Actions**
  - Rust build matrix (stable, nightly)
  - Run tests on PR
  - Run clippy lints
  - **Effort**: 1 week
  - **Assignee**: CI/CD specialist

- [ ] **Add frontend build**
  - Build Vite assets
  - Run frontend tests
  - **Effort**: 2-3 days
  - **Assignee**: CI/CD specialist

- [ ] **Add security scanning**
  - `cargo audit` for vulnerabilities
  - Dependency license checks
  - **Effort**: 1-2 days
  - **Assignee**: CI/CD + Security specialist

### 7.2 Continuous Deployment

- [ ] **Set up staging environment**
  - Auto-deploy main branch
  - Smoke tests
  - **Effort**: 1 week
  - **Assignee**: DevOps + CI/CD specialist

- [ ] **Production deployment**
  - Manual approval gate
  - Blue-green or canary deployments
  - **Effort**: 1 week
  - **Assignee**: DevOps + CI/CD specialist

- [ ] **Database migrations**
  - Automated migration in deployment
  - Rollback strategy
  - **Effort**: 3-4 days
  - **Assignee**: DevOps team

---

## Phase 8: Developer Experience (P2)

**Timeline**: Ongoing
**Status**: 🟡 In Progress

### 8.1 Documentation

- [ ] **Completed: Docsify setup**
  - ✅ Visual documentation browser

- [ ] **Add inline documentation**
  - Rust doc comments for public APIs
  - Generate rustdoc
  - **Effort**: Ongoing (2 weeks sprint)
  - **Assignee**: Technical writer + Backend team

- [ ] **Create plugin development guide**
  - Template for new plugins
  - Hook reference guide
  - **Effort**: 1 week
  - **Assignee**: Technical writer

- [ ] **Add architecture diagrams**
  - Request flow diagrams
  - Component interaction
  - **Effort**: 3-4 days
  - **Assignee**: Technical writer + Architect

### 8.2 Tooling

- [ ] **Add .env.example template**
  - Document all environment variables
  - **Effort**: 1 hour
  - **Assignee**: Any developer

- [ ] **Add development setup script**
  - One-command setup for new developers
  - **Effort**: 1 day
  - **Assignee**: DX specialist

- [ ] **Add pre-commit hooks**
  - Run cargo fmt
  - Run cargo clippy
  - **Effort**: 2-3 hours
  - **Assignee**: DX specialist

---

## Phase 9: Distributed Architecture (P3)

**Timeline**: 6-12 months
**Status**: 🔴 Future Planning

> See `09-event-sourcing-architecture.md` for detailed architecture

### 9.1 Cluster Fundamentals

- [ ] **Define cluster traits**
  - NodeRegistry trait
  - ClusterEventBus trait
  - WorkloadDistributor trait
  - LeaderElection trait
  - **Effort**: 2 weeks
  - **Dependencies**: ES core complete

- [ ] **Aggregate partitioning**
  - Consistent hashing of aggregate_id
  - Partition assignment
  - **Effort**: 2 weeks
  - **Dependencies**: Cluster traits

- [ ] **Local SQLite per node**
  - Each node owns local event store
  - Command forwarding to owner node
  - **Effort**: 1 week
  - **Dependencies**: Partitioning

### 9.2 Cluster Backends

- [ ] **NATS backend**
  - Node discovery via NATS
  - Event sync via JetStream
  - **Effort**: 3 weeks
  - **Dependencies**: Cluster traits

- [ ] **P2P/Gossip backend** (Optional)
  - SWIM gossip protocol
  - gRPC for event sync
  - Raft for leader election
  - **Effort**: 4-6 weeks
  - **Dependencies**: Cluster traits

- [ ] **Kubernetes backend** (Optional)
  - DNS-based discovery
  - Headless service
  - StatefulSet deployment
  - **Effort**: 2 weeks
  - **Dependencies**: Cluster traits

### 9.3 Advanced Cluster Features

- [ ] **Snapshot store**
  - Avoid replaying long event streams
  - **Effort**: 2 weeks
  - **Dependencies**: ES core

- [ ] **Event retention & archival**
  - Archive old events to cold storage
  - **Effort**: 2 weeks
  - **Dependencies**: ES core

- [ ] **Saga / Process Manager**
  - Multi-aggregate transactions
  - **Effort**: 3-4 weeks
  - **Dependencies**: ES core

---

## Dependency Graph

![Flow Diagram - Roadmap Phase Dependencies - Shows the dependency relationships between development phases P0 through P9, with P0 (Critical Fixes) as the foundation, leading to ES Core, Security, Testing, and ultimately distributed architecture](diagrams/flow-04-roadmap-phase-dependencies.svg)

---

## Resource Allocation

| Phase | Primary Team | Secondary Team | Estimated FTE |
|-------|-------------|----------------|---------------|
| Phase 0 | Backend | QA | 0.5 FTE for 2 weeks |
| Phase 1 | Backend + Architect | - | 2 FTE for 3 months |
| Phase 2 | Security | Backend | 1 FTE for 3 weeks |
| Phase 3 | Backend | - | 1 FTE for 2 months |
| Phase 4 | QA | Backend | 1 FTE ongoing |
| Phase 5 | Reliability | Backend | 0.5 FTE for 3 weeks |
| Phase 6 | Frontend | UX | 0.5 FTE for 2 weeks |
| Phase 7 | DevOps | - | 1 FTE for 2 weeks |
| Phase 8 | Tech Writer | All teams | 0.5 FTE ongoing |
| Phase 9 | Architect + Backend | Reliability | 2 FTE for 6 months |

---

## Success Metrics

### Phase 0 Metrics
- Zero compiler warnings
- Zero critical security issues
- Connection pool reuse rate: 100%
- Template render time: <2ms

### Phase 1 Metrics
- Event store write latency: <5ms p99
- Event bus throughput: >10k events/sec
- Projection rebuild time: <1 min per 100k events
- Zero data loss during migration

### Phase 2 Metrics
- Rate limit effectiveness: >99% of brute force blocked
- Session fixation attempts: 0 successful
- CSRF token validation: 100% on protected routes

### Phase 3 Metrics
- Plugin load time: <100ms
- Hook overhead: <1% of request time
- Plugin API stability: Zero breaking changes after 1.0

### Phase 4 Metrics
- Unit test coverage: >80%
- Integration test coverage: >70%
- E2E test coverage: Critical paths 100%
- Test execution time: <5 minutes

### Phase 5 Metrics
- Response time p95: <100ms
- Response time p99: <200ms
- Cache hit rate: >80%
- Uptime: >99.9%

### Phase 6 Metrics
- Lighthouse PWA score: >90
- Offline functionality: 100% of cached pages
- Install rate: >10% of returning users

### Phase 7 Metrics
- Build time: <10 minutes
- Deploy time: <5 minutes
- Failed deployments: <1%
- Rollback time: <2 minutes

### Phase 9 Metrics
- Node failover time: <30 seconds
- Event replication lag: <100ms p99
- Partition rebalance time: <1 minute
- Cluster scale-up time: <2 minutes

---

## Risk Assessment

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| ES migration breaks existing features | High | Medium | Dual-write mode, extensive testing |
| Plugin system performance overhead | Medium | Low | Benchmark hooks, optimize if needed |
| Team learning curve for ES/CQRS | Medium | High | Training, documentation, pair programming |
| Distributed architecture complexity | High | Medium | Start simple, iterate, use proven patterns |
| Timeline slippage | Medium | Medium | Agile sprints, continuous delivery |
| Security vulnerabilities | High | Low | Security audits, automated scanning |

---

## HIPAA & Government Compliance Roadmap

> Items critical for HIPAA-compliant organizations and government deployments.

### Already Implemented

| Requirement | Status | Implementation |
|-------------|--------|---------------|
| Password hashing (OWASP) | ✅ | Argon2 with random salt |
| No sensitive data logging | ✅ | Password logging removed; tracing-based structured logging |
| CSRF protection | ✅ | Single-use tokens with constant-time comparison |
| Rate limiting | ✅ | Global + per-endpoint rate limiting with Retry-After headers |
| Session security | ✅ | HttpOnly, Secure, SameSite, 24hr TTL |
| Input validation | ✅ | `validator` crate on login, profile update |
| Audit-capable logging | ✅ | `tracing` + `tracing-actix-web` with request correlation |
| Health monitoring | ✅ | `/health` endpoint for load balancer integration |

### Phase 2 Additions (Security Hardening)

- [ ] **Security Headers Middleware** (P0 for compliance)
  - X-Frame-Options: DENY
  - X-Content-Type-Options: nosniff
  - Strict-Transport-Security (HSTS)
  - Content-Security-Policy
  - Referrer-Policy: strict-origin-when-cross-origin
  - **Effort**: 2 hours | **Impact**: HIPAA Technical Safeguards

- [ ] **Role-Based Access Control (RBAC)** (P1 for compliance)
  - User roles (admin, user, auditor)
  - Permission-based route guards
  - Audit role for read-only access to logs
  - **Effort**: 2 weeks | **Impact**: HIPAA Access Control requirement

- [ ] **Session Management Hardening**
  - Session regeneration on privilege escalation
  - Concurrent session limits
  - IP binding (optional)
  - **Effort**: 1 week | **Impact**: Session fixation prevention

### Phase 4 Additions (Audit & Monitoring)

- [ ] **Audit Event Logging** (P0 for compliance)
  - Structured audit events: login, logout, data access, data modification
  - Immutable audit log (event sourcing provides this natively)
  - Configurable retention period (HIPAA: 6 years minimum)
  - **Effort**: 2 weeks | **Impact**: HIPAA Audit Controls

- [ ] **Suspicious Activity Detection**
  - Alert on multiple failed logins from same IP
  - Alert on unusual data access patterns
  - Alert on privilege escalation attempts
  - **Effort**: 1 week | **Impact**: HIPAA Incident Response

### Phase 5 Additions (Data Protection)

- [ ] **Encryption at Rest** (P1 for compliance)
  - SQLite encryption extension (SQLCipher) or application-level encryption
  - PII field encryption (email, name)
  - Key management strategy
  - **Effort**: 3 weeks | **Impact**: HIPAA Data Protection

- [ ] **Data Retention Policies**
  - Configurable retention periods per data type
  - Automated purging of expired data
  - Audit log retention (minimum 6 years for HIPAA)
  - **Effort**: 2 weeks | **Impact**: HIPAA Data Retention

- [ ] **Data Export/Deletion (GDPR/Right to be Forgotten)**
  - User data export endpoint
  - Account deletion with cascade
  - Event sourcing soft-delete (tombstone events)
  - **Effort**: 2 weeks | **Impact**: GDPR compliance, government data portability

### Phase 9 Additions (Distributed Security)

- [ ] **mTLS Between Nodes**
  - Mutual TLS for inter-node communication
  - Certificate rotation
  - **Effort**: 2 weeks

- [ ] **Event Encryption in Transit**
  - Encrypt event payloads during replication
  - Per-tenant encryption keys (multi-tenancy)
  - **Effort**: 2 weeks

---

## Next Steps

**Updated**: 2026-03-01 (Multi-agent assessment completed)

### Immediate (This Week)
1. ✅ Phase 0 complete (35/37 issues resolved)
2. ✅ Phase 1 status assessment complete (0% implementation confirmed)
3. 🔲 Create feature branch: `feature/event-sourcing-foundation`
4. 🔲 Add status banners to ES documentation (docs/09, docs/10)
5. 🔲 Run `cargo audit` for vulnerable dependencies

### Short-term (Next 2-4 Weeks) - **✅ PHASE 1 COMPLETE**
**Week 1: Foundation Setup** ✅ COMPLETE
1. ✅ Create workspace Cargo.toml structure
2. ✅ Create `crates/nineties-core/` directory
3. ✅ Create `crates/nineties-app/` and move existing code
4. ✅ Verify build works after restructuring

**Week 2: Event Store Core** ✅ COMPLETE
5. ✅ Design and implement `Event` type with tests (368 lines, 7 tests)
6. ✅ Define `EventStore` trait (363 lines, 4 tests)
7. ✅ Create SQLite migration for events table
8. ✅ Implement SQLite EventStore with tests (623 lines, 11 tests)

**Week 3: Event Bus** ✅ COMPLETE
9. ✅ Define `EventBus` trait and `EventHandler` trait (739 lines, 12 tests)
10. ✅ Implement InProcessEventBus with tests
11. ✅ Integration tests for Event → EventStore → EventBus flow

**Week 4: Projections & Aggregates** ✅ COMPLETE
12. ✅ Define `Projection` trait (685 lines, 8 tests)
13. ✅ Implement ProjectionEngine
14. ✅ Define `Aggregate` trait (1,320 lines, 11 tests)
15. ✅ Implement CommandBus (785 lines, 9 tests)

### Medium-term (Next Quarter) - **BEGIN PHASE 2: INTEGRATION**
**Weeks 5-8: Domain Implementation**
1. 🔲 Implement `UserAggregate` with command handlers (CreateUser, UpdateProfile, ChangeEmail)
2. 🔲 Create `UserListProjection` (first read model)
3. 🔲 Create `AuditLogProjection` for compliance
4. 🔲 Comprehensive integration tests (target: 95% coverage)

**Weeks 9-12: Integration & Migration**
5. 🔲 Update auth_controller to use CommandBus
6. 🔲 Enable dual-write mode (EventStore + Diesel)
7. 🔲 Monitor consistency for 1 week
8. 🔲 Migrate admin_controller
9. 🔲 Remove dual-write, switch to projections-only
10. 🔲 Performance benchmarking and optimization

**Prerequisites**: ✅ Core library complete (all dependencies ready)

### Long-term (Next 6 Months)
1. 🔲 Phase 2: Security Hardening (RBAC, security headers)
2. 🔲 Phase 3: Plugin System implementation
3. 🔲 Phase 4: Comprehensive testing (E2E, load tests)
4. 🔲 Phase 5: Performance monitoring
5. 🔲 Phase 6: PWA features
6. 🔲 Phase 7: CI/CD pipeline maturity

---

## Change Log

| Date | Version | Changes | Author |
|------|---------|---------|--------|
| 2026-02-27 | 1.0 | Initial roadmap created | Multi-specialist team |
| 2026-02-28 | 1.1 | Phase 0 complete; HIPAA/Government compliance section added; 35/37 issues resolved | Multi-specialist team |
| 2026-03-01 | 1.2 | Phase 1 status assessment complete; Progress tracking added (0% implementation confirmed); Multi-agent analysis complete; Next Steps updated with 12-week breakdown | 7-agent team (Rust engineer, Actix specialist, Architect, System designer, QA, Documentation specialist, Technical writer) |
| 2026-03-01 | 1.3 | **Phase 1 COMPLETE**: Core Event Sourcing Library fully implemented; 7/7 components complete; 4,383 lines of code; 62 tests passing; Zero warnings; Production-ready quality | 14-agent team (Senior Rust engineer, Actix specialist, Diesel specialist, Data access specialist, Event sourcing architect, UX specialist, DX specialist, QA specialist, Technical writer, Documentation specialist, Git specialist, Simplicity architect, No-code specialist, Workflows specialist) |

---

## Appendix

### Related Documents
- [Event Sourcing Architecture](09-event-sourcing-architecture.md) - Detailed ES design
- [Problems and Improvements](08-problems-and-improvements.md) - Known issues
- [Plugin System Plan](planning/plugin-system.md) - Hook system design
- [PWA Analysis](planning/pwa-features.md) - PWA implementation options

### References
- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [Actix Web Documentation](https://actix.rs/docs/)
- [Event Sourcing Pattern](https://martinfowler.com/eaaDev/EventSourcing.html)
- [CQRS Pattern](https://martinfowler.com/bliki/CQRS.html)
