# Security Policy

## Supported Versions

Currently, `julesctl` is in active development. Security updates are applied only to the latest version on the `main` branch.

| Version | Supported          |
| ------- | ------------------ |
| `main`  | :white_check_mark: |
| Older   | :x:                |

## Reporting a Vulnerability

We take the security of `julesctl`, and by extension the security of your local repositories and AI credentials, very seriously.

If you discover any security vulnerabilities, please **do not** publicly disclose them (e.g., via GitHub Issues or public Discord channels).

Instead, please responsibly report them via email or directly to the repository owners.

### What to Report

We are particularly interested in vulnerabilities that relate to:
1.  **Credential Leaks:** Any mechanism where the OS Keyring integration is bypassed or API keys are written to plaintext files or logs.
2.  **State Contamination:** Any vulnerability that allows the `~/.config/julesctl/` directory to be manipulated maliciously, or allows the Ahenk P2P sync mechanism to expose local state to unauthorized peers.
3.  **Command Injection:** As `julesctl` relies on executing shell commands (`git fetch`, `git push`, `$EDITOR`), any vectors allowing an attacker to inject arbitrary commands via branch names, commit messages, or Jules AI API payloads are critical.

### Response Time

We aim to respond to all vulnerability reports within 48 hours. If the vulnerability is accepted, we will coordinate with you on publishing a fix and providing a CVE/credit if applicable.
