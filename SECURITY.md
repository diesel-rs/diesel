# Security Policy for Diesel

## Supported Versions

The Diesel team provides only support for the latest released version at Diesel at any point in time. Users are strongly encouraged to upgrade to the latest version for the best security posture.

## Reporting a Vulnerability

We take the security of Diesel very seriously. If you believe you've found a security vulnerability, we encourage you to inform us responsibly through coordinated disclosure.

### How to Report

**Do not report security vulnerabilities through public GitHub issues, discussions, or social media.**

Instead, please use one of these secure channels:

1. **GitHub Security Advisories** (preferred): Use the "Report a vulnerability" button in the Security tab
2. **Email** (backup): Send details to `github@weiznich.de`

### What to Include

To help us understand and address the issue quickly, please include:

**Required Information:**
- Brief description of the vulnerability type (6 or less sentences)
- Affected version(s) and components
- Proof-of-concept code to reproduce the issue

**Helpful Additional Details:**
- Full paths of affected source files
- Suggested mitigation or fix (if you have ideas)
- A path resolving the reported problem

Your initial report should be rather brief. For the case that additional details might be required for confirming the reported problem or to asses the severity of the reported issue the Diesel team will provide detailed followup questions.

### Our Response Process

**What We'll Do:**
1. Acknowledge your report and assign a tracking ID
2. Assess the vulnerability and determine severity
3. Develop and test a fix
4. Coordinate disclosure timeline with you
5. Release security update and publish advisory
6. Credit you in our security advisory (if desired)

## Scope

This security policy applies to:

**In Scope:**
- The Diesel Query builder 
- Any Diesel procedural macro
- The Diesel command line tool

**Out of Scope:**
- Any general Rust/Cargo security issue
- Any issue occurring in a third party crate that cannot be triggered through Diesel code

