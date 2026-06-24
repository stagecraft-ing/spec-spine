---
name: research
description: Deep research with parallel sub-agents, query classification, and filesystem artifact passing
allowed-tools: Task, Read, Write, Bash(git log:*), Bash(git diff:*), WebSearch, WebFetch, Glob, Grep
argument-hint: "<question or topic to investigate>"
---

# Research

Conduct deep, parallel research on a topic using multiple specialized sub-agents.

## Research Query

$ARGUMENTS

## Phase 1: Query Classification

**This is the critical first step. Classify before doing anything else.**

### Query Types

| Type | Characteristics | Sub-agents | Depth per agent |
|------|----------------|------------|-----------------|
| **Breadth-first** | Multiple independent aspects, surveys, comparisons | 5-10 | 5-10 searches each |
| **Depth-first** | Single topic requiring thorough understanding, technical deep-dives | 2-4 | 10-15 searches each |
| **Simple factual** | Single fact, specific data point, quick lookup | 1-2 | 3-5 searches each |

### Classification Decision

After reading the query, determine:
1. **Query type**: breadth / depth / simple
2. **Resource allocation**: how many sub-agents to spawn
3. **Search domains**: codebase, academic, technical docs, news, general web
4. **Scope**: codebase-only, web-only, or hybrid

### Codebase vs. Web Research

- If the query is about **this repository** (architecture, code patterns, dependencies, history), use codebase tools (Grep, Glob, Read, git commands).
- If the query is about **external topics** (technologies, standards, comparisons), use web tools (WebSearch, WebFetch).
- Many queries require **both**: spin up sub-agents for each domain.

## Phase 2: Parallel Research Execution

Spawn all sub-agents **in a single message** for true parallelization using the Task tool.

### Sub-agent Prompt Protocol

Each Task prompt MUST begin with a depth-mode trigger phrase:

| Mode | Trigger prefix | Expected effort |
|------|---------------|-----------------|
| Quick verification | "Quick check:", "Verify:", "Confirm:" | 3-5 searches |
| Focused investigation | "Investigate:", "Explore:", "Find details about:" | 5-10 searches |
| Deep research | "Deep dive:", "Comprehensive:", "Thorough research:" | 10-15 searches |

### Sub-agent Output Protocol (Filesystem Artifacts)

**Each sub-agent MUST:**
1. Write its full report to `/tmp/research_[timestamp]_[topic_slug].md`
2. Return ONLY:
   - File path to the full report
   - 2-3 sentence summary
   - Key topics covered
   - Number of sources found

This reduces token usage by approximately 90% compared to passing full reports inline.

### Example Dispatch Patterns

**Breadth-first** ("Compare approach A vs approach B vs approach C"):
```
Task 1: "Investigate: approach A architecture, performance characteristics, and ecosystem maturity"
Task 2: "Investigate: approach B architecture, performance characteristics, and ecosystem maturity"
Task 3: "Investigate: approach C architecture, performance characteristics, and ecosystem maturity"
Task 4: "Explore: performance benchmarks comparing the three approaches"
Task 5: "Investigate: developer experience, tooling, and community size for each"
Task 6: "Quick check: latest release dates and roadmap status for each"
```

**Depth-first** ("How does the spec compiler validation pipeline work?"):
```
Task 1: "Deep dive: trace the spec compiler entry point through all validation phases (codebase research using Grep/Read)"
Task 2: "Comprehensive: map all error codes and validation rules in the spec compiler"
Task 3: "Thorough research: document the data flow and intermediate representations used during compilation"
```

**Simple factual** ("What license does this project use?"):
```
Task 1: "Quick check: find the LICENSE file and any license declarations in package manifests"
```

## Phase 3: Synthesis from Filesystem Artifacts

After all sub-agents complete:

1. **Collect file references**: gather all `/tmp/research_*.md` paths from sub-agent responses.
2. **Read reports**: use Read to access each research artifact.
3. **Merge findings**:
   - Identify common themes across reports.
   - Deduplicate overlapping information.
   - Preserve unique insights from each report.
   - Flag contradictions between sources.
4. **Consolidate sources**:
   - Merge all cited sources and references.
   - Remove duplicates.
   - Organize by relevance and credibility.
5. **Write final report**: save to `/tmp/research_final_[timestamp].md`.

## Phase 4: Deliver Final Report

### Report Structure

```markdown
# Research Report: [Query Topic]

## Executive Summary
[3-5 paragraph overview synthesizing all findings]

## Key Findings
1. **[Finding 1]**: synthesized from multiple sub-agent reports
2. **[Finding 2]**: cross-referenced and verified
3. **[Finding 3]**: with supporting evidence

## Detailed Analysis

### [Theme 1]
[Comprehensive synthesis from all relevant sub-agent findings]

### [Theme 2]
[Comprehensive synthesis from all relevant sub-agent findings]

## Sources and References
[Consolidated list organized by type: codebase files, documentation, web sources]

## Research Metadata
- Query classification: [breadth / depth / simple]
- Sub-agents deployed: [count and focus areas]
- Total sources analyzed: [count]
- Research artifacts: [list of /tmp/research_*.md files]
```

### Presentation to User

1. Display the **Executive Summary** and **Key Findings** directly in conversation.
2. Provide the **path to the full report** file.
3. List **individual sub-agent report paths** so the user can drill into any area.
4. Highlight any **contradictions or gaps** found during synthesis.

## Research Quality Principles

- **Prefer authoritative sources**: official docs, academic papers, primary sources over blog posts.
- **Cross-reference claims**: no single-source conclusions for important findings.
- **Identify gaps**: explicitly state what could NOT be determined.
- **Distinguish fact from inference**: clearly label speculation or extrapolation.
- **Recency matters**: prefer recent sources; flag outdated information.

## Execution

Now classify the query and launch parallel research sub-agents.
