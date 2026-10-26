# univer-licensing

`Type: research`
`Status: resolved`

## Question

What are the redistribution/licensing obligations of shipping or invoking univer-cli from Alex? From primary sources (univer-cli LICENSE + README "Data and security" + package manifests; Univer SDK/Pro license pages): Apache-2.0 covers what; what the bundled localhost runtime development credential is, its 90-day rotation and redistribution authorization; which `@univerjs-pro` components have separate terms; whether npx-time installation (not bundling) changes the obligations; and what must be disclosed to users (network calls: update checks, explicit HTTP imports).

## Answer
Branch `research/univer-licensing`, file `research/univer-cli/02-univer-licensing.md`.

SAFE_WITH_DISCLOSURE: invoke the official package unmodified via npx at runtime (not a bundled copy — 105 of 123 `@univerjs-pro/*` packages have no license text; rebranding falls outside the localhost runtime credential, which hard-expires 2026-11-15 and rotates every 90 days). Disclose network surfaces: update checks to registry.npmjs.org (opt-out exists), HTTP imports, resources downloads, Chrome provisioning, `npx skills add` fetch, Node >=24. Feeds ticket 04 (bootstrap must stay npx-vendor-unmodified).
