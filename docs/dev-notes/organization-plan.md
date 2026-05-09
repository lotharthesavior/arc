# Documentation Organization Plan

**Date**: 2026-03-01
**Agent**: Agent 9 (Technical Writer)
**Status**: Ready for Execution

---

## Executive Summary

This plan reorganizes 14 documentation files from the repository root into a logical, navigable structure within the `/docs` directory. Only README.md will remain at root level.

---

## Current State

### Root Level Files (14 files)
```
├── CORE_EVENT_SOURCING_IMPLEMENTATION_SUMMARY.md (16KB)
├── QA_REPORT.md (8KB)
├── PHASE_1_ASSESSMENT.md (13KB)
├── IMPLEMENTATION_SUMMARY.md (17KB)
├── SESSION_FIX_SUMMARY.md (10KB)
├── PROGRESS.md (12KB)
├── PLAN.md (5KB)
├── async-hook-plan.md (6KB)
├── plugin-system-plan.md (16KB)
├── creating-dynamic-page.md (13KB)
├── performance-analysis.md (11KB)
├── pwa-analysis.md (10KB)
├── diagram-catalog.md (3KB)
├── prompt.md (1KB)
└── README.md (8KB) ← STAYS AT ROOT
```

### Current docs/ Structure
```
docs/
├── 01-overview.md through 11-*.md
├── ARCHITECTURE_DIAGRAM_UPDATE.md
├── CLAUDE.md
├── COMPILATION_FIX_RATE_LIMIT_MIDDLEWARE.md
├── DIAGRAM_CONVERSION_SUMMARY.md
├── README.md
├── RELEASE_PROCESS.md
├── SESSION_CONFIGURATION.md
├── roadmap.md
├── _coverpage.md
├── _sidebar.md
├── diagrams/
│   ├── source/
│   ├── *.svg files
│   └── *.md files
└── imgs/
```

---

## Target State

### New Directory Structure
```
docs/
├── Core Documentation (existing)
│   ├── 01-overview.md
│   ├── 02-architecture.md
│   ├── ... (existing numbered docs)
│   ├── roadmap.md
│   ├── RELEASE_PROCESS.md
│   └── SESSION_CONFIGURATION.md
│
├── implementation/          ← NEW
│   ├── README.md           (index of implementation docs)
│   ├── event-sourcing-core.md (renamed from CORE_EVENT_SOURCING_IMPLEMENTATION_SUMMARY.md)
│   ├── multi-agent-summary.md (renamed from IMPLEMENTATION_SUMMARY.md)
│   ├── session-fix.md (renamed from SESSION_FIX_SUMMARY.md)
│   └── progress-report.md (renamed from PROGRESS.md)
│
├── qa/                     ← NEW
│   ├── README.md           (index of QA reports)
│   ├── qa-report.md (renamed from QA_REPORT.md)
│   └── phase-1-assessment.md (renamed from PHASE_1_ASSESSMENT.md)
│
├── planning/               ← NEW
│   ├── README.md           (index of planning docs)
│   ├── compiler-warnings-fix.md (renamed from PLAN.md)
│   ├── async-hooks.md (renamed from async-hook-plan.md)
│   ├── plugin-system.md (renamed from plugin-system-plan.md)
│   ├── dynamic-pages.md (renamed from creating-dynamic-page.md)
│   └── pwa-features.md (renamed from pwa-analysis.md)
│
├── analysis/               ← NEW
│   ├── README.md           (index of analysis docs)
│   └── performance.md (renamed from performance-analysis.md)
│
├── dev-notes/              ← NEW
│   ├── README.md           (working notes index)
│   └── prompt-notes.md (renamed from prompt.md)
│
├── diagrams/
│   ├── catalog.md (moved from root diagram-catalog.md)
│   ├── source/
│   └── ... (existing diagram files)
│
├── Technical Metadata (existing)
│   ├── CLAUDE.md
│   ├── ARCHITECTURE_DIAGRAM_UPDATE.md
│   ├── COMPILATION_FIX_RATE_LIMIT_MIDDLEWARE.md
│   └── DIAGRAM_CONVERSION_SUMMARY.md
│
└── Docsify Config (existing)
    ├── _sidebar.md
    ├── _coverpage.md
    ├── README.md
    └── imgs/
```

---

## File Categorization & Moves

### Category 1: Implementation Documentation
**Target**: `docs/implementation/`
**Description**: Summaries of completed implementation work, progress reports

| Original File | New Name | New Path | Size |
|--------------|----------|----------|------|
| CORE_EVENT_SOURCING_IMPLEMENTATION_SUMMARY.md | event-sourcing-core.md | docs/implementation/ | 16KB |
| IMPLEMENTATION_SUMMARY.md | multi-agent-summary.md | docs/implementation/ | 17KB |
| SESSION_FIX_SUMMARY.md | session-fix.md | docs/implementation/ | 10KB |
| PROGRESS.md | progress-report.md | docs/implementation/ | 12KB |

**Rationale**: These are all post-implementation summaries documenting what was built and how.

---

### Category 2: QA & Assessment Reports
**Target**: `docs/qa/`
**Description**: Quality assurance reports, assessments, test results

| Original File | New Name | New Path | Size |
|--------------|----------|----------|------|
| QA_REPORT.md | qa-report.md | docs/qa/ | 8KB |
| PHASE_1_ASSESSMENT.md | phase-1-assessment.md | docs/qa/ | 13KB |

**Rationale**: Both are QA/assessment documents evaluating implementation status and quality.

---

### Category 3: Planning & Design Documents
**Target**: `docs/planning/`
**Description**: Future plans, design proposals, feature planning

| Original File | New Name | New Path | Size |
|--------------|----------|----------|------|
| PLAN.md | compiler-warnings-fix.md | docs/planning/ | 5KB |
| async-hook-plan.md | async-hooks.md | docs/planning/ | 6KB |
| plugin-system-plan.md | plugin-system.md | docs/planning/ | 16KB |
| creating-dynamic-page.md | dynamic-pages.md | docs/planning/ | 13KB |
| pwa-analysis.md | pwa-features.md | docs/planning/ | 10KB |

**Rationale**: All describe planned features or improvements not yet implemented.

---

### Category 4: Technical Analysis
**Target**: `docs/analysis/`
**Description**: Performance analysis, benchmarking, technical comparisons

| Original File | New Name | New Path | Size |
|--------------|----------|----------|------|
| performance-analysis.md | performance.md | docs/analysis/ | 11KB |

**Rationale**: Standalone technical analysis document, warrants its own category for future analyses.

---

### Category 5: Developer Notes
**Target**: `docs/dev-notes/`
**Description**: Working notes, prompts, temporary documentation

| Original File | New Name | New Path | Size |
|--------------|----------|----------|------|
| prompt.md | prompt-notes.md | docs/dev-notes/ | 1KB |

**Rationale**: Appears to be working notes or prompts, not formal documentation.

---

### Category 6: Diagram Documentation
**Target**: `docs/diagrams/`
**Description**: Diagram catalog and references

| Original File | New Name | New Path | Size |
|--------------|----------|----------|------|
| diagram-catalog.md | catalog.md | docs/diagrams/ | 3KB |

**Rationale**: Belongs with other diagram documentation in the diagrams/ directory.

---

### Stay at Root
**File**: `README.md`
**Rationale**: Primary entry point for the repository, must stay at root per convention.

---

## Index Files to Create

### 1. docs/implementation/README.md
```markdown
# Implementation Documentation

Summaries and reports of completed implementation work.

## Documents

- [Event Sourcing Core Library](event-sourcing-core.md) - Phase 1 core library implementation
- [Multi-Agent Session Summary](multi-agent-summary.md) - Comprehensive multi-agent implementation
- [Session Configuration Fix](session-fix.md) - Network session authentication fix
- [Progress Report](progress-report.md) - Overall project progress (2026-02-27)

## Navigation

- [Back to Documentation Home](../README.md)
- [QA Reports](../qa/)
- [Planning Documents](../planning/)
```

### 2. docs/qa/README.md
```markdown
# Quality Assurance Reports

QA reports, assessments, and test results.

## Documents

- [QA Report](qa-report.md) - Post Event Sourcing implementation QA
- [Phase 1 Assessment](phase-1-assessment.md) - Multi-agent assessment of Phase 1

## Navigation

- [Back to Documentation Home](../README.md)
- [Implementation Docs](../implementation/)
```

### 3. docs/planning/README.md
```markdown
# Planning & Design Documents

Feature plans, design proposals, and architectural decisions.

## Documents

- [Compiler Warnings Fix Plan](compiler-warnings-fix.md) - Plan to address compiler warnings
- [Async Hooks](async-hooks.md) - Async and multi-process plan for hooks
- [Plugin System](plugin-system.md) - Hookable plugin architecture
- [Dynamic Pages](dynamic-pages.md) - Dynamic page creation feature
- [PWA Features](pwa-features.md) - Progressive Web App analysis and planning

## Navigation

- [Back to Documentation Home](../README.md)
- [Roadmap](../roadmap.md)
```

### 4. docs/analysis/README.md
```markdown
# Technical Analysis

Performance analysis, benchmarks, and technical comparisons.

## Documents

- [Performance Analysis](performance.md) - Arc vs Minimal TCP Server comparison

## Navigation

- [Back to Documentation Home](../README.md)
```

### 5. docs/dev-notes/README.md
```markdown
# Developer Notes

Working notes, prompts, and temporary documentation.

## Documents

- [Prompt Notes](prompt-notes.md) - Development prompts and notes

## Navigation

- [Back to Documentation Home](../README.md)
```

---

## Cross-Reference Updates

### Files with References to Update

1. **CORE_EVENT_SOURCING_IMPLEMENTATION_SUMMARY.md**
   - References: docs/09, docs/10, docs/roadmap.md
   - Update to: ../09-..., ../10-..., ../roadmap.md

2. **QA_REPORT.md**
   - No external references found

3. **PHASE_1_ASSESSMENT.md**
   - References: docs/02-architecture.md, docs/09, docs/10, docs/roadmap.md, PROGRESS.md
   - Update to: ../02-architecture.md, ../09..., ../10..., ../roadmap.md, ../implementation/progress-report.md

4. **IMPLEMENTATION_SUMMARY.md**
   - References: docs/_sidebar.md
   - Update to: ../_sidebar.md

5. **docs/09-event-sourcing-architecture.md**
   - May reference root-level planning docs
   - Update references to new planning/ locations

6. **docs/10-event-sourcing-implementation-guide.md**
   - May reference root-level docs
   - Update references accordingly

7. **docs/roadmap.md**
   - May reference planning docs
   - Update to planning/ locations

---

## Sidebar Navigation Update

Update `docs/_sidebar.md` with new structure:

```markdown
* Getting Started
  * [Overview](01-overview.md)
  * [Architecture](02-architecture.md)

* Development
  * [Backend](03-backend.md)
  * [Frontend](04-frontend.md)
  * [Database](05-database.md)
  * [Testing](06-testing.md)
  * [Session Configuration](SESSION_CONFIGURATION.md)

* Reference
  * [API Reference](07-api-reference.md)
  * [Problems & Improvements](08-problems-and-improvements.md)

* Architecture & Planning
  * [Event Sourcing Architecture](09-event-sourcing-architecture.md)
  * [Event Sourcing Implementation Guide](10-event-sourcing-implementation-guide.md)
  * [Roadmap](roadmap.md)
  * [Release Process](RELEASE_PROCESS.md)
  * [Planning Docs](planning/)

* Implementation
  * [Implementation Summaries](implementation/)
  * [Progress Report](implementation/progress-report.md)

* Quality Assurance
  * [QA Reports](qa/)

* Analysis & Planning
  * [Technical Analysis](analysis/)
  * [Planning Documents](planning/)

* Diagrams
  * [Diagram Catalog](diagrams/catalog.md)
  * [Architecture Diagrams](diagrams/)

* [GitHub](https://github.com/lotharthesavior/arc)
```

---

## Root README Enhancement

Add documentation section to root README.md:

```markdown
## Documentation

Comprehensive documentation is available in the `/docs` directory and can be browsed interactively:

```bash
npm run docs:serve
```

Then visit http://localhost:3000

### Documentation Structure

- **[Getting Started](docs/01-overview.md)** - Quick start and overview
- **[Architecture](docs/02-architecture.md)** - System architecture and design
- **[Development Guides](docs/03-backend.md)** - Backend, frontend, database guides
- **[API Reference](docs/07-api-reference.md)** - Complete API documentation
- **[Roadmap](docs/roadmap.md)** - Project roadmap and future plans

### Additional Documentation

- **[Implementation Reports](docs/implementation/)** - Implementation summaries and progress
- **[QA Reports](docs/qa/)** - Quality assurance and assessment reports
- **[Planning Documents](docs/planning/)** - Feature plans and design proposals
- **[Technical Analysis](docs/analysis/)** - Performance analysis and benchmarks
- **[Diagrams](docs/diagrams/)** - Architecture and flow diagrams

For AI agents working with documentation, see [docs/CLAUDE.md](docs/CLAUDE.md).
```

---

## Execution Checklist

### Phase 1: Directory Setup
- [ ] Create `docs/implementation/`
- [ ] Create `docs/qa/`
- [ ] Create `docs/planning/`
- [ ] Create `docs/analysis/`
- [ ] Create `docs/dev-notes/`

### Phase 2: Create Index Files
- [ ] Create `docs/implementation/README.md`
- [ ] Create `docs/qa/README.md`
- [ ] Create `docs/planning/README.md`
- [ ] Create `docs/analysis/README.md`
- [ ] Create `docs/dev-notes/README.md`

### Phase 3: Move & Rename Files
- [ ] Move implementation docs (4 files)
- [ ] Move QA reports (2 files)
- [ ] Move planning docs (5 files)
- [ ] Move analysis docs (1 file)
- [ ] Move dev notes (1 file)
- [ ] Move diagram catalog (1 file)

### Phase 4: Update Cross-References
- [ ] Update references in event-sourcing-core.md
- [ ] Update references in phase-1-assessment.md
- [ ] Update references in multi-agent-summary.md
- [ ] Update references in docs/09-event-sourcing-architecture.md
- [ ] Update references in docs/10-event-sourcing-implementation-guide.md
- [ ] Update references in docs/roadmap.md

### Phase 5: Update Navigation
- [ ] Update docs/_sidebar.md
- [ ] Enhance root README.md with documentation section

### Phase 6: Verification
- [ ] Verify all files moved successfully
- [ ] Verify no broken links
- [ ] Test Docsify navigation
- [ ] Verify root directory is clean (only README.md)

---

## Benefits

1. **Clean Root Directory** - Only README.md at root, clear entry point
2. **Logical Organization** - Related documents grouped by purpose
3. **Easy Navigation** - Clear categories and index files
4. **Maintainability** - New documents have obvious home
5. **Discoverability** - Index files help users find what they need
6. **Professional Appearance** - Well-organized structure shows maturity

---

## Risks & Mitigation

### Risk 1: Broken Links
**Mitigation**: Comprehensive grep for cross-references before and after move, test all links

### Risk 2: Git History Loss
**Mitigation**: Use `git mv` to preserve history, not `mv` + `git add`

### Risk 3: External References
**Mitigation**: Search for any external docs/READMEs that might link to moved files, update accordingly

---

## Post-Organization Tasks

1. Update any CI/CD scripts that reference moved files
2. Update any GitHub wiki links if applicable
3. Consider creating a "documentation moved" notice for any external links
4. Archive this plan document to docs/dev-notes/ for historical record

---

## Approval Status

- [x] Plan Created
- [ ] Plan Reviewed
- [ ] Execution Approved
- [ ] Execution Started
- [ ] Execution Completed
- [ ] Verification Completed

---

**Created by**: Agent 9 (Technical Writer)
**Date**: 2026-03-01
**Ready for**: Execution
