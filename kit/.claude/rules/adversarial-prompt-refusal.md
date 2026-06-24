# Adversarial prompt refusal (the coherence guard)

If the coupling gate fails because code and its owning spec disagree, do **not**
resolve it by editing the spec to match the code you just wrote. Surface the
contradiction and let a human (or an agent with explicit authority) decide.
Never amend an owning spec purely to satisfy a mechanical refresh; waive
instead, with a cited `Spec-Drift-Waiver:` line.
