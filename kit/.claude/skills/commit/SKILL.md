---
name: commit
description: Create a git commit with an impact-focused conventional commit message.
allowed-tools: Bash
---

# Commit

Create a git commit following these steps.

## 1. Survey the changes

```
git status
git diff --cached
git diff
git log --oneline -5
```

Identify what is staged vs unstaged, the nature of each change (feature,
fix, refactor, docs, test, chore), and the user-visible impact. Match the
scoping conventions visible in recent history.

## 2. Draft a conventional-commit message

Format: `type(scope): subject`

**Type (required):** `feat`, `fix`, `refactor`, `docs`, `test`, `chore`.
Add a scope when it clarifies the affected area, e.g. `feat(compiler):`,
`fix(coupling):`, `docs(standards):`.

**Subject line:**
- 72 characters maximum (hard limit; count them).
- Lead with the impact or problem solved, not the technique used.
- No trailing period. No emojis.

**Good vs bad:**
- BAD: `refactor: extract helper for diagnostic formatting`
- GOOD: `fix: lint no longer crashes on an empty spec body`
- BAD: `feat: add new subcommand handler`
- GOOD: `feat(registry): relationships query for a spec neighborhood`

**Body (optional):** separate from the subject with a blank line. Use
dash-prefixed bullets only for multiple distinct changes. Keep lines
under 72 characters. Explain how only when it is non-obvious; the subject
already covers what and why.

**Issue linking:** `Fixes #NNN` or `Closes #NNN` on its own line after
the body, when applicable.

## 3. Stage the relevant files

Use `git add` with specific paths. Do not use `git add -A` or `git add .`
unless every changed file belongs in this commit. Never stage files that
look like secrets (`.env`, credentials, tokens).

## 4. Create the commit

Pass the message via heredoc:

```
git commit -m "$(cat <<'EOF'
type(scope): subject line here

Optional body with details.
EOF
)"
```

## 5. Verify

Run `git status` to confirm the commit succeeded and the tree is in the
expected state.

## Banned content

- No `Co-Authored-By` or any AI/Claude attribution line.
- No marketing taglines, links, or promotional text.
- No emojis anywhere in the message.
- No padding about what was not changed. Be direct and factual.

$ARGUMENTS
