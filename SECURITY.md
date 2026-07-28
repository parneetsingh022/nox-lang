# Security Policy

## Supported Versions

This project is under active development. Security fixes are generally applied only to the latest version available on the default branch and to the most recent tagged release, when applicable.

| Version | Supported |
| --- | --- |
| Latest release | Yes |
| Default branch | Yes |
| Older releases | No |

Because the project may not yet have stable releases, support guarantees can change as the project evolves.

## Reporting a Vulnerability

Please do not report suspected security vulnerabilities through a public issue, pull request, discussion, or other public channel.

Instead, report the vulnerability privately to the project maintainer using a private contact method listed on the maintainer's GitHub profile. A dedicated security email address may be added later.

Include as much of the following information as possible:

- A clear description of the vulnerability.
- The affected version, commit, or branch.
- Steps needed to reproduce the issue.
- A minimal proof of concept, when safe to provide.
- The possible impact.
- Any known mitigations or workarounds.
- Whether the issue has been disclosed elsewhere.

Do not include real credentials, private data, or destructive payloads in the report.

## What to Expect

After receiving a report, the maintainer will aim to:

1. Confirm receipt of the report.
2. Review and attempt to reproduce the issue.
3. Assess its severity and affected versions.
4. Develop and test a fix when appropriate.
5. Coordinate disclosure and release timing with the reporter.
6. Publish an advisory or release note when users need to take action.

Response times are not guaranteed, especially while the project by individual contributor.

## Disclosure Guidelines

Please allow reasonable time for the issue to be investigated and fixed before making it public.

The project may request coordinated disclosure so that a patch can be prepared before technical details are widely shared. Credit will be given to reporters who wish to be acknowledged, unless legal, privacy, or safety concerns prevent it.

## Out of Scope

The following are generally not considered security vulnerabilities unless they create a concrete security impact:

- Crashes caused only by invalid local input during normal development.
- Denial-of-service reports without a realistic attack scenario.
- Problems that require modifying the victim's local files or build environment first.
- Vulnerabilities in unsupported versions.
- Social-engineering attacks unrelated to the project.
- Reports generated entirely by automated scanners without reproducible evidence.

Since this project is a language implementation, malformed source files that cause excessive memory use, hangs, or unsafe behavior may be considered security-relevant when the compiler or tooling is expected to process untrusted input.

## Safe Harbor

Security research performed in good faith is welcome. The project will not pursue action against researchers who:

- Avoid privacy violations, data destruction, and service disruption.
- Access only the minimum data needed to demonstrate the issue.
- Report findings privately and allow reasonable time for remediation.
- Follow applicable laws and platform policies.
