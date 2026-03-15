# Security Policy

## Supported Versions

Currently, security updates are focused on the latest release cycle. As the project is in its early stages (pre-1.0), we prioritize the most recent minor version.

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | :white_check_mark: |
| 0.2.x   | :x:                |
| 0.1.x   | :x:                |

## Reporting a Vulnerability

We take the security of Muxspace seriously, especially given that it handles sensitive data like browser cookies, PTY sessions, and local database persistence.

### How to report
If you find a security vulnerability, please do not open a public GitHub issue. Instead, please report it through one of the following channels:
* **Email:** Send a detailed report to the maintainers at the contact address listed on the [InSelfControll GitHub profile](https://github.com/InSelfControll/muxspace).
* **Private Disclosure:** Use the GitHub "Report a vulnerability" feature on the repository if enabled.

### What to include
To help us triage the issue quickly, please include:
* A description of the vulnerability and its potential impact.
* Steps to reproduce the issue (PoC code or clear instructions).
* Details about your environment (OS, Rust version, and Muxspace version).

### What to expect
* **Acknowledgement:** You can expect an initial acknowledgement of your report within **48 hours**.
* **Updates:** We will provide status updates at least once a week while the vulnerability is being investigated.
* **Disclosure:** Once a fix is verified, we will coordinate a public disclosure date with you. We prefer to follow "Coordinated Vulnerability Disclosure" to ensure users have time to update before details are made public.

### Scope
This policy covers the Muxspace core application code. For vulnerabilities in our key dependencies—such as `dioxus`, `portable-pty`, `webkit2gtk`, or `sled`—we encourage you to also report them to the respective upstream maintainers.
