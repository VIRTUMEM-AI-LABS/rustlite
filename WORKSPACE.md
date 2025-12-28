# RustLite Workspace Organization

Clean, professional workspace structure for the RustLite embedded database project.

## Root Directory Structure

```
rustlite/
├── LICENSE                    # Apache 2.0 license
├── README.md                  # Main project overview
├── Cargo.toml                 # Workspace manifest
├── Cargo.lock                 # Dependency lock file
│
├── docs/                      # 📚 All documentation
│   ├── README.md              # Documentation index
│   ├── ARCHITECTURE.md        # System design
│   ├── BRANDING.md            # Brand guidelines
│   ├── CHANGELOG.md           # Release notes
│   ├── CODE_OF_CONDUCT.md     # Community standards
│   ├── CONTRIBUTING.md        # Contribution guide
│   ├── GOVERNANCE.md          # Project governance
│   ├── ROADMAP.md             # Development roadmap
│   ├── v0.2_persistence_plan.md
│   ├── workspace_structure.md
│   ├── workspace_migration_summary.md
│   ├── guides/                # Implementation guides
│   └── marketing/             # Marketing materials
│
├── config/                    # ⚙️ Configuration files
│   ├── Cargo.toml.backup      # Original v0.1 Cargo.toml
│   └── README.md              # Config folder guide
│
├── crates/                    # 📦 Workspace member crates
│   ├── rustlite-api/          # Public API (published as 'rustlite')
│   ├── rustlite-core/         # Core types and in-memory storage
│   ├── rustlite-wal/          # Write-Ahead Log (v0.2+)
│   ├── rustlite-storage/      # Storage engine (v0.2+)
│   └── rustlite-snapshot/     # Snapshot manager (v0.2+)
│
├── assets/                    # 🎨 Branding and logos
│   ├── logo-icon.svg
│   ├── logo-wordmark.svg
│   └── exports/               # PNG exports
│
├── scripts/                   # 🔧 Utility scripts
│   ├── generate-assets.sh
│   ├── generate-assets.ps1
│   └── assets/
│
├── hooks/                     # 🪝 Git hooks
│   ├── pre-push
│   ├── pre-push.ps1
│   ├── enable.sh
│   └── install-hooks.ps1
│
├── .github/                   # GitHub workflows and templates
├── .gitignore                 # Git ignore rules
├── .gitattributes             # Git attributes
│
├── src/                       # Original v0.1 source (preserved)
└── target/                    # Build artifacts (generated)
```

## Key Sections

### 📚 Documentation (`docs/`)
- User guides and API documentation
- Architecture and design decisions
- Development roadmaps and plans
- Community guidelines

### ⚙️ Configuration (`config/`)
- Backup configuration files
- Build and environment configs

### 📦 Source Code (`crates/`)
- Multi-crate workspace structure
- Published crate: `rustlite-api` (published as `rustlite`)
- Internal crates for modular development

### 🎨 Branding (`assets/`)
- Logo files (SVG)
- PNG exports for web and social media

### 🔧 Utilities (`scripts/`, `hooks/`)
- Asset generation scripts
- Git pre-push hooks for quality checks

## Getting Started

1. **Read**: Start with [README.md](README.md) and [docs/README.md](docs/README.md)
2. **Build**: `cargo build --workspace`
3. **Test**: `cargo test --workspace`
4. **Contribute**: See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md)

## Building & Development

```bash
# Build entire workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Build documentation
cargo doc --no-deps --open

# Check code
cargo clippy --workspace -- -D warnings
```

## Important Files to Keep

- `LICENSE` — Apache 2.0 license (required)
- `README.md` — Project overview (required)
- `Cargo.toml` — Workspace manifest (required)
- `Cargo.lock` — Dependency lock (recommended)

## Clean Structure Philosophy

- ✅ **Root level**: Only LICENSE, README, Cargo files
- ✅ **docs/**: All documentation and guides
- ✅ **config/**: Configuration and backup files
- ✅ **crates/**: Source code (workspace members)
- ✅ **assets/**: Branding and media
- ✅ **scripts/**, **hooks/**: Utilities

---

**Last Updated**: October 27, 2025
