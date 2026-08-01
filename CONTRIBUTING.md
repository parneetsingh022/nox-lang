# Contributing

Thank you for your interest in contributing to **nyx**.

Contributions of all sizes are welcome, including bug fixes, tests, documentation, diagnostics, parser improvements, language-design discussions, and developer tooling.

## Before You Start

For substantial changes, open an issue or discussion before writing a large patch. This helps avoid duplicated work and gives maintainers a chance to confirm the intended direction.

Small fixes, test improvements, documentation updates, and clearly scoped refactors may be submitted directly as pull requests.

## Development Setup

### Requirements

Install:

- A recent stable Rust toolchain
- `rustfmt`
- Clippy
- Git

Install the required Rust components with:

```bash
rustup component add rustfmt clippy
```

Clone the repository and enter the project directory:

```bash
git clone https://github.com/parneetsingh022/nyx-lang.git
cd nyx-lang
```

Build the project:

```bash
cargo build
```

Run the test suite:

```bash
cargo test
```

## Project Structure

The main directories are:

```text
src/        Compiler and language implementation
tests/      Integration tests and snapshots
```

Keep implementation code in the most specific existing module. Avoid adding new modules or abstractions unless they make the code easier to understand or maintain.

## Making Changes

Create a focused branch:

```bash
git switch -c <type>/<short-description>
```

Examples:

```text
feat/let-statements
fix/unclosed-parenthesis-diagnostic
test/identifier-spans
docs/parser-architecture
refactor/statement-dispatch
```

Keep each pull request focused on one logical change. Unrelated cleanup should usually be submitted separately.

## Code Style

Follow standard Rust conventions and the existing style of the repository.

Before submitting a change, run:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

Prefer:

- Clear names over short or clever names
- Small, focused functions
- Explicit error handling
- Helpful diagnostics
- Comments that explain why, not what the code already says
- Existing abstractions over unnecessary new ones
- Tests that describe behavior rather than implementation details

Avoid:

- Unrelated formatting or refactoring
- Panics for ordinary user input
- Suppressing warnings without a clear reason
- Duplicated sources of truth
- Large abstractions added for hypothetical future needs

## Tests

Changes to parsing, lexing, diagnostics, spans, or AST behavior should include tests.

Useful test categories include:

- Successful parsing
- Invalid or incomplete input
- Operator precedence
- Source spans
- Diagnostic variants
- Error recovery
- Snapshot output
- Regression tests for fixed bugs

Prefer small tests with one clear purpose. Use parameterized tests when several cases exercise the same behavior.

When adding or updating snapshots, review the generated output carefully before committing it.

## Documentation

Public items should have documentation when their purpose is not immediately obvious.

Documentation should explain:

- What the item represents
- Why it exists
- Important invariants
- How spans or symbols are used
- Any non-obvious ownership or parsing behavior

Update relevant README or documentation files when changing public behavior, syntax, commands, or project setup.

## Commit Messages

Use concise, imperative commit messages.

Recommended format:

```text
<type>(<scope>): <summary>
```

Examples:

```text
feat(parser): add let statement parsing
fix(lexer): report unterminated block comments
test(parser): cover identifier spans
refactor(ast): add spanned declaration identifiers
docs: add contribution guidelines
```

Common types include:

```text
feat
fix
test
refactor
docs
chore
ci
perf
```

## Pull Requests

A pull request should:

- Explain what changed
- Explain why the change is needed
- Mention important design decisions
- Include tests where appropriate
- Pass formatting, linting, and tests
- Avoid unrelated changes

A useful pull request description includes:

```text
## Summary

## Motivation

## Testing

## Notes
```

Draft pull requests are welcome for early feedback.

## Reporting Bugs

When reporting a bug, include:

- The nyx source input
- The expected behavior
- The actual behavior
- Relevant diagnostics or terminal output
- The operating system
- The Rust version
- The commit or release being tested

Use a minimal reproducible example whenever possible.

## Proposing Language Changes

Language-design changes should include:

- The problem being solved
- Example syntax
- Expected semantics
- Invalid cases
- Interaction with existing syntax
- Parser and diagnostic implications
- Alternatives considered

Do not assume that adding syntax is automatically beneficial. Simplicity, consistency, diagnostics, and implementation cost should all be considered.


## AI-Assisted Contributions

AI tools may be used to assist with contributions, but contributors remain fully responsible for everything they submit.

Acceptable uses include:

- Brainstorming test cases
- Explaining unfamiliar code
- Drafting documentation
- Suggesting refactors
- Generating small amounts of boilerplate
- Reviewing a patch for possible issues

AI-generated output must not be submitted without careful human review.

Before submitting AI-assisted work, contributors must:

- Understand the code and documentation they are submitting
- Verify that the change is correct
- Run all relevant tests
- Check for fabricated APIs, incorrect assumptions, and unnecessary abstractions
- Ensure the contribution matches the repository's style and design
- Confirm that no confidential, private, licensed, or proprietary material was provided to the AI tool
- Confirm that generated content does not copy incompatible licensed material

AI tools must not be used to:

- Submit code the contributor cannot explain
- Generate large unreviewed patches
- Bypass testing or code review
- Fabricate test results, benchmarks, issue references, or citations
- Upload secrets, credentials, private reports, or unpublished vulnerabilities
- Produce low-effort spam issues or pull requests
- Impersonate another contributor or maintainer

Contributors should disclose meaningful AI assistance when it materially influenced the submitted implementation, design, or documentation.

A brief note in the pull request description is sufficient:

```text
AI assistance: Used to brainstorm tests and review documentation. All output was manually reviewed and verified.
```

Disclosure is generally unnecessary for minor assistance such as spelling, grammar, or basic command lookup.

Maintainers may request additional explanation, tests, or revisions for AI-assisted contributions. A contribution may be rejected if the author cannot explain or maintain it.

## Security Issues

Do not report security vulnerabilities in public issues.

Follow the instructions in [SECURITY.md](SECURITY.md) and use the private reporting method described there.

## Code of Conduct

All contributors must follow the project [Code of Conduct](CODE_OF_CONDUCT.md).

## License

By contributing, you agree that your contributions will be licensed under the same license as the project.

