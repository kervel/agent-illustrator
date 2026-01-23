# Quality Standards: Agent Illustrator

## Overview

| Attribute | Value |
|-----------|-------|
| Project | Agent Illustrator |
| Created | 2026-01-23 |
| Quality Level | Standard |

---

## Quality Gates

### Enforcement
- **Enforce Gates**: true
- **Block Merge on Failure**: true

### Thresholds
| Metric | Minimum | Target |
|--------|---------|--------|
| Quality Score | 80 | 90 |
| Test Coverage | 80% | 90% |

---

## Code Quality Metrics

### Complexity
- **Cyclomatic Complexity Threshold**: 10 per function
- **Max File Lines**: 500 (Rust files can be larger due to tests)
- **Max Function Lines**: 50
- **Max Function Parameters**: 5

### Rust-Specific
- `cargo clippy` must pass with no warnings
- `cargo fmt --check` must pass
- No `unsafe` blocks without `// SAFETY:` comment
- All public items must have rustdoc comments

---

## Testing Requirements

### Coverage
- **Require Tests**: true
- **Minimum Coverage**: 80%
- **Target Coverage**: 90%

### Test Types Required
| Type | Required | Tool |
|------|----------|------|
| Unit Tests | Yes | `cargo test` |
| Integration Tests | Yes | `cargo test --test '*'` |
| Snapshot Tests | Yes (for SVG output) | `insta` |
| Property Tests | Recommended | `proptest` |

### Test Quality
- Tests must verify behavior, not implementation
- Parser tests must include error case coverage
- SVG output tests should use snapshot testing
- Edge cases must be explicitly tested

---

## Performance Budgets

### Enforcement
- **Enforce Budgets**: true (after baseline established)

### Targets (to be refined)
| Metric | Budget |
|--------|--------|
| Parse time (1KB input) | < 10ms |
| Render time (simple illustration) | < 50ms |
| Memory usage | < 100MB for typical use |

*Note: Establish baselines before enforcing strictly*

---

## Documentation Standards

### Required Documentation
- README.md with project overview and quick start
- rustdoc for all public APIs
- Examples for each major feature
- Error messages must be self-documenting

### Language Specification
- Formal grammar documentation
- Semantic behavior documentation
- Examples for each language construct

---

## Code Review Requirements

### Process
- **Require Code Review**: true
- **Minimum Reviewers**: 1

### Review Checklist
- [ ] Tests added/updated for changes
- [ ] Documentation updated if API changed
- [ ] Error messages are clear and actionable
- [ ] No new clippy warnings introduced
- [ ] Snapshot tests reviewed if changed

---

## CI/CD Requirements

### Pre-Merge Checks
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] Coverage threshold met

### Release Checks
- [ ] All CI checks pass
- [ ] CHANGELOG.md updated
- [ ] Version bumped appropriately
- [ ] Documentation generated and reviewed

---

## Custom Checks

### Parser Quality
- All error paths must produce user-friendly messages
- Error recovery should not produce cascading errors
- Position information must be accurate in diagnostics

### Renderer Quality
- SVG output must be valid and well-formed
- Generated SVG should be human-readable (formatted)
- Layout must be deterministic (same input = same output)

### LLM-Friendliness
- Language syntax should be unambiguous
- Error messages should suggest corrections
- Common mistakes should have helpful diagnostics

---

## Exemptions

*No exemptions currently granted.*

To request an exemption:
1. Document the specific standard being exempted
2. Explain why the exemption is necessary
3. Define the scope and duration
4. Get team approval

---

## Notes

- Quality level: Standard (80% coverage, 80 quality score)
- Created by `/specswarm:init`
- Enforced by `/specswarm:ship` before merge
- Adjust thresholds as project matures

---

*Created: 2026-01-23*
*Last Updated: 2026-01-23*
