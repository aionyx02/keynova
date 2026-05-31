---
type: reference
status: active
priority: p1
updated: 2026-05-31
context_policy: on_demand
owner: project
---

# License and Compliance

## Project License

Keynova project code is released under `AGPL-3.0-or-later`.

- Human-readable name: GNU Affero General Public License version 3 or any later version
- SPDX identifier: `AGPL-3.0-or-later`
- Canonical project license file: `/LICENSE`

This applies to **Keynova-owned code in this repository**. Third-party dependencies keep their own licenses.

## What Recipients May Do

Under AGPL, recipients may:

- use the software for any purpose, including commercial use
- study the source code
- modify the software
- redistribute original or modified copies
- charge money for distribution, support, hosting, or related services

AGPL is a strong copyleft license, not a non-commercial license.

## Core Obligations

If someone conveys or redistributes Keynova or a modified version, they must at minimum:

- keep copyright and license notices intact
- provide a copy of the AGPL license text
- provide the corresponding source code for the covered work
- keep the covered work under AGPL-compatible terms required by the license
- mark modified versions clearly, including that changes were made and the relevant date

If an interactive UI is modified and distributed, AGPL/GPL family rules also expect the modified work to preserve the required legal notices where applicable.

## Network / SaaS Rule

AGPL is stricter than GPL because it adds a network-use condition.

If someone modifies Keynova and runs that modified version so users interact with it remotely over a network, the operator must offer those remote users access to the **Corresponding Source** of that modified version at no charge through a standard way of copying software.

Practical interpretation for this project:

- shipping a modified desktop app triggers ordinary copyleft distribution obligations
- turning a modified version into a hosted remote service can also trigger the AGPL network-source obligation
- for any hosted deployment of a modified version, the safest practice is to provide an obvious source link in the UI or service footer

## Private / Internal Use

Purely private use is different from redistribution.

- Running Keynova privately without giving copies to others does not by itself trigger normal distribution obligations.
- However, AGPL's network clause matters once a **modified** version is used for remote interaction with outside users.

If a company only uses an unmodified internal copy inside its own organization, the practical compliance burden is much lower than public redistribution. If the deployment becomes customer-facing, re-check AGPL section 13.

## Commercial Use

Commercial use is allowed.

Examples that are allowed in principle:

- selling support, setup, training, or consulting
- charging for packaged distribution
- offering paid hosting or managed operation
- using Keynova inside a paid product workflow

What AGPL blocks is taking the covered code proprietary after modification and refusing to provide the required source to recipients or relevant remote users.

## Third-Party Licenses

Keynova does **not** become "AGPL-only across every byte in the distribution".

Important boundary:

- your project code can be AGPL
- bundled third-party components remain under their own licenses
- the final distribution is therefore a multi-license distribution

Current dependency mix is mostly permissive (`MIT`, `Apache-2.0`, `BSD`, `ISC`), with some `MPL-2.0` and data/documentation-related licenses.

Compliance implications:

- keep third-party license texts and notices when required
- do not claim that all bundled dependencies are relicensed to AGPL
- if you ship installers, archives, or source bundles, include third-party notices alongside the project license

## Google and Other Service Terms

Open-source licensing and hosted API terms are separate layers.

For example:

- Keynova code may be AGPL
- Google Cloud Translation API is still governed by Google's service terms
- OpenAI-compatible, Anthropic, Tavily, or other providers remain governed by their own API or platform contracts

So AGPL does **not** give downstream users any right to your third-party API keys, paid accounts, quota, or hosted credentials.

## API Keys and Secrets

This repository now treats secrets as local configuration, not redistributable project assets.

Current policy in code:

- API keys are stored locally in user config
- `/setting` masks sensitive values in normal reads
- workflow history no longer stores secret values in clear text

Operational guidance:

- never commit real API keys into the repository
- never publish release builds with hard-coded provider credentials
- if users need a provider account, require them to enter their own key locally

## Warranty and Liability

AGPL, like GPL, is distributed without warranty unless someone separately offers one.

Operationally, this means:

- the default legal posture is "as-is"
- if you offer commercial support or special warranties, make clear those promises come from you, not automatically from all contributors

## Contributions

At the moment, git history indicates a single author/contributor identity in the repository. That means relicensing your own project code is operationally straightforward.

Going forward:

- treat pull requests as contributions under the repository's current license unless you establish a different contributor policy
- if you later want dual licensing, a CLA, or custom commercial exceptions, set that policy before accepting broad outside contributions

## Recommended Repository Hygiene

To keep the project legally tidy:

1. Keep the root `LICENSE` file current.
2. Keep `package.json` and `src-tauri/Cargo.toml` license metadata aligned.
3. Include third-party notices in release artifacts.
4. Do not distribute provider credentials.
5. When adding new source files, prefer an SPDX header such as:

```text
SPDX-License-Identifier: AGPL-3.0-or-later
```

6. If you later add a remotely hosted edition, add a visible source-code link to satisfy the AGPL network requirement conservatively.

## Practical Release Checklist

Before publishing a release:

1. Verify the project license file matches repository metadata.
2. Verify no secrets are committed.
3. Verify release notes or docs do not misstate third-party licenses.
4. Bundle or reference third-party license notices.
5. If the release is a hosted modified service, verify a source-code offer is visible to remote users.

## Not Legal Advice

This document is an engineering compliance guide, not formal legal advice. For revenue-sharing, embedded OEM redistribution, enterprise SaaS, or custom exceptions, ask qualified counsel to review the exact distribution model.
