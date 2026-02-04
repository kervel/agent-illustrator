# Benchmarks

Standard prompts for evaluating Agent Illustrator output quality across versions.
Run each prompt with a fresh agent session using the skill doc (`--skill`), grammar (`--grammar`), and examples (`--examples`).

Record: version, model, iteration count, lint warnings, and a screenshot.

---

## Benchmark 1: IoT Edge Architecture

**Prompt:**

> Draw an IoT edge architecture. 3 edge devices on the left send data to an edge gateway. The gateway runs preprocessing and anomaly detection locally. Normal data flows right to the cloud (MQTT broker -> time-series DB -> dashboard). Anomaly alerts go directly up to an alerting service. Show an 'Edge' zone and a 'Cloud' zone with distinct backgrounds.

**What this tests:**

- Zones with background fills (contains constraint + opacity)
- Mixed connection types (straight data flow + curved anomaly alert)
- Label placement on horizontal, vertical, and curved connections
- Nested groups (gateway containing preprocessing + anomaly detection)
- Multi-element row layout (cloud pipeline)
- Via-point routing to avoid crossing elements

**Results:**

| Version | Model | Iterations | Lint clean | Notes |
|---------|-------|-----------|------------|-------|
| v0.1.12 | Claude Sonnet 4 | 5 | Yes (iter 4) | Curve crossing fixed by lint. Final polish for opacity and via-point tuning. |
