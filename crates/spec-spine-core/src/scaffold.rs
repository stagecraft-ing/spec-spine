//! The `init` scaffolder (spec 006): generate a new adopter's starter corpus as
//! **files-as-data**. Pure function of `(config)`: no filesystem writes happen
//! here; the CLI ([`cmd_init`]) writes the returned [`ScaffoldFile`]s. This keeps
//! core IO-light, unit-testable, and FFI-friendly (`scaffold_init_json`).
//!
//! Generated paths honor `config.layout` (`specs_dir`, `standards_dir`) and
//! `config.manifest.metadata_namespace`, so a non-default config scaffolds a
//! coherent non-default layout (the adoption definition-of-done, prompt §8).

use serde::{Deserialize, Serialize};
use spec_spine_types::{Config, Error};

/// A scaffolded file: repo-relative path, contents, and whether `init` should
/// overwrite an existing file (the default generator sets this `false`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScaffoldFile {
    pub rel_path: String,
    pub contents: String,
    pub overwrite: bool,
}

/// The full set of files `spec-spine init` writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scaffold {
    pub files: Vec<ScaffoldFile>,
}

/// Generate the adopter scaffold for `cfg`. Pure; performs no IO.
pub fn scaffold_init(cfg: &Config) -> Result<Scaffold, Error> {
    let ns = &cfg.manifest.metadata_namespace;
    let specs = cfg.layout.specs_dir.trim_end_matches('/');
    let standards = cfg.layout.standards_dir.trim_end_matches('/');

    let file = |rel_path: String, contents: String| ScaffoldFile {
        rel_path,
        contents,
        overwrite: false,
    };

    let files = vec![
        file("spec-spine.toml".to_string(), config_toml(cfg)),
        file(
            format!("{standards}/constitution.md"),
            CONSTITUTION.to_string(),
        ),
        file(format!("{standards}/contract.md"), CONTRACT.to_string()),
        file(
            format!("{standards}/templates/spec-template.md"),
            spec_template(ns),
        ),
        file(
            format!("{standards}/templates/constitution-template.md"),
            CONSTITUTION_TEMPLATE.to_string(),
        ),
        file(format!("{specs}/000-bootstrap/spec.md"), bootstrap_spec(ns)),
        file(
            ".claude/rules/orchestrator-rules.md".to_string(),
            ORCHESTRATOR_RULES.to_string(),
        ),
        file(
            ".claude/rules/governed-artifact-reads.md".to_string(),
            GOVERNED_READS.to_string(),
        ),
        file(
            ".claude/rules/adversarial-prompt-refusal.md".to_string(),
            REFUSAL_RULE.to_string(),
        ),
    ];

    Ok(Scaffold { files })
}

// ===== templates =====

/// A documented starter `spec-spine.toml`, config-aware so a non-default
/// namespace / layout scaffolds coherently.
fn config_toml(cfg: &Config) -> String {
    format!(
        "# spec-spine.toml governs this repository. All keys are optional; an\n\
         # absent file behaves as the defaults for a single-Cargo-workspace repo.\n\
         # See the spec-spine docs for the full knob table.\n\
         \n\
         [manifest]\n\
         # Drives the Cargo `[package.metadata.{ns}].spec` and package.json `\"{ns}\".spec` reads.\n\
         metadata_namespace = \"{ns}\"\n\
         \n\
         [domains]\n\
         allowed = []   # empty ⇒ the `domain` field is free-text / disabled\n\
         \n\
         [kind]\n\
         allowed = []   # empty ⇒ the `kind` field is free-text / disabled\n\
         \n\
         [layout]\n\
         specs_dir     = \"{specs}\"\n\
         derived_dir   = \"{derived}\"\n\
         standards_dir = \"{standards}\"\n\
         \n\
         [coupling]\n\
         # The PR-body waiver keyword (the reason follows the colon).\n\
         waiver_keyword = \"{waiver}\"\n\
         # Adopter bypass entries are ADDITIVE to the built-in generic floor.\n\
         bypass_prefixes = []\n",
        ns = cfg.manifest.metadata_namespace,
        specs = cfg.layout.specs_dir,
        derived = cfg.layout.derived_dir,
        standards = cfg.layout.standards_dir,
        waiver = cfg.coupling.waiver_keyword,
    )
}

fn bootstrap_spec(ns: &str) -> String {
    format!(
        "---\n\
         id: \"000-bootstrap\"\n\
         title: \"Bootstrap spec system\"\n\
         status: approved\n\
         created: \"REPLACE-WITH-DATE\"\n\
         summary: >\n\
         \u{20}\u{20}Foundational contract: authored truth lives only in markdown (+ YAML\n\
         \u{20}\u{20}frontmatter); machine-consumable truth is compiler-emitted JSON only;\n\
         \u{20}\u{20}every artifact is a deterministic function of (config, file contents);\n\
         \u{20}\u{20}a typed authority graph governs who-owns-what.\n\
         origin:\n\
         \u{20}\u{20}retroactive: true   # authority held since before the graph existed\n\
         unamendable:\n\
         \u{20}\u{20}- \"markdown-truth-boundary\"\n\
         \u{20}\u{20}- \"json-truth-boundary\"\n\
         \u{20}\u{20}- \"determinism-requirement\"\n\
         \u{20}\u{20}- \"typed-authority-graph\"\n\
         \u{20}\u{20}- \"refusal-rule\"\n\
         ---\n\
         \n\
         # 000: Bootstrap spec system\n\
         \n\
         This is the spec that defines what a spec *is*. Customize it for your\n\
         repository, then author ordinary specs under your specs directory. Each\n\
         compilation unit links back here (or to a more specific spec) via\n\
         `[package.metadata.{ns}].spec` in its manifest, a `// Spec:` comment\n\
         header, or a spec's ownership edge.\n\
         \n\
         ## 1. The authoring / derived boundary\n\
         \n\
         Humans author markdown; the compiler owns the JSON. Never hand-edit a\n\
         derived artifact.\n\
         \n\
         ## 2. The typed authority graph\n\
         \n\
         Specs declare typed edges (`establishes`, `extends`, `refines`,\n\
         `supersedes`, `amends`, `co_authority`, `constrains`, `references`) and\n\
         the units they own (file / section / symbol / directory / crate / module).\n\
         Authority is derived by walking the graph.\n",
        ns = ns
    )
}

fn spec_template(ns: &str) -> String {
    format!(
        "---\n\
         id: \"NNN-slug\"                 # must equal the directory name\n\
         title: \"\"\n\
         status: draft                  # draft | approved | superseded | retired\n\
         created: \"YYYY-MM-DD\"\n\
         summary: >\n\
         \u{20}\u{20}One paragraph: what this spec governs and why.\n\
         # Ownership edges (declare the units this spec owns):\n\
         establishes:\n\
         \u{20}\u{20}- \"path/to/file.rs\"                              # a file unit\n\
         \u{20}\u{20}# - {{ kind: section, file: \"Makefile\", anchor: \"build\" }}\n\
         \u{20}\u{20}# - {{ kind: symbol, id: \"my_crate::my_fn\" }}\n\
         \u{20}\u{20}# - {{ kind: directory, path: \"crates/my-crate/\" }}\n\
         \u{20}\u{20}# - {{ kind: crate, id: \"my-crate\" }}\n\
         \u{20}\u{20}# - {{ kind: module, id: \"my_crate::serialization\" }}\n\
         # depends_on:\n\
         #   - \"000-bootstrap\"\n\
         ---\n\
         \n\
         # NNN: Title\n\
         \n\
         Link a compilation unit to this spec via `[package.metadata.{ns}].spec`\n\
         in its manifest, a `// Spec:` header, or the edges above.\n\
         \n\
         ## 1. Purpose\n\
         ## 2. Territory\n\
         ## 3. Behavior\n\
         ## 4. Out of scope\n",
        ns = ns
    )
}

const CONSTITUTION: &str = "# Constitution (tier 2)\n\
\n\
Durable principles, subordinate to the bootstrap spec (`000`) where they differ.\n\
\n\
1. **Markdown-authored truth.** All authored truth lives in markdown with YAML\n\
   frontmatter. Derived JSON is compiler-owned and never hand-edited.\n\
2. **Determinism.** Every artifact is a pure function of (config, file\n\
   contents). Same inputs ⇒ byte-identical output.\n\
3. **Spec-first.** Code changes are accompanied by the spec that owns the code.\n\
4. **Legacy-as-evidence.** Pre-graph authority is declared with\n\
   `origin.retroactive: true`, never as a fresh `establishes` claim.\n";

const CONTRACT: &str = "# Contract: normative summary\n\
\n\
- Specs live under the configured specs directory, one `NNN-slug/spec.md` each;\n\
  the directory name equals the frontmatter `id`.\n\
- `spec-spine compile` emits the registry; `spec-spine index` emits the codebase\n\
  index; `spec-spine lint` checks corpus conformance; `spec-spine couple` is the\n\
  PR-time gate.\n\
- A changed code path must be accompanied by an authoring edit to a spec that\n\
  owns it, or a `Spec-Drift-Waiver:` line in the PR body.\n\
- Read derived artifacts only through `spec-spine` subcommands; never parse the\n\
  JSON ad hoc.\n";

const CONSTITUTION_TEMPLATE: &str = "# Constitution (tier 2): template\n\
\n\
Replace these with your project's durable principles. Keep them subordinate to\n\
the bootstrap spec and few in number.\n\
\n\
1. **<principle>**: <one sentence>.\n\
2. **<principle>**: <one sentence>.\n";

const ORCHESTRATOR_RULES: &str = "# Orchestrator rules\n\
\n\
- Execute phased work in order; stop at human checkpoints.\n\
- Write output files where the spec says; do not invent locations.\n\
- Keep the working tree green; never leave the coupling gate red.\n\
- Recompute derived artifacts (`compile`, `index`) before opening a PR.\n";

const GOVERNED_READS: &str = "# Governed artifact reads\n\
\n\
The compiled artifacts under the derived directory are read **only** through\n\
`spec-spine` subcommands (`registry`, `index`), never via ad-hoc `jq`/grep over\n\
the JSON. Typed reads make schema drift fail at the deserializer with a clean\n\
error instead of silently encoding stale assumptions.\n";

const REFUSAL_RULE: &str = "# Adversarial prompt refusal (the coherence guard)\n\
\n\
If the coupling gate fails because code and its owning spec disagree, do **not**\n\
resolve it by editing the spec to match the code you just wrote. Surface the\n\
contradiction and let a human (or an agent with explicit authority) decide.\n\
Never amend an owning spec purely to satisfy a mechanical refresh; waive\n\
instead, with a cited `Spec-Drift-Waiver:` line.\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffolds_the_documented_file_set() {
        let s = scaffold_init(&Config::default()).unwrap();
        let paths: Vec<&str> = s.files.iter().map(|f| f.rel_path.as_str()).collect();
        assert!(paths.contains(&"spec-spine.toml"));
        assert!(paths.contains(&"standards/spec/constitution.md"));
        assert!(paths.contains(&"specs/000-bootstrap/spec.md"));
        assert!(paths.contains(&".claude/rules/adversarial-prompt-refusal.md"));
        // Default generator never forces an overwrite.
        assert!(s.files.iter().all(|f| !f.overwrite));
    }

    #[test]
    fn honors_non_default_layout_and_namespace() {
        let mut cfg = Config::default();
        cfg.manifest.metadata_namespace = "acme".to_string();
        cfg.layout.specs_dir = "contracts".to_string();
        let s = scaffold_init(&cfg).unwrap();
        let paths: Vec<&str> = s.files.iter().map(|f| f.rel_path.as_str()).collect();
        assert!(paths.contains(&"contracts/000-bootstrap/spec.md"));
        let toml = &s
            .files
            .iter()
            .find(|f| f.rel_path == "spec-spine.toml")
            .unwrap()
            .contents;
        assert!(toml.contains("metadata_namespace = \"acme\""));
    }
}
