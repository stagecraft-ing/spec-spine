---
name: implement-plan
description: Execute a plan file step-by-step with progress tracking and phase checkpoints
allowed-tools: Bash, Read, Edit, Write, Glob, Grep, Agent
argument-hint: "<path-to-plan-file>"
---

# Implement Plan

Execute a plan document while maintaining progress tracking and checkpoint discipline.

## Input

Plan file path: $ARGUMENTS

If no path is provided, search for plan files:
1. Look for `*.plan.md` or files in a `plans/` directory.
2. List candidates and ask the user to select one.

## Phase 0: Parse Plan

1. **Read the plan file** at the given path.
2. **Extract structure**:
   - YAML frontmatter (if present): status, dates, metadata
   - Goals / objectives section
   - Acceptance criteria (these become trackable tasks)
   - Implementation details / steps
   - Any existing progress checkboxes
3. **Validate plan readiness**:
   - If no clear acceptance criteria or implementation steps exist, stop and ask for clarification.
   - If status is `completed`, confirm with user before re-implementing.
   - If status is `blocked`, ask what needs unblocking.

## Phase 1: Generate Task List and Initialize

### Build the checklist

Extract discrete tasks from:
- Acceptance criteria (each criterion = one checkbox)
- Implementation steps (each concrete step = one checkbox)
- Any sub-tasks described in the plan body

### Insert progress section

If the plan does not already have an "Implementation Progress" section, insert one after the first `#` heading:

```markdown
## Implementation Progress

- [ ] Task from acceptance criteria 1
- [ ] Task from acceptance criteria 2
- [ ] Implementation step A
- [ ] Implementation step B
```

### Update frontmatter

Update the plan's YAML frontmatter:
- Set `status` to `in-development` (from `draft`, `ready-for-development`, or similar)
- Set `startDate` to today's date if not already set
- Set `updated` to current ISO timestamp
- Set `progress` to `0`

### CHECKPOINT: Present the task list to the user and wait for approval before proceeding.

Show:
- Total number of tasks extracted
- The full checklist
- Estimated complexity (low / medium / high based on task count and plan detail)

**Do not begin implementation until the user confirms.**

## Phase 2: Implementation

Work through each task systematically:

### Per-task loop

1. **Announce** which task you are starting.
2. **Implement** the task: write code, create files, modify configs as needed.
3. **Verify** the task:
   - Run relevant build commands, type checks, or tests as applicable (`<your build command>`, `<your test command>`).
   - Never disable or skip failing tests; fix them.
4. **Update the plan file**:
   - Check off the completed checkbox: `- [x] Task description`
   - Update `progress` in frontmatter: `(completed / total) * 100`, rounded to nearest integer
   - Update `updated` timestamp
5. **Move to next task.**

### Implementation rules

- **Read the entire plan** before starting. Understand the full scope so early decisions support later tasks.
- **Keep the plan file in sync** after every completed task, not in batches.
- **Do not commit automatically.** Only commit if the user explicitly requests it.
- **Never skip tests.** If tests fail, fix them.
- **Preserve plan structure.** Do not reorganize or rewrite existing plan sections. Only add the progress section and update frontmatter/checkboxes.
- **Run quality gates** where applicable (build, type-check, lint) to catch errors early.
- **Run the spec-spine gate chain** (`spec-spine compile`, `spec-spine index check`, `spec-spine lint --fail-on-warn`) after any change that touches spec-owned paths or `.derived/` regeneration.

### Mid-implementation checkpoint

At the halfway point (50% progress), pause and report:
- Tasks completed so far
- Any issues encountered
- Any deviations from the plan
- Remaining tasks

**Wait for user confirmation before continuing.**

## Phase 3: Completion

When all tasks are checked off:

1. **Update frontmatter**:
   - Set `status` to `in-review`
   - Set `progress` to `100`
   - Update `updated` timestamp

2. **Run final verification**: execute all relevant quality checks (tests, lints, type checks, builds) and report any failures.

3. **Deliver summary**:

```
## Implementation Complete

**Plan**: [plan title]
**Tasks completed**: X / X
**Status**: in-review
**Duration**: [start to now]

### What was done
- [Bullet summary of major changes]

### Files modified
- [List of key files created or changed]

### Verification results
- Tests: [pass/fail count]
- Build: [pass/fail]
- Spec-spine gate: [compile ok | lint ok | index ok | couple ok]

### Known issues or follow-ups
- [Any items that need attention]
```

## Status State Machine

Valid status transitions:

```
draft --> in-development --> in-review --> completed
                ^                |
                |                v
              blocked <------+
```

- `draft` or `ready-for-development`: initial states, transition to `in-development` when work begins.
- `in-development`: active implementation in progress.
- `blocked`: encountered a blocker; document the issue in the plan and notify the user.
- `in-review`: all tasks complete, awaiting review.
- `completed`: review passed (set by user, not by this command).

## Error Handling

| Situation | Action |
|-----------|--------|
| Plan file not found | Search for candidates, ask user for correct path |
| No frontmatter | Warn that this may not be a structured plan; offer to add frontmatter |
| Already completed | Confirm with user before re-implementing |
| Blocked status | Ask user to resolve blocker before proceeding |
| Test/build failure during task | Fix the issue, do not skip. If unfixable, mark task as blocked and continue with others |
| Ambiguous task | Ask user for clarification before implementing |
| Spec coupling failure | Surface the drift, do not silently patch the spec to match new code |

## Progress Calculation

- Count ONLY checkboxes in the "Implementation Progress" section.
- `progress = round((checked / total) * 100)`
- Update after every task completion, not in batches.
