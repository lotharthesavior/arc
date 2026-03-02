# Architecture Diagram Update Summary

## Changes Made

### 1. Created New Mermaid Diagram

**File:** `docs/diagrams/source/architecture-20-mvc-request-flow.mmd`

**Description:** Professional mermaid diagram replacing the ASCII art in the architecture documentation

**Features:**
- Shows complete MVC request flow
- Layered architecture visualization
- Color-coded components (Primary: Actix/Routes, Secondary: Controllers/Services, Tertiary: Models/Diesel, Database: SQLite)
- Middleware layer with SessionMiddleware, NormalizePath, and AuthMiddleware
- Route definitions (/, /signin, /signout, /admin/*)
- Controllers, Services, Helpers, Models layers
- Database integration with Diesel ORM

### 2. Generated SVG File

**File:** `docs/diagrams/architecture-20-mvc-request-flow.svg`

**Size:** 32KB

**Generated using:**
```bash
mmdc -i source/architecture-20-mvc-request-flow.mmd \
     -o architecture-20-mvc-request-flow.svg \
     -t default \
     -b transparent
```

### 3. Updated Architecture Documentation

**File:** `docs/02-architecture.md`

**Change:** Replaced ASCII art diagram (lines 9-73) with standard image reference:

```markdown
![Architecture Diagram - MVC Request Flow - Shows HTTP request flowing through Actix Web server, middleware layer, routes, controllers, services and helpers, models, Diesel ORM, to SQLite database](diagrams/architecture-20-mvc-request-flow.svg)
```

**Benefits:**
- Professional, scalable SVG rendering
- Consistent with other documentation diagrams
- Better visual clarity and readability
- Mobile-friendly and high-DPI display support

### 4. Created Documentation Guidelines

**File:** `docs/CLAUDE.md`

**Purpose:** Comprehensive guide for AI agents working with documentation

**Contents:**
- **Critical Rule:** No inline mermaid diagrams
- Complete workflow for creating diagrams (6 steps)
- Naming conventions and categories
- Alt text guidelines
- Quality checklist
- Troubleshooting guide
- Examples from this repository
- Best practices summary

**Ensures:**
- Future documentation follows the same pattern
- Consistent diagram management across the project
- Easy onboarding for contributors
- Professional documentation standards

## Diagram Naming Convention Followed

**Pattern:** `{category}-{number}-{description}`

**This Diagram:**
- Category: `architecture` (system design/component relationships)
- Number: `20` (next sequential number after architecture-19)
- Description: `mvc-request-flow` (clear, descriptive, kebab-case)

## Next Available Numbers

For future diagrams:
- `architecture-21` (next architecture diagram)
- `flow-16` (next flow diagram)
- `state-05` (next state diagram)
- Other categories as needed

## Files Changed

1. ✅ `docs/diagrams/source/architecture-20-mvc-request-flow.mmd` - Created
2. ✅ `docs/diagrams/architecture-20-mvc-request-flow.svg` - Generated
3. ✅ `docs/02-architecture.md` - Updated (removed ASCII art, added image reference)
4. ✅ `docs/CLAUDE.md` - Created (documentation guidelines)

## Verification

- [x] SVG file successfully generated (32KB)
- [x] Diagram follows mermaid best practices
- [x] Alt text is descriptive and follows pattern
- [x] Image path is relative and correct
- [x] Documentation renders correctly
- [x] Guidelines documented in CLAUDE.md
- [x] Naming convention followed
- [x] Color palette consistent with existing diagrams

## Future Guidelines

All AI agents and contributors should:
1. Read `docs/CLAUDE.md` before adding diagrams
2. Never add inline mermaid code blocks to documentation
3. Always create source `.mmd` files in `docs/diagrams/source/`
4. Always generate `.svg` files before committing
5. Follow the established naming convention
6. Write descriptive alt text for accessibility
7. Test diagrams at mermaid.live before committing

---

**Date:** 2026-03-01  
**Updated by:** Claude Code (Agent Orchestration)
