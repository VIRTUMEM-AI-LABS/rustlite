# RustLite Governance

This document describes the governance model for the RustLite project.

## Project Status

**Current Phase**: Early Development (Pre-1.0)

RustLite is currently in its initial development phase. The governance model will evolve as the project matures and the community grows.

## Project Goals

1. **Build a production-ready embedded database** that rivals SQLite in reliability
2. **Foster an open, welcoming community** of contributors
3. **Maintain high code quality** and security standards
4. **Make data storage safe and accessible** for Rust developers

## Roles and Responsibilities

### Founder/Maintainer

**Current**: Srikanth Aakuthota

**Responsibilities:**
- Overall project vision and direction
- Final decision on controversial changes
- Release management
- Security response coordination
- Trademark and legal matters
- Community health and moderation

### Core Contributors (Future)

Individuals who have made significant, sustained contributions to the project.

**How to become a core contributor:**
- Consistent high-quality contributions over 3+ months
- Demonstrated understanding of project architecture
- Positive collaboration with community
- Nomination by existing maintainer(s)

**Privileges:**
- Write access to repository
- Participate in architectural decisions
- Review and merge pull requests
- Triage issues

### Contributors

Anyone who contributes to the project through:
- Code contributions (features, bug fixes)
- Documentation improvements
- Bug reports and issue triage
- Code reviews
- Community support

**All contributors are valued** and recognized in our CONTRIBUTORS.md file (to be created).

## Decision-Making Process

### Current Phase (Pre-1.0)

**Benevolent Dictator Model**
- Maintainer makes final decisions on features, architecture, and releases
- Community input strongly encouraged through issues and discussions
- Major changes discussed publicly before implementation
- Consensus sought when possible

### Future Phase (Post-1.0)

**Lazy Consensus Model**
- Proposals made through GitHub issues/discussions
- Minimum 72-hour discussion period for major changes
- Objections must be substantiated with technical reasoning
- Core contributors can veto with explanation
- Maintainer breaks deadlocks

## Types of Changes

### Trivial Changes
- Documentation typos
- Code formatting
- Minor bug fixes
- Test improvements

**Process:**
- Submit PR directly
- Any contributor can review
- One approval sufficient

### Standard Changes
- New features (aligned with roadmap)
- Bug fixes requiring design decisions
- Performance improvements
- Dependency updates

**Process:**
- Open issue for discussion (preferred)
- Submit PR with description
- Core contributor review required
- Automated tests must pass

### Significant Changes
- Breaking API changes
- Major architectural changes
- New dependencies with significant impact
- Changes to project governance

**Process:**
- RFC (Request for Comments) in GitHub discussions
- Minimum 1-week discussion period
- Maintainer approval required
- May require design document

## Contribution License Agreement (CLA)

**Model**: Developer Certificate of Origin (DCO)

Contributors certify that their contributions are their own work or properly licensed by adding:

```
Signed-off-by: Your Name <your.email@example.com>
```

To commits (use `git commit -s`).

**No CLA required.** By submitting a PR, you agree to license your contribution under Apache-2.0.

## Code Review Standards

All code must be reviewed before merging.

**Review Criteria:**
- Code quality and style (cargo fmt, clippy)
- Test coverage (new features must have tests)
- Documentation (public APIs must be documented)
- Performance impact (benchmarks for critical paths)
- Security implications
- Backward compatibility (post-1.0)

**Review Process:**
1. Automated checks (CI)
2. Code review by contributor(s)
3. Approval by core contributor or maintainer
4. Merge via "Squash and Merge" or "Rebase and Merge"

## Release Process

### Version Scheme

**Semantic Versioning** (post-1.0):
- MAJOR.MINOR.PATCH (e.g., 1.2.3)
- MAJOR: Breaking changes
- MINOR: New features, backward compatible
- PATCH: Bug fixes, backward compatible

**Pre-1.0**:
- 0.MINOR.PATCH
- MINOR may include breaking changes

### Release Cadence

**Current Target**: Quarterly releases
- v0.1: Q4 2025 ✅
- v0.2: Q1 2026
- v0.3: Q2 2026
- v0.4: Q3 2026

**Patch Releases**: As needed for critical bugs/security

### Release Checklist

- [ ] All tests passing
- [ ] Version bumped in Cargo.toml
- [ ] CHANGELOG.md updated
- [ ] Documentation updated
- [ ] Benchmarks run (performance regression check)
- [ ] Git tag created (vX.Y.Z)
- [ ] Published to crates.io
- [ ] GitHub release created
- [ ] Announcement (blog, social media)

## Communication Channels

### GitHub Issues
**Purpose**: Bug reports, feature requests, task tracking

**Guidelines:**
- Use issue templates
- Search before creating
- Stay on topic
- Be respectful

### GitHub Discussions
**Purpose**: Questions, ideas, RFCs, announcements

**Categories:**
- General: Questions and chat
- Ideas: Feature proposals
- Show and Tell: Projects using RustLite
- RFCs: Significant change proposals

### Discord (Planned)
**Purpose**: Real-time chat, community building

**Channels** (planned):
- #general
- #development
- #help
- #announcements

### Email
- **General**: dev@rustlite.dev (planned)
- **Security**: security@rustlite.dev (planned)
- **Code of Conduct**: conduct@rustlite.dev (planned)

## Security Policy

### Reporting Vulnerabilities

**DO NOT** report security vulnerabilities through public GitHub issues.

**Instead**:
1. Email security@rustlite.dev (planned) or
2. Use GitHub Security Advisories (private reporting)

**What to include:**
- Description of vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

**Response Timeline:**
- Acknowledgment: 48 hours
- Initial assessment: 1 week
- Fix timeline: Varies by severity

### Security Fixes

- High severity: Patch release ASAP
- Medium severity: Next scheduled release
- Low severity: Queued for future release

**Disclosure:**
- Coordinated disclosure after fix available
- Credit to reporter (unless anonymous)
- Security advisory published

## Code of Conduct

RustLite follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

**Enforcement:**
- Minor violations: Private warning
- Repeated violations: Temporary ban
- Severe violations: Permanent ban

**Reporting**: conduct@rustlite.dev (planned)

## Intellectual Property

### Copyright

Copyright held by individual contributors. Each file includes:

```rust
// Copyright (c) 2025 RustLite Contributors
// SPDX-License-Identifier: Apache-2.0
```

### License

Licensed under **Apache-2.0** to provide patent protection and corporate-friendly terms for the Rust ecosystem.

### Trademark (Future)

"RustLite" name and logo (when created) will be trademarked to protect the project identity.

**Permitted Use:**
- Referring to this project
- Stating compatibility ("Works with RustLite")
- Educational materials

**Requires Permission:**
- Modified/forked versions claiming to be "RustLite"
- Commercial products named "RustLite [something]"

## Finances (Future)

Currently unfunded. Future funding options:

- GitHub Sponsors
- Open Collective
- Corporate sponsorship
- Grants (e.g., Rust Foundation)

**Use of Funds:**
- Infrastructure (CI, hosting)
- Marketing (swag, conferences)
- Security audits
- Contributor recognition

**Transparency:**
- Public budget
- Quarterly financial reports

## Amendment Process

This governance document can be amended by:

1. Proposal via GitHub Discussion (RFC)
2. Minimum 2-week discussion period
3. Maintainer approval
4. PR to update this document
5. Announcement of changes

## Credits and Inspiration

This governance model is inspired by:
- [Rust Project Governance](https://github.com/rust-lang/rfcs/blob/master/text/1068-rust-governance.md)
- [Node.js Governance](https://github.com/nodejs/node/blob/main/GOVERNANCE.md)
- [Apache Software Foundation](https://www.apache.org/foundation/governance/)

---

**Last Updated**: October 25, 2025

Questions about governance? Open a discussion on GitHub or contact the maintainer.
