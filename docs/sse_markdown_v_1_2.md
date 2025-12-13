<!--
Title: Synavera Script Etiquette — Markdown & Documentation Profile
Version: 1.2
Status: Stable
SSE Profile: Markdown & Documentation v1.2; Universal SSE v1.2
Audience: Synavera developers, maintainers, security auditors, support, operator-doc authors, and tooling authors
Scope: Markdown and mixed-format documentation (README, guides, ADRs, threat models, operator docs) under Synavera standards
Last Reviewed: 2025-12-06T05:50:00Z
Security / Safety: Non-compliant documentation may misrepresent risk, mislead operators, or undermine auditability across Synavera systems
Migration Note: This Markdown & Documentation profile has been elevated to v1.2 by aligning its own structure and metadata with SSE-Markdown; documents authored against v1.1 remain valid but should be re-audited against this version
Linked Artifacts:
  - Universal SSE: v1.2
  - SSE-JSON: v1.2
  - SSE-YAML: v1.2
  - Language Profiles: SSE-{Bash,C Core,C Secure,C++,Integrated Control,Java,Kotlin,Python,Rust} v1.2
-->

# Synavera Script Etiquette — Markdown & Documentation Profile v1.2

Markdown under Synavera is not “just fancy text.” It is the visible skin of the system: the place where intent, risk, and expectations are explained to humans.

Documentation is treated as a **first-class part of the security and ethics surface**:

- it shapes user behaviour,
- it frames how risk is perceived,
- it teaches people how to run, debug, and safely retire the software,
- and it acts as the human-readable mirror of what the code and configs are doing.

This profile exists so that any Synavera-flavoured Markdown document:

- declares what it is and why it exists,
- is honest about what it might make you do to your system,
- stays aligned with the underlying code and configuration,
- and is readable by everyone from a seasoned engineer to a curious troubleshooting user.

Documentation here is treated as a guided conversation with the reader:
clear, explicit, slightly opinionated, and always answerable to the user.

---

## 1. Role of Documentation Under Synavera

**Chosen Approach:** Docs as operational artefacts, not decoration

In Synavera, documentation is never “extra.” If the system has behaviour that matters, there should be at least one Markdown document that:

- explains that behaviour,
- explains why it is designed that way,
- and points to the code and config that implement it.

A Synavera Markdown document is assumed to be at least one of the following:

- A **launchpad**: README, quickstart, install guide, introductory overview.
- A **map**: architecture overview, module tour, ADR (Architecture Decision Record), design notes.
- A **safety rail**: threat model, security notes, operational runbook, incident post-mortem, rollback playbook.
- A **contract surface**: human-readable explanation of what configs, APIs, or policies really mean for the user.

The same document can play multiple roles, but it should be clear which roles it covers. For example, a README might be both a launchpad and a very light map, while a dedicated operator handbook is a safety rail and a contract surface.

Because of that, docs must never:

- Smuggle in dangerous commands without warning.
- Handwave away risk with “just run this as root” or “trust us, it’s fine.”
- Contradict the behaviour of the underlying code or configuration.
- Suggest “temporary” unsafe workarounds without documenting their impact and a path back to safety.

Where documentation and code conflict about risk or behaviour, the combined system is considered **unsafe**
until the disagreement is resolved and the docs updated. The resolution can be “code is wrong, fix code” or “docs are wrong, fix docs,” but the mismatch must not be allowed to linger silently.

---

## 2. Document Skeleton and Metadata

**Chosen Approach:** Explicit front-matter, stable identity

Every substantial Markdown document (README, guide, ADR, operator manual, threat model, AI-prompt spec) must begin with a short, structured metadata band.

This band answers three questions immediately:

1. *What is this?*
2. *Who is it for?*
3. *How old and how trusted is it?*

In plain Markdown (no YAML frontmatter required), aim for something like:

```markdown
<!--
Title: DCIPHERED — Operator Handbook
Version: 1.3
Status: Stable
SSE Profile: Markdown & Documentation v1.2
Audience: Operators, Support, Curious Power Users
Scope: Day-to-day use, diagnostics, and safe recovery for DCIPHERED agents.
Last Reviewed: 2025-11-27 (UTC)
Security / Safety: Contains commands that modify system state; some require elevated privileges.
Linked Artifacts:
  - Code: /agents/dciphered_daemon (SSE-Rust v1.1)
  - Config: config/dciphered.yaml (SSE-YAML v1.1)
  - Runbook: docs/dciphered-recovery.md (this profile)
-->

# DCIPHERED — Operator Handbook
```

Minimum fields:

- `Title` – human-readable document name.
- `Version` – doc version, not necessarily the same as the software version.
- `Status` – e.g. Draft, Experimental, Stable, Deprecated, Archived.
- `SSE Profile` – must explicitly reference `Markdown & Documentation v1.2` once this profile is adopted.
- `Audience` – who this is for (developer, operator, new user, auditor, AI tooling).
- `Scope` – what this document covers and, importantly, what it does **not** cover.
- `Last Reviewed` – in UTC, ISO-8601 (`YYYY-MM-DD` acceptable; full timestamp optional).
- `Security / Safety` – one or two sentences of honest risk framing.
- `Linked Artifacts` – where to look for the code, config, and companion docs that this document describes.

Optional but recommended fields:

- `Owner` – the person or team responsible for keeping this document accurate.
- `Applies To` – explicit version range of the software (e.g. `DCIPHERED v0.8.x`).
- `Tags` – short labels like `onboarding`, `operator`, `threat-model`, `ai-prompt`.

This metadata is the human-facing parallel to `_meta` blocks in SSE-JSON/YAML.
It tells a future reader whether they can trust this document, and what parts of the system it speaks for.
A missing or obviously stale metadata band should be treated as a warning sign that the doc may be outdated.

---

## 3. Tone, Readability, and Multi-Layer Audience

**Chosen Approach:** Layered explanations, jargon with handrails

Docs are written for three overlapping groups:

- **Experienced developers** looking for structure, invariants, and edge cases.
- **Operators and support** staff who need reliable checklists and workflows.
- **Curious users** who want to understand what is happening to their system, even if they never plan to edit code.

Different sections can lean toward different audiences, but the overall document should feel like one coherent voice that respects all three.

To support this, Markdown should:

- Start sections with a short, plain-language summary before diving into detail.
- Introduce jargon with a quick definition at first use, especially for cryptography, networking, or distributed-systems terms.
- Prefer short paragraphs and descriptive headings to dense walls of text.
- Use examples liberally when explaining workflows or configuration patterns.
- Make it easy to skim: headings, subheadings, and occasional lists are encouraged where they clarify structure.

A useful pattern is:

1. **Summary line** in plain language.
2. **Why it matters** in one or two sentences.
3. **Details** for those who need them.
4. **Links** to deeper references (code, config, ADRs).

Avoid performative cleverness or memes in safety-critical sections.
Humour is allowed, but never at the expense of clarity, consent, or accuracy.
If a joke could confuse a tired operator at 3AM, the joke loses.

---

## 4. Security, Safety, and Risk Annotation in Docs

**Chosen Approach:** Make risk impossible to miss

Any documentation that teaches people to:

- run commands,
- change configuration,
- open network ports,
- handle secrets or credentials,
- modify trust anchors or certificates,
- or disable safeguards,

must annotate risk explicitly.

Use consistent callouts:

```markdown
> **Security Note:** The commands below generate a new keypair and write it to disk.
> They must be run as the user who will operate DCIPHERED, not as root.

> **Safety Note:** This step restarts the system service.
> If you are on a production machine, schedule a maintenance window.
```

Conventions:

- Use **Security Note** for anything involving confidentiality, integrity, authentication, or access control.
- Use **Safety Note** for operational risk: downtime, data loss, performance impact, or local machine state.
- Use **Privacy Note** where the behaviour may involve telemetry, logging of personal data, or external services.
- Do not hide these in the middle of a paragraph: give them their own blockquotes or subheadings.
- When in doubt, err on the side of over-annotating rather than assuming the reader will “just know”.

Docs must never encourage:

- `curl ... | sh` as a default or recommended installation path.
- Blindly disabling OS security features or SELinux/AppArmor profiles without explaining consequences and alternatives.
- Copy-pasting secrets into logs, screenshots, or long-term shared documents.
- Sharing real API keys, tokens, or private material in screenshots or examples.

If such patterns must be shown (for example, explaining why they are bad), mark them as **examples of what not to do** and avoid making them copy-paste friendly.
Use obviously fake placeholders (`DO_NOT_USE_REAL_KEY_HERE`) and surround with warnings.

Risk annotations should be kept up to date. If a new class of risk is discovered (for example, a new side-effect of a config flag), update both the relevant section and its Security / Safety / Privacy Notes.

---

## 5. Code Blocks, Commands, and Examples

**Chosen Approach:** Safe by default, annotated when sharp

Every executable snippet in Markdown is an invitation for a reader to run it.
Treat code blocks as a contract: if someone copy-pastes this into their terminal or editor, the outcome should be predictable and consistent with the safety notes around it.

### 5.1 General rules

- Use fenced code blocks with language hints where appropriate (`bash`, `sh`, `rust`, `python`, `json`, `yaml`, `toml`, etc.).
- Prefer commands that are idempotent or explicitly safe, especially in quickstarts.
- For commands with side effects, explain:
  - what they do,
  - what they change,
  - how to undo them (where feasible),
  - and what to check afterwards to confirm success.
- Never rely on “you should know what this does before running it” as a safety net.
- Tag examples that are **destructive** or **irreversible** with a clear warning.

Examples of good patterns:

```bash
# As your regular user: verify the agent can start in dry-run mode.
dciphered --dry-run --config ./config/dciphered.yaml
```

```bash
# As root: install the system service once you are satisfied with the dry-run.
sudo dciphered install --systemd
```

### 5.2 Privilege and context

Always be explicit about who should run a command and in what context:

```bash
# As your regular user in a development environment:
dciphered --self-check
```

```bash
# As root (required to install system service on a production host):
sudo dciphered install --systemd
```

Where a command must be run as root or with `sudo`, pair it with a Security or Safety Note explaining why and what could go wrong. If there is a way to perform the same action without persistent privilege escalation (for example, using a temporary capability or a privileged helper), mention it.

If a command is only safe on non-production machines, say so explicitly.

### 5.3 Copy-paste ergonomics

Docs should not tempt someone to run a command that is obviously wrong for their environment just because it is easy to copy. To support safe copy-paste:

- Avoid shell prompts (`$`, `#`) **inside** copy-pastable blocks; keep them in comments if needed.
- Show environment variables and placeholders clearly, for example:
  - `YOUR_DOMAIN_HERE`
  - `PATH_TO_CONFIG`
  - `DCIPHERED_DATA_DIR`
- Explain placeholders right above or below the block, so the reader knows what to substitute.
- Keep multi-step sequences short and logical; if a process is long, break it into sub-steps with intermediate checks.

If a code block is meant as **illustration only** and should *not* be run as-is, label it clearly:

```markdown
> **Example Only:** The configuration fragment below is illustrative.
> Do not copy it directly into production without adapting limits and paths.
```

---

## 6. Linking, Cross-References, and Versioning

**Chosen Approach:** Traceable from prose to code and config

Any Markdown that describes specific behaviour, configuration, or APIs must link to the authoritative artefacts:

- Source code modules (by path, and optionally by function or class name).
- Versioned configuration files (SSE-JSON/YAML artifacts).
- API schemas or OpenAPI documents.
- ADRs that explain why a design choice was made.
- External standards, RFCs, or specs where relevant.

Examples:

```markdown
For the exact default values and environment overrides, see:

- `config/dciphered.yaml` (SSE-YAML v1.1)
- `src/agent/config/defaults.rs` (SSE-Rust v1.1)
- `docs/adr/0003-config-defaults.md` (decision rationale)
```

Versioning guidelines:

- When a document describes behaviour tied to a specific version of a system, say so explicitly:

  > This document describes DCIPHERED v0.8.x behaviour.
  > For v0.7.x and earlier, see `docs/archive/dciphered-operator-v0.7.md`.

- If behaviour changes in a backwards-incompatible way, either:
  - update the doc and bump its version, with a short changelog section, or
  - archive the old doc and create a new one, linking the two.

Stale docs are treated as a risk surface.
If a document is known to be outdated, it must say so at the top in the metadata band (for example, `Status: Deprecated` or `Status: Archived`).

---

## 7. Adopting SSE-Markdown in an Existing Repository

**Chosen Approach:** Incremental upgrade without rewriting history

Not every repository will start life with SSE in place.
That is fine. The goal is not to rewrite the past but to make the **current** and **future** documentation steadily more predictable and trustworthy.

A practical upgrade path:

1. **Pick one primary doc** (usually the top-level `README.md`) and add an SSE metadata band at the top.
2. **Tighten the README** to match this profile: clear summary, security/safety notes where needed, and links to code/config.
3. **Identify critical docs** (operator runbooks, install guides, threat models) and apply the same pattern.
4. When touching any other Markdown files during normal work, **opportunistically upgrade** them: add metadata, fix tone, annotate risk.
5. Over time, introduce a simple checklist or CI lint step that warns about missing metadata bands or obvious anti-patterns.

This keeps the migration aligned with normal development instead of becoming a giant “docs rewrite” project that never quite finishes.

---

## 8. AI-Targeted Documentation and Prompts

**Chosen Approach:** Make the etiquette machine-legible

Some Markdown documents will exist primarily to steer AI tools that help generate or maintain Synavera code.
These documents are effectively part of the **control plane** for AI-generated changes and must be treated as such.

For AI-facing docs:

- Clearly label them in the metadata:

  ```markdown
  Audience: AI-assisted tooling, Developers
  Scope: How to apply SSE-Rust v1.1 in this repository.
  ```

- Use consistent terminology from the SSE universe: `Security / Safety Notes`, `append-only logs`, `SSE-JSON`, `SSE-YAML`, etc.
- Prefer imperative, unambiguous instructions over vague “style suggestions”.
- Include one or more explicit **prompt exemplars** showing how you expect AI tools to be invoked, for example:

  ```markdown
  Example prompt:

  > Apply Synavera Script Etiquette — Rust Profile v1.1 to this module.
  > Add Security / Safety Notes where the code touches the filesystem or network.
  > Keep comments narrative and explain intent, not syntax.
  ```

- If certain areas of the code are particularly sensitive (cryptography, key handling, network boundary code), call that out and impose stricter rules.

Remember: AI-facing docs are the instructions you are handing to a very fast, very literal assistant.
If the instructions are fuzzy, the results will be fuzzy.
If the instructions are precise and aligned with SSE, the generated code is far more likely to match your ethos.

---

## 9. Relationship to Universal SSE and Language Profiles

**Chosen Approach:** Docs obey the same constitution

This profile inherits all universal principles from Synavera Script Etiquette v1.2.
In particular:

- Clarity beats cleverness.
- No hidden behaviour or surprise persistence.
- Ethics and user agency are non-negotiable constraints.
- Code and docs should be written as if a stranger will audit them.

Markdown & Documentation v1.2 also depends on:

- SSE-JSON v1.2, for `_meta` patterns and configuration clarity.
- SSE-YAML v1.2, for human-edited control surfaces and orchestration semantics.
- Language-specific profiles (Rust, Python, Bash, C/C++, etc.) for how described behaviour is implemented in code.

In any conflict between:

- this Markdown profile and the universal SSE, the universal etiquette prevails;
- this Markdown profile and a language profile, treat it as a documentation bug and resolve the inconsistency in favour of accuracy and user agency.

Documentation is considered incomplete if it describes behaviour that is not reflected in code or config, or if code and config materially differ from the documented risk posture.
In such cases, fixing the mismatch is part of the work, not an optional extra.

---

## 10. Minimal SSE-Compliant README Example

A stripped-down skeleton for a new Synavera project’s README, using **Syn-Syu** as an example. This shows how the metadata band, tone, risk notes, and linking conventions come together in practice.

```markdown
<!--
Title: Syn-Syu — Project README
Version: 0.1
Status: Draft
SSE Profile: Markdown & Documentation v1.2
Audience: Developers, Power Users, Curious Troubleshooters
Scope: High-level overview and safe local use of Syn-Syu.
Last Reviewed: 2025-11-27 (UTC)
Security / Safety: Contains commands that clone and run local scripts; no system-wide changes occur unless explicitly requested.
Linked Artifacts:
  - Code: . (SSE-Bash v1.1 / SSE-Rust v1.1, as applicable)
  - Config: config/syn-syu.yaml (SSE-YAML v1.1, if used)
-->

# Syn-Syu — Project README

Syn-Syu is a Synavera-flavoured helper for managing packages and related tooling
on Arch-based systems. This document explains what Syn-Syu does at a high level
and how to run it safely on a development machine.

## 1. What Syn-Syu Does (High-Level)

- Provides scripted helpers for common Synavera package workflows.
- Aims to make operations explicit, reversible, and well-logged.
- Avoids hidden side effects or surprise system-wide changes.

> **Security Note:** Syn-Syu may interact with your package manager when you
> explicitly ask it to. Review commands before confirming actions that modify
> installed packages or system configuration.

## 2. Cloning the Repository (Developer Machine)

```bash
# As your regular user:
git clone https://github.com/Synavera-Discorporated/Syn-Syu.git
cd Syn-Syu
```

> **Safety Note:** Cloning the repository only writes into your current
> directory and does not modify system-wide state.

## 3. Exploring Syn-Syu Safely

```bash
# As your regular user:
./syn-syu.sh --help
```

If your project uses a different entrypoint or binary name, document that here
instead. The first run should ideally be a **read-only** or **help** command
that lets users see what the tool can do before making changes.

For configuration details (if applicable), see:

- `config/syn-syu.yaml` (SSE-YAML v1.1)
- `docs/syn-syu-config.md` (this profile)
```

This skeleton can be copied and adapted for each new project, with the specifics
of Security / Safety Notes, commands, and links adjusted to match reality.
Over time, a consistent pattern of SSE-compliant READMEs makes it much easier
for newcomers, auditors, and future-you to understand how each Synavera project
relates to the others.

---

## 11. Final Notes

Markdown under Synavera is where the system speaks in full sentences.
It is expected to be:

- honest about risk,
- explicit about scope and audience,
- and tightly linked to the code and configuration it describes.

Well-written documentation is not a luxury feature; it is part of how Synavera
keeps its promise of user sovereignty and transparent intent.

When in doubt, assume a future you, or a stranger, will read this doc at 3AM in the middle of a problem.
Write for that person. Give them the clarity and context you wish someone had given you.

#============================================================
*End of Synavera Script Etiquette — Markdown & Documentation Profile v1.2*

