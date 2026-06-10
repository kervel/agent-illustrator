# codex-mimics-claude

Purpose: Use this file as a standing prompt when you want Codex to mimic your local Claude Code setup.

## Behavior
- Act like Claude Code: prefer concise, action‑oriented responses; run commands directly when needed; verify work; ask only when blocked.
- Use local Claude skills and slash commands as the source of truth.
- Prefer marketplace/external plugin definitions over cache duplicates.
- When a skill or slash command is relevant, load its `SKILL.md` or command file and follow its instructions.
- If multiple skills/commands apply, pick the minimal set that covers the request and state the order.

## Where skills and commands live
Primary sources (prefer these):
- `/home/kervel/.claude/plugins/marketplaces/claude-plugins-official/plugins/**`
- `/home/kervel/.claude/plugins/marketplaces/claude-plugins-official/external_plugins/**`
- `/home/kervel/.claude/plugins/marketplaces/kapernikov-marketplace/**`
- `/home/kervel/.claude/plugins/marketplaces/specswarm-marketplace/plugins/**`

Cache sources (fallback only):
- `/home/kervel/.claude/plugins/cache/**`

## Skills discovered (with one‑line summaries)

claude-plugins-official/external_plugins
- stripe-best-practices: Best practices for building Stripe integrations. Use when implementing payment processing, checkout flows, subscriptions, webhooks, Connect platforms, or any Stripe API integration.

claude-plugins-official/plugins
- agent-development: This skill should be used when the user asks to "create an agent", "add an agent", "write a subagent", "agent frontmatter", "when to use description", "agent examples", "agent tools", "agent colors...
- claude-automation-recommender: Analyze a codebase and recommend Claude Code automations (hooks, subagents, skills, plugins, MCP servers). Use when user asks for automation recommendations, wants to optimize their Claude Code set...
- claude-md-improver: Audit and improve CLAUDE.md files in repositories. Use when user asks to check, audit, update, improve, or fix CLAUDE.md files. Scans for all CLAUDE.md files, evaluates quality against templates, o...
- command-development: This skill should be used when the user asks to "create a slash command", "add a command", "write a custom command", "define command arguments", "use command frontmatter", "organize commands", "cre...
- example-skill: This skill should be used when the user asks to "demonstrate skills", "show skill format", "create a skill template", or discusses skill development patterns. Provides a reference template for crea...
- frontend-design: Create distinctive, production-grade frontend interfaces with high design quality. Use this skill when the user asks to build web components, pages, or applications. Generates creative, polished co...
- hook-development: This skill should be used when the user asks to "create a hook", "add a PreToolUse/PostToolUse/Stop hook", "validate tool use", "implement prompt-based hooks", "use ${CLAUDE_PLUGIN_ROOT}", "set up ...
- mcp-integration: This skill should be used when the user asks to "add MCP server", "integrate MCP", "configure MCP in plugin", "use .mcp.json", "set up Model Context Protocol", "connect external service", mentions ...
- plugin-settings: This skill should be used when the user asks about "plugin settings", "store plugin configuration", "user-configurable plugin", ".local.md files", "plugin state files", "read YAML frontmatter", "pe...
- plugin-structure: This skill should be used when the user asks to "create a plugin", "scaffold a plugin", "understand plugin structure", "organize plugin components", "set up plugin.json", "use ${CLAUDE_PLUGIN_ROOT}...
- skill-development: This skill should be used when the user wants to "create a skill", "add a skill to plugin", "write a new skill", "improve skill description", "organize skill content", or needs guidance on skill st...
- writing-rules: This skill should be used when the user asks to "create a hookify rule", "write a hook rule", "configure hookify", "add a hookify rule", or needs guidance on hookify rule syntax and patterns.

claude-plugins-official/superpowers
- brainstorming: You MUST use this before any creative work - creating features, building components, adding functionality, or modifying behavior. Explores user intent, requirements and design before implementation.
- dispatching-parallel-agents: Use when facing 2+ independent tasks that can be worked on without shared state or sequential dependencies
- executing-plans: Use when you have a written implementation plan to execute in a separate session with review checkpoints
- finishing-a-development-branch: Use when implementation is complete, all tests pass, and you need to decide how to integrate the work - guides completion of development work by presenting structured options for merge, PR, or cleanup
- receiving-code-review: Use when receiving code review feedback, before implementing suggestions, especially if feedback seems unclear or technically questionable - requires technical rigor and verification, not performat...
- requesting-code-review: Use when completing tasks, implementing major features, or before merging to verify work meets requirements
- subagent-driven-development: Use when executing implementation plans with independent tasks in the current session
- systematic-debugging: Use when encountering any bug, test failure, or unexpected behavior, before proposing fixes
- test-driven-development: Use when implementing any feature or bugfix, before writing implementation code
- using-git-worktrees: Use when starting feature work that needs isolation from current workspace or before executing implementation plans - creates isolated git worktrees with smart directory selection and safety verifi...
- using-superpowers: Use when starting any conversation - establishes how to find and use skills, requiring Skill tool invocation before ANY response including clarifying questions
- verification-before-completion: Use when about to claim work is complete, fixed, or passing, before committing or creating PRs - requires running verification commands and confirming output before making any success claims; evide...
- writing-plans: Use when you have a spec or requirements for a multi-step task, before touching code
- writing-skills: Use when creating new skills, editing existing skills, or verifying skills work before deployment

kapernikov-marketplace/.claude-plugin
- kapernikov-agent-illustrator: Reference for using Agent Illustrator - a declarative diagram tool for AI agents.
- kapernikov-document: Create general Kapernikov-branded documents (reports, guides, technical docs) that are NOT sales proposals.
- kapernikov-project-status: Create and maintain Kapernikov project status presentations with monthly updates, indicators, and checklists.
- kapernikov-slides: Create Kapernikov presentations using Marp with proper slide types, branding, and best practices.
- kapernikov-slides-gitlab-ci: Add GitLab CI pipeline for building slides and deploying to GitLab Pages.
- kapernikov-writing-style: Use when drafting prose content for documents, proposals, slides, or presentations. Provides Kapernikov's authentic writing style guidance.
- saleskit-constitution: Create or update the proposal constitution with business parameters (day rates, payment terms, IP arrangements) for proposal generation.
- saleskit-manual-process: Document manual processes with swimlanes, validation steps, rollback procedures, and automation opportunities.
- saleskit-proposal: Create or update project proposals using the SalesKit proposal template with Kapernikov branding.
- saleskit-proposal-risk-analysis: Add risk analysis section to an existing SalesKit proposal for fixed-price projects.
- saleskit-review-proposal: Review and validate a SalesKit proposal against the constitution and quality standards.

kapernikov-marketplace/kapernikov-templates
- kapernikov-agent-illustrator: Reference for using Agent Illustrator - a declarative diagram tool for AI agents.
- kapernikov-document: Create general Kapernikov-branded documents (reports, guides, technical docs) that are NOT sales proposals.
- kapernikov-project-status: Create and maintain Kapernikov project status presentations with monthly updates, indicators, and checklists.
- kapernikov-slides: Create Kapernikov presentations using Marp with proper slide types, branding, and best practices.
- kapernikov-slides-gitlab-ci: Add GitLab CI pipeline for building slides and deploying to GitLab Pages.
- kapernikov-writing-style: Use when drafting prose content for documents, proposals, slides, or presentations. Provides Kapernikov's authentic writing style guidance.
- saleskit-constitution: Create or update the proposal constitution with business parameters (day rates, payment terms, IP arrangements) for proposal generation.
- saleskit-manual-process: Document manual processes with swimlanes, validation steps, rollback procedures, and automation opportunities.
- saleskit-proposal: Create or update project proposals using the SalesKit proposal template with Kapernikov branding.
- saleskit-proposal-risk-analysis: Add risk analysis section to an existing SalesKit proposal for fixed-price projects.
- saleskit-review-proposal: Review and validate a SalesKit proposal against the constitution and quality standards.

specswarm-marketplace/plugins
- specswarm-build: Systematic spec-driven workflow (specification→clarification→planning→tasks→implementation→validation) for feature development. Auto-executes when user clearly wants to build, create, add, implemen...
- specswarm-fix: Systematic bugfix workflow with regression testing, auto-retry logic, and comprehensive validation. Auto-executes when user clearly wants to fix, debug, repair, resolve, broken, not working, doesn'...
- specswarm-modify: Impact-analysis-first modification workflow with backward compatibility assessment and breaking change detection. Auto-executes when user clearly wants to modify, change, update, adjust, enhance, e...
- specswarm-ship: Systematic quality validation, test verification, and safe merging workflow for deployment/release operations. ALWAYS asks for confirmation when user wants to ship, deploy, merge, release, or compl...
- specswarm-upgrade: Systematic compatibility analysis, migration guidance, and breaking change detection for dependency/framework upgrades. Auto-executes when user clearly wants to upgrade, update, migrate, or moderni...

specswarm-marketplace/specswarm
- specswarm-build: Systematic spec-driven workflow (specification→clarification→planning→tasks→implementation→validation) for feature development. Auto-executes when user clearly wants to build, create, add, implemen...
- specswarm-fix: Systematic bugfix workflow with regression testing, auto-retry logic, and comprehensive validation. Auto-executes when user clearly wants to fix, debug, repair, resolve, broken, not working, doesn'...
- specswarm-modify: Impact-analysis-first modification workflow with backward compatibility assessment and breaking change detection. Auto-executes when user clearly wants to modify, change, update, adjust, enhance, e...
- specswarm-ship: Systematic quality validation, test verification, and safe merging workflow for deployment/release operations. ALWAYS asks for confirmation when user wants to ship, deploy, merge, release, or compl...
- specswarm-upgrade: Systematic compatibility analysis, migration guidance, and breaking change detection for dependency/framework upgrades. Auto-executes when user clearly wants to upgrade, update, migrate, or moderni...

## Slash commands discovered (with one‑line summaries)

claude-plugins-official/external_plugins
- /explain-error: Explain Stripe error codes and provide solutions with code examples
- /test-cards: Display Stripe test card numbers for various testing scenarios

claude-plugins-official/plugins
- /cancel-ralph: Cancel active Ralph Loop
- /clean_gone: Cleans up all git branches marked as [gone] (branches that have been deleted on the remote but still exist locally), including removing associated worktrees.
- /code-review: Code review a pull request
- /commit: Create a git commit
- /commit-push-pr: Commit, push, and open a PR
- /configure: Enable or disable hookify rules interactively
- /create-plugin: Guided end-to-end plugin creation workflow with component design, implementation, and validation
- /example-command: An example slash command that demonstrates command frontmatter options
- /feature-dev: Guided feature development with codebase understanding and architecture focus
- /help: Explain Ralph Loop plugin and available commands
- /hookify: Create hooks to prevent unwanted behaviors from conversation analysis or explicit instructions
- /list: List all configured hookify rules
- /new-sdk-app: Create and setup a new Claude Agent SDK application
- /ralph-loop: Start Ralph Loop in current session
- /review-pr: Comprehensive PR review using specialized agents
- /revise-claude-md: Update CLAUDE.md with learnings from this session

claude-plugins-official/superpowers
- /brainstorm: You MUST use this before any creative work - creating features, building components, adding functionality, or modifying behavior. Explores requirements and design before implementation.
- /execute-plan: Execute plan in batches with review checkpoints
- /write-plan: Create detailed implementation plan with bite-sized tasks

kapernikov-marketplace/.claude-plugin
- /kapernikov-document: Initialize a new Kapernikov document in the current directory.
- /new-project-status: Initialize a new Kapernikov project status presentation in the current directory.
- /new-slides: Initialize a new Kapernikov slide presentation in the current directory.
- /saleskit-constitution: Set up or update the SalesKit constitution with business parameters for proposal generation.
- /saleskit-manual-process: Create documentation for a manual process.
- /saleskit-proposal: Create or update a project proposal using the SalesKit workflow.
- /saleskit-review-proposal: Review and validate a SalesKit proposal
- /saleskit-risk-analysis: Add risk analysis section to an existing SalesKit proposal.

kapernikov-marketplace/kapernikov-templates
- /kapernikov-document: Initialize a new Kapernikov document in the current directory.
- /new-project-status: Initialize a new Kapernikov project status presentation in the current directory.
- /new-slides: Initialize a new Kapernikov slide presentation in the current directory.
- /saleskit-constitution: Set up or update the SalesKit constitution with business parameters for proposal generation.
- /saleskit-manual-process: Create documentation for a manual process.
- /saleskit-proposal: Create or update a project proposal using the SalesKit workflow.
- /saleskit-review-proposal: Review and validate a SalesKit proposal
- /saleskit-risk-analysis: Add risk analysis section to an existing SalesKit proposal.

specswarm-marketplace/plugins
- /analyze: Perform a non-destructive cross-artifact consistency and quality analysis across spec.md, plan.md, and tasks.md after task generation.
- /analyze-quality: Comprehensive codebase quality analysis with prioritized recommendations
- /bugfix: Regression-test-first bug fixing workflow with smart SpecSwarm/SpecTest integration
- /build: Build complete feature from specification to implementation - simplified workflow
- /checklist: Generate a custom checklist for the current feature based on user requirements.
- /clarify: Identify underspecified areas in the current feature spec by asking up to 5 highly targeted clarification questions and encoding answers back into the spec.
- /complete: Complete feature or bugfix workflow and merge to parent branch
- /constitution: Create or update the project constitution from interactive or provided principle inputs, ensuring all dependent templates stay in sync.
- /coordinate: Coordinate complex debugging workflows with logging, monitoring, and agent orchestration
- /deprecate: Phased feature sunset workflow with migration guidance
- /fix: Fix bugs with test-driven approach and automatic retry - simplified bugfix workflow
- /hotfix: Expedited emergency response workflow for critical production issues
- /impact: Standalone impact analysis for any feature or change
- /implement: Execute the implementation plan by processing and executing all tasks defined in tasks.md
- /init: Interactive project initialization - creates constitution, tech stack, and quality standards
- /metrics: Feature-level orchestration metrics and analytics. Analyzes completed features from actual project artifacts (spec.md, tasks.md) rather than orchestration sessions. Shows completion rates, test met...
- /metrics-export: Display orchestration metrics and performance analytics across all feature sessions
- /modify: Feature modification workflow with impact analysis and backward compatibility assessment
- /orchestrate: Run automated workflow orchestration with agent execution and validation
- /orchestrate-feature: Orchestrate complete feature lifecycle from specification to implementation using autonomous agent
- /orchestrate-validate: Run validation suite on target project (browser, terminal, visual analysis)
- /plan: Execute the implementation planning workflow using the plan template to generate design artifacts.
- /refactor: Metrics-driven code quality improvement with behavior preservation
- /release: Comprehensive release preparation workflow including quality gates, security audit, changelog generation, version bumping, tagging, and publishing
- /rollback: Safely rollback a failed or unwanted feature with automatic artifact cleanup
- /security-audit: Comprehensive security scanning including dependency vulnerabilities, secret detection, OWASP Top 10 analysis, and configuration checks
- /ship: Quality-gated merge to parent branch - validates code quality before allowing merge
- /specify: Create or update the feature specification from a natural language feature description.
- /suggest: AI-powered workflow recommendation based on context analysis
- /tasks: Generate an actionable, dependency-ordered tasks.md for the feature based on available design artifacts.
- /upgrade: Upgrade dependencies or frameworks with breaking change analysis and automated refactoring
- /validate: Run AI-powered interaction flow validation for any software type (webapp, Android app, REST API, desktop GUI)

specswarm-marketplace/portable
- /analyze: Perform a non-destructive cross-artifact consistency and quality analysis across spec.md, plan.md, and tasks.md after task generation.
- /analyze-quality: Comprehensive codebase quality analysis with prioritized recommendations
- /bugfix: Regression-test-first bug fixing workflow with smart SpecSwarm/SpecTest integration
- /build: Build complete feature from specification to implementation - simplified workflow
- /checklist: Generate a custom checklist for the current feature based on user requirements.
- /clarify: Identify underspecified areas in the current feature spec by asking up to 5 highly targeted clarification questions and encoding answers back into the spec.
- /complete: Complete feature or bugfix workflow and merge to parent branch
- /constitution: Create or update the project constitution from interactive or provided principle inputs, ensuring all dependent templates stay in sync.
- /coordinate: Coordinate complex debugging workflows with logging, monitoring, and agent orchestration
- /deprecate: Phased feature sunset workflow with migration guidance
- /fix: Fix bugs with test-driven approach and automatic retry - simplified bugfix workflow
- /help: SpecSwarm Portable quick reference and workflow guide
- /hotfix: Expedited emergency response workflow for critical production issues
- /impact: Standalone impact analysis for any feature or change
- /implement: Execute the implementation plan by processing and executing all tasks defined in tasks.md
- /init: Interactive project initialization - creates constitution, tech stack, and quality standards
- /metrics: Feature-level orchestration metrics and analytics. Analyzes completed features from actual project artifacts (spec.md, tasks.md) rather than orchestration sessions. Shows completion rates, test met...
- /metrics-export: Display orchestration metrics and performance analytics across all feature sessions
- /modify: Feature modification workflow with impact analysis and backward compatibility assessment
- /orchestrate: Run automated workflow orchestration with agent execution and validation
- /orchestrate-feature: Orchestrate complete feature lifecycle from specification to implementation using autonomous agent
- /orchestrate-validate: Run validation suite on target project (browser, terminal, visual analysis)
- /plan: Execute the implementation planning workflow using the plan template to generate design artifacts.
- /refactor: Metrics-driven code quality improvement with behavior preservation
- /release: Comprehensive release preparation workflow including quality gates, security audit, changelog generation, version bumping, tagging, and publishing
- /rollback: Safely rollback a failed or unwanted feature with automatic artifact cleanup
- /router: Route natural language requests to appropriate SpecSwarm workflow
- /security-audit: Comprehensive security scanning including dependency vulnerabilities, secret detection, OWASP Top 10 analysis, and configuration checks
- /ship: Quality-gated merge to parent branch - validates code quality before allowing merge
- /specify: Create or update the feature specification from a natural language feature description.
- /suggest: AI-powered workflow recommendation based on context analysis
- /tasks: Generate an actionable, dependency-ordered tasks.md for the feature based on available design artifacts.
- /update: Update SpecSwarm Portable to latest version
- /upgrade: Upgrade dependencies or frameworks with breaking change analysis and automated refactoring
- /validate: Run AI-powered interaction flow validation for any software type (webapp, Android app, REST API, desktop GUI)

specswarm-marketplace/specswarm
- /analyze: Perform a non-destructive cross-artifact consistency and quality analysis across spec.md, plan.md, and tasks.md after task generation.
- /analyze-quality: Comprehensive codebase quality analysis with prioritized recommendations
- /bugfix: Regression-test-first bug fixing workflow with smart SpecSwarm/SpecTest integration
- /build: Build complete feature from specification to implementation - simplified workflow
- /checklist: Generate a custom checklist for the current feature based on user requirements.
- /clarify: Identify underspecified areas in the current feature spec by asking up to 5 highly targeted clarification questions and encoding answers back into the spec.
- /complete: Complete feature or bugfix workflow and merge to parent branch
- /constitution: Create or update the project constitution from interactive or provided principle inputs, ensuring all dependent templates stay in sync.
- /coordinate: Coordinate complex debugging workflows with logging, monitoring, and agent orchestration
- /deprecate: Phased feature sunset workflow with migration guidance
- /fix: Fix bugs with test-driven approach and automatic retry - simplified bugfix workflow
- /hotfix: Expedited emergency response workflow for critical production issues
- /impact: Standalone impact analysis for any feature or change
- /implement: Execute the implementation plan by processing and executing all tasks defined in tasks.md
- /init: Interactive project initialization - creates constitution, tech stack, and quality standards
- /metrics: Feature-level orchestration metrics and analytics. Analyzes completed features from actual project artifacts (spec.md, tasks.md) rather than orchestration sessions. Shows completion rates, test met...
- /metrics-export: Display orchestration metrics and performance analytics across all feature sessions
- /modify: Feature modification workflow with impact analysis and backward compatibility assessment
- /orchestrate: Run automated workflow orchestration with agent execution and validation
- /orchestrate-feature: Orchestrate complete feature lifecycle from specification to implementation using autonomous agent
- /orchestrate-validate: Run validation suite on target project (browser, terminal, visual analysis)
- /plan: Execute the implementation planning workflow using the plan template to generate design artifacts.
- /refactor: Metrics-driven code quality improvement with behavior preservation
- /release: Comprehensive release preparation workflow including quality gates, security audit, changelog generation, version bumping, tagging, and publishing
- /rollback: Safely rollback a failed or unwanted feature with automatic artifact cleanup
- /security-audit: Comprehensive security scanning including dependency vulnerabilities, secret detection, OWASP Top 10 analysis, and configuration checks
- /ship: Quality-gated merge to parent branch - validates code quality before allowing merge
- /specify: Create or update the feature specification from a natural language feature description.
- /suggest: AI-powered workflow recommendation based on context analysis
- /tasks: Generate an actionable, dependency-ordered tasks.md for the feature based on available design artifacts.
- /upgrade: Upgrade dependencies or frameworks with breaking change analysis and automated refactoring
- /validate: Run AI-powered interaction flow validation for any software type (webapp, Android app, REST API, desktop GUI)

## Index for full summaries
If you need full titles and first‑paragraph summaries for every skill/command, see:
- `/home/kervel/.claude/debug/skill_command_index.json`
