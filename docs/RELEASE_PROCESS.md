# Release Process & Formalization

> **Prepared by**: Git Specialist + CI/CD Specialist + GitHub Actions Specialist
>
> **Date**: 2026-02-27
>
> **Status**: Active

---

## Table of Contents

1. [Versioning Strategy](#versioning-strategy)
2. [Release Types](#release-types)
3. [Release Checklist](#release-checklist)
4. [Automated Release Process](#automated-release-process)
5. [Changelog Management](#changelog-management)
6. [Hotfix Process](#hotfix-process)
7. [Communication Plan](#communication-plan)

---

## Versioning Strategy

**Nineties follows [Semantic Versioning 2.0.0](https://semver.org/)**

### Version Format: `MAJOR.MINOR.PATCH`

```
v0.2.2
│ │ │
│ │ └─ PATCH: Bug fixes, security patches (backwards compatible)
│ └─── MINOR: New features (backwards compatible)
└───── MAJOR: Breaking changes (not backwards compatible)
```

### Version Incrementation Rules

| Change Type | Version | Example | When to Use |
|-------------|---------|---------|-------------|
| **Breaking Change** | MAJOR | 0.2.2 → 1.0.0 | API changes, removed features, incompatible updates |
| **New Feature** | MINOR | 0.2.2 → 0.3.0 | New functionality, enhancements |
| **Bug Fix** | PATCH | 0.2.2 → 0.2.3 | Bug fixes, security patches, documentation |
| **Pre-release** | SUFFIX | 0.3.0-alpha.1 | Alpha, beta, rc versions |

### Pre-1.0.0 Versioning

**Current Status**: v0.2.2 (pre-stable)

During pre-1.0.0 development:
- MINOR version bumps may include breaking changes
- PATCH version bumps are for fixes only
- **Event Sourcing migration will trigger 1.0.0 release**

### Version Milestones

| Version | Milestone | Target Date | Key Features |
|---------|-----------|-------------|--------------|
| v0.2.x | Current | - | MVC starter, JWT auth, WebSockets |
| v0.3.0 | Security & Quality | Q1 2026 | Rate limiting, validation, CI/CD |
| v0.4.0 | Plugin System | Q2 2026 | Plugin architecture, hooks |
| v0.5.0 | Event Sourcing MVP | Q2 2026 | Event store, basic ES patterns |
| v1.0.0 | Event Sourcing Complete | Q3 2026 | Full ES architecture, production-ready |
| v1.1.0 | PWA Support | Q3 2026 | Offline capability, installable |
| v2.0.0 | Distributed Architecture | Q4 2026+ | Clustering, multi-node |

---

## Release Types

### 1. Regular Release (Feature Release)

**Cadence**: Every 2-4 weeks

**Includes**:
- New features
- Enhancements
- Non-critical bug fixes
- Documentation updates

**Process**:
1. Create release branch: `release/v0.x.0`
2. Run full test suite
3. Update CHANGELOG.md
4. Bump version in Cargo.toml
5. Create release tag
6. Merge to main
7. GitHub Actions handles the rest

### 2. Patch Release (Bug Fix)

**Cadence**: As needed

**Includes**:
- Critical bug fixes
- Security patches
- Performance fixes

**Process**:
1. Fix on `hotfix/issue-description` branch
2. Fast-track testing
3. Bump PATCH version
4. Tag and release
5. Backport to release branches if needed

### 3. Major Release (Breaking Changes)

**Cadence**: Rare, planned in advance

**Includes**:
- Breaking API changes
- Architecture overhaul (e.g., Event Sourcing migration)
- Major feature additions

**Process**:
1. Announce breaking changes 1 month in advance
2. Create migration guide
3. Beta/RC releases for testing
4. Extended QA period
5. Comprehensive release notes
6. Version bump and release

### 4. Pre-release (Alpha/Beta/RC)

**Cadence**: As needed during major development

**Format**:
- Alpha: `v0.5.0-alpha.1` (unstable, frequent changes)
- Beta: `v0.5.0-beta.1` (feature-complete, testing)
- RC: `v0.5.0-rc.1` (release candidate, final testing)

**Purpose**:
- Early feedback
- Testing in production-like environments
- Community involvement

---

## Release Checklist

### Pre-Release (1 week before)

- [ ] **Code Freeze**: No new features after this point
- [ ] **Feature Complete**: All planned features merged
- [ ] **Documentation Updated**:
  - [ ] README.md
  - [ ] CHANGELOG.md
  - [ ] API docs
  - [ ] Migration guides (if breaking changes)
- [ ] **Version Bump**:
  - [ ] Update `Cargo.toml` version
  - [ ] Update `package.json` version (if changed)
- [ ] **Testing**:
  - [ ] All tests passing (unit, integration, e2e)
  - [ ] Manual testing of new features
  - [ ] Performance benchmarks
  - [ ] Security scan passed
- [ ] **Dependencies**:
  - [ ] Update dependencies to latest stable
  - [ ] Run `cargo audit`
  - [ ] Run `npm audit`

### Release Day

- [ ] **Final Checks**:
  - [ ] CI pipeline green
  - [ ] No critical bugs
  - [ ] Documentation reviewed
- [ ] **Create Release Branch**:
  ```bash
  git checkout -b release/v0.x.0
  git push origin release/v0.x.0
  ```
- [ ] **Tag Release**:
  ```bash
  git tag -a v0.x.0 -m "Release v0.x.0: Brief description"
  git push origin v0.x.0
  ```
- [ ] **Monitor GitHub Actions**:
  - [ ] CI pipeline completes
  - [ ] Release artifacts built
  - [ ] Docker images pushed
  - [ ] Documentation published
- [ ] **Verify Release**:
  - [ ] GitHub release created with changelog
  - [ ] Binaries downloadable
  - [ ] Docker image pullable
  - [ ] Documentation live

### Post-Release (within 24 hours)

- [ ] **Merge to Main**:
  ```bash
  git checkout main
  git merge release/v0.x.0
  git push origin main
  ```
- [ ] **Announcement**:
  - [ ] GitHub Discussions post
  - [ ] Twitter/Social media
  - [ ] Discord/Slack community
  - [ ] Email newsletter (if applicable)
- [ ] **Monitor**:
  - [ ] Watch for bug reports
  - [ ] Monitor error tracking (if enabled)
  - [ ] Community feedback
- [ ] **Retrospective**:
  - [ ] What went well?
  - [ ] What could improve?
  - [ ] Update release process if needed

---

## Automated Release Process

### GitHub Actions Workflow

**Trigger**: Push tag matching `v*.*.*`

```bash
# Example: Trigger release
git tag -a v0.3.0 -m "Release v0.3.0: Add rate limiting and input validation"
git push origin v0.3.0
```

**Automated Steps**:

1. **CI Pipeline** (`ci.yml`):
   - Code quality checks (rustfmt, clippy)
   - Build on Linux, macOS, Windows
   - Run all tests
   - Generate code coverage
   - Build frontend assets
   - Build documentation

2. **Release Pipeline** (`release.yml`):
   - Create GitHub Release
   - Generate changelog from commits
   - Build binaries for all platforms:
     - Linux (x86_64)
     - macOS (x86_64, aarch64)
     - Windows (x86_64)
   - Publish to crates.io (optional)
   - Build and push Docker image
   - Publish documentation to GitHub Pages

3. **Security Pipeline** (`security.yml`):
   - Dependency audit
   - Secrets scan
   - License compliance
   - SAST analysis

### Manual Release (Fallback)

If GitHub Actions fail or for manual releases:

```bash
# 1. Build release binaries
cargo build --release

# 2. Create release archive
tar czf nineties-v0.3.0-linux-x86_64.tar.gz -C target/release nineties

# 3. Create GitHub release manually
gh release create v0.3.0 \
  --title "Release v0.3.0" \
  --notes-file CHANGELOG.md \
  nineties-v0.3.0-linux-x86_64.tar.gz

# 4. Publish to crates.io
cargo publish
```

---

## Changelog Management

### Commit Message Convention

**Format**: `<type>(<scope>): <subject>`

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `perf`: Performance improvement
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `test`: Adding/updating tests
- `chore`: Maintenance tasks
- `ci`: CI/CD changes
- `security`: Security fixes

**Examples**:
```
feat(auth): add rate limiting to login endpoint
fix(websocket): resolve connection timeout issue
perf(template): cache Tera engine for faster rendering
docs(es): add event sourcing implementation guide
security(validation): prevent SQL injection in user queries
```

### CHANGELOG.md Structure

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Feature X that does Y

### Changed
- Updated component Z

### Fixed
- Bug in module A

### Security
- Patched vulnerability in dependency B

## [0.3.0] - 2026-03-15

### Added
- Rate limiting middleware for authentication endpoints
- Input validation with validator crate
- Pre-commit hooks for code quality

### Changed
- Replaced println! with tracing for structured logging
- Updated documentation with Docsify

### Fixed
- Connection pool recreation issue
- Unused variable warnings

### Security
- Added CSRF token validation
- Removed password logging from error messages

## [0.2.2] - 2026-02-20

...
```

### Automated Changelog Generation

**Using GitHub Actions** (in release workflow):
- Parses commit messages since last tag
- Categorizes by type (feat, fix, etc.)
- Generates formatted release notes
- Attaches to GitHub Release

**Tool**: Built into `.github/workflows/release.yml`

---

## Hotfix Process

### When to Hotfix

**Criteria** (any of):
- Critical security vulnerability (CVSS ≥ 7.0)
- Data loss or corruption bug
- Service outage or crash
- Major performance degradation

### Hotfix Workflow

```bash
# 1. Create hotfix branch from latest release tag
git checkout -b hotfix/v0.2.3 v0.2.2

# 2. Make the fix
# ... code changes ...

# 3. Test thoroughly
cargo test
cargo clippy

# 4. Update changelog
echo "## [0.2.3] - $(date +%Y-%m-%d)" >> CHANGELOG.md
echo "### Fixed" >> CHANGELOG.md
echo "- Critical bug X" >> CHANGELOG.md

# 5. Bump version
sed -i 's/version = "0.2.2"/version = "0.2.3"/' Cargo.toml

# 6. Commit and tag
git commit -am "fix: critical bug in authentication (hotfix v0.2.3)"
git tag -a v0.2.3 -m "Hotfix v0.2.3: Critical authentication bug"

# 7. Push
git push origin hotfix/v0.2.3
git push origin v0.2.3

# 8. Merge back to main and develop
git checkout main
git merge hotfix/v0.2.3
git push origin main
```

### Hotfix Communication

**Immediate**:
- GitHub Security Advisory (if security issue)
- Email to users (if critical)
- Pin GitHub issue with details

**Within 1 hour**:
- Update status page
- Social media announcement
- Community channels

**Within 24 hours**:
- Postmortem report (if major issue)
- Prevention plan

---

## Communication Plan

### Release Announcement Template

```markdown
# 🎉 Nineties v0.3.0 Released!

We're excited to announce the release of Nineties v0.3.0!

## ✨ Highlights

- 🔒 **Rate Limiting**: Protect your auth endpoints from brute-force attacks
- ✅ **Input Validation**: Comprehensive validation with the validator crate
- 🔍 **Better Logging**: Structured logging with tracing
- 📚 **Improved Docs**: Visual documentation with Docsify

## 📦 Installation

**Cargo**:
```bash
cargo install nineties
```

**Docker**:
```bash
docker pull username/nineties:0.3.0
```

**Binary**:
Download from [GitHub Releases](https://github.com/user/nineties/releases/tag/v0.3.0)

## 📝 Changelog

Full changelog: [CHANGELOG.md](CHANGELOG.md)

## 🙏 Contributors

Thank you to all contributors who made this release possible!

## 🐛 Reporting Issues

Found a bug? [Open an issue](https://github.com/user/nineties/issues/new)

---

Happy coding! 🚀
```

### Channels

| Channel | Audience | Timing |
|---------|----------|--------|
| GitHub Releases | Developers | On release |
| GitHub Discussions | Community | Within 1 hour |
| Discord/Slack | Active users | Within 1 hour |
| Twitter/Social | General public | Within 2 hours |
| Email Newsletter | Subscribers | Within 24 hours |
| Blog Post | Detailed announcement | Within 1 week |

---

## Release Metrics

**Track for each release**:
- Time to release (from code freeze to published)
- Number of bugs reported (first week)
- Download count (first week)
- Docker pulls
- Community feedback score
- Test coverage percentage
- Performance benchmarks

**Review quarterly**:
- Release cadence
- Hotfix frequency
- Time to fix critical bugs
- Community satisfaction

---

## Release Schedule (Proposed)

```
2026:
  Q1:
    - v0.3.0 (Security & Quality) - March 15
    - v0.3.1 (Bug fixes) - As needed

  Q2:
    - v0.4.0 (Plugin System) - May 1
    - v0.5.0 (ES MVP) - June 15

  Q3:
    - v0.5.x (ES iterations) - July-August
    - v1.0.0 (ES Complete) - September 1

  Q4:
    - v1.1.0 (PWA) - October 15
    - v1.2.0 (Polish) - December 1
```

---

## Summary

**Nineties Release Process**:
- ✅ Semantic Versioning 2.0.0
- ✅ Automated via GitHub Actions
- ✅ Comprehensive testing required
- ✅ Security-first approach
- ✅ Clear communication plan
- ✅ Hotfix procedures defined

**Key Principles**:
1. **Quality over speed**: Don't rush releases
2. **Communicate early**: Keep users informed
3. **Automate everything**: Reduce human error
4. **Track metrics**: Continuous improvement
5. **Security first**: Never compromise on security

---

## Resources

- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [GitHub Releases Guide](https://docs.github.com/en/repositories/releasing-projects-on-github)
- [Rust Release Best Practices](https://doc.rust-lang.org/cargo/reference/publishing.html)

---

**Next Review**: Q2 2026 (after v0.5.0 release)
