//! The `spec-spine.toml` configuration model.
//!
//! Everything the reference repos had to fork over is a knob here. An absent
//! config yields a working default for a single-Cargo-workspace repo with
//! `specs/` at the root ([`Config::default`]). Every struct is
//! `#[serde(default, deny_unknown_fields)]`: missing keys default, and a
//! *misspelled* knob is a loud [`Error::Config`] rather than a silently-ignored
//! setting — the exact failure class that left template-encore blind to its npm
//! packages. See `docs/design/00-architecture.md` §3.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The full configuration. All sections are optional. `Default` is derived —
/// each field's own `Default` supplies the conventional value.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub manifest: ManifestConfig,
    /// Opt-in `domain` taxonomy (empty `allowed` ⇒ free-text/disabled).
    pub domains: AllowlistConfig,
    /// Opt-in `kind` taxonomy — symmetric with `domains` (empty ⇒ disabled).
    pub kind: AllowlistConfig,
    pub layout: LayoutConfig,
    pub index: IndexConfig,
    pub branding: BrandingConfig,
    pub coupling: CouplingConfig,
    pub provenance: ProvenanceConfig,
    pub frontmatter: FrontmatterConfig,
}

/// `[manifest]` — how a manifest links a compilation unit back to its spec.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManifestConfig {
    /// Drives both `[package.metadata.<ns>].spec` (Cargo) and `"<ns>".spec`
    /// (package.json). OAP used `oap`; aide/encore used `spec`.
    pub metadata_namespace: String,
}

impl Default for ManifestConfig {
    fn default() -> Self {
        ManifestConfig {
            metadata_namespace: "spec-spine".to_string(),
        }
    }
}

/// A reusable opt-in categorical allowlist (used by `[domains]` and `[kind]`).
///
/// Empty ⇒ the field is free-text / disabled (no enum check). Non-empty ⇒ a
/// closed enum: the field value, *when present*, must be a member (a `V`-error
/// otherwise). Field absence is allowed.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AllowlistConfig {
    pub allowed: Vec<String>,
}

impl AllowlistConfig {
    /// True if this taxonomy is disabled (no allowlist configured).
    pub fn is_disabled(&self) -> bool {
        self.allowed.is_empty()
    }

    /// True if `value` is permitted: always when disabled, else membership.
    pub fn permits(&self, value: &str) -> bool {
        self.is_disabled() || self.allowed.iter().any(|a| a == value)
    }
}

/// `[layout]` — path conventions. Never hardcode `specs/`, `.derived/`, etc.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayoutConfig {
    pub specs_dir: String,
    pub derived_dir: String,
    pub standards_dir: String,
    pub schemas_dir: String,
    /// Root Cargo workspace manifest (relative to repo root).
    pub cargo_workspace: String,
    /// Manifests that DECLARE npm/pnpm workspace members. The indexer reads
    /// member globs from whichever exists. The default reads root
    /// `package.json#workspaces` — fixing the template-encore bug where a
    /// hardcoded `public/pnpm-workspace.yaml` made all npm packages invisible.
    pub npm_workspaces: Vec<String>,
    /// Crates outside the root Cargo workspace.
    pub standalone_rust_workspaces: Vec<String>,
    /// npm packages outside the declared workspaces.
    pub standalone_npm_packages: Vec<String>,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        LayoutConfig {
            specs_dir: "specs".to_string(),
            derived_dir: ".derived".to_string(),
            standards_dir: "standards/spec".to_string(),
            schemas_dir: "standards/schemas".to_string(),
            cargo_workspace: "Cargo.toml".to_string(),
            npm_workspaces: vec![
                "package.json".to_string(),
                "pnpm-workspace.yaml".to_string(),
            ],
            standalone_rust_workspaces: Vec::new(),
            standalone_npm_packages: Vec::new(),
        }
    }
}

/// `[index]` — inputs and exclusions for the codebase indexer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IndexConfig {
    /// Globs folded into the content-hash beyond the always-hashed core
    /// (all spec.md + discovered manifests + `spec-spine.toml`).
    pub extra_hashed_inputs: Vec<String>,
    /// Directory names pruned from symbol/section resolution walks.
    pub resolver_exclusions: Vec<String>,
}

impl Default for IndexConfig {
    fn default() -> Self {
        IndexConfig {
            extra_hashed_inputs: vec![
                "standards/**".to_string(),
                ".github/workflows/**".to_string(),
            ],
            resolver_exclusions: vec![
                "target".to_string(),
                "node_modules".to_string(),
                ".derived".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".next".to_string(),
            ],
        }
    }
}

/// `[branding]` — identifiers stamped into emitted `build` metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BrandingConfig {
    pub compiler_id: String,
    pub indexer_id: String,
}

impl Default for BrandingConfig {
    fn default() -> Self {
        BrandingConfig {
            compiler_id: "spec-spine".to_string(),
            indexer_id: "spec-spine".to_string(),
        }
    }
}

/// `[coupling]` — the PR-time gate's exemptions and waiver keyword.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CouplingConfig {
    /// **Additional** paths exempt from the gate, on top of the always-applied
    /// hardcoded floor (`spec_spine_core::DEFAULT_BYPASS_PREFIXES`). Match rules:
    /// trailing `/` ⇒ dir prefix; leading `**/` ⇒ tail-suffix anywhere; else
    /// exact file. This list is **additive** — it adds to the floor and can never
    /// remove a floor entry. The default is **empty**: the floor is the single
    /// built-in source, so an adopter declares only their own additions rather
    /// than restating (and seeming able to override) the floor.
    pub bypass_prefixes: Vec<String>,
    /// The PR-body waiver keyword; the free-text reason follows the colon.
    pub waiver_keyword: String,
    /// Opt-in mechanical auto-waiver for dependency-only diffs (spec 005
    /// §3.5). When `true` and no PR-body waiver is present, the CLI compares
    /// the parsed base/head JSON of every non-bypassed changed path: if all
    /// are `package.json` manifests whose only differences are version
    /// strings inside the standard dependency tables (same package keys),
    /// the gate self-waives — the path dependabot-class PRs cannot take
    /// (they can edit neither specs nor PR bodies). Anything beyond a
    /// version string — a new package, a `scripts` edit, spec-binding
    /// metadata — refuses the auto-waiver, fail-closed. Default `false`.
    pub auto_waive_dependency_only: bool,
}

impl Default for CouplingConfig {
    fn default() -> Self {
        CouplingConfig {
            // Empty by design — the floor lives in `couple.rs` and is always
            // unioned in; duplicating it here was redundant and misleadingly
            // implied it was overridable.
            bypass_prefixes: Vec::new(),
            waiver_keyword: "Spec-Drift-Waiver:".to_string(),
            auto_waive_dependency_only: false,
        }
    }
}

/// `[provenance]` — the OPEN provenance-scheme registry (kind → URI scheme).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProvenanceConfig {
    pub uri_schemes: BTreeMap<String, String>,
}

impl Default for ProvenanceConfig {
    fn default() -> Self {
        let mut uri_schemes = BTreeMap::new();
        uri_schemes.insert("knowledge".to_string(), "knowledge://".to_string());
        uri_schemes.insert("code-fingerprint".to_string(), "fingerprint://".to_string());
        ProvenanceConfig { uri_schemes }
    }
}

/// `[frontmatter]` — recognized-key extensions.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FrontmatterConfig {
    /// Keys an adopter recognizes (suppresses the lint's unknown-key warning);
    /// they still overflow into `extra_frontmatter`.
    pub extra_known_keys: Vec<String>,
}

/// Load and validate a `spec-spine.toml` from its source text.
///
/// Returns [`Error::Config`] (mapped to exit code 3) on any malformed or
/// unknown-key error — never panics.
pub fn load_config(toml_src: &str) -> Result<Config> {
    toml::from_str(toml_src).map_err(|e| Error::Config(e.to_string()))
}
