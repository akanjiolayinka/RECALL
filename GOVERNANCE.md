# Governance

Recall is small, so its governance is simple. This page says who decides
what, so nobody has to guess.

## The maintainer

Recall was created by Olayinka Akanji, who is currently its only
maintainer. The maintainer:

- decides what goes into Recall and what doesn't, and sets the direction in
  [ROADMAP.md](ROADMAP.md)
- reviews and merges pull requests
- makes releases
- handles security reports and Code of Conduct reports

Some decisions are fixed by the project's purpose and aren't up for debate
in a single pull request. Recall stays local: no cloud services, no API
keys, no accounts, no telemetry, and no network calls from the app. Changes
that weaken that won't be merged, however useful they look.

## How decisions get made

Most decisions happen in the open, in issues, discussions and pull request
reviews. If you want to change something big, like a new file type, a new
model or a different storage format, open an issue or a discussion first
and describe the problem you're solving. Agreeing on the approach before
anyone writes the code saves a lot of wasted effort.

When people disagree, the maintainer makes the final call and explains the
reasoning. That isn't the same as ignoring feedback: if new information
comes up, decisions can be revisited.

## How pull requests get merged

A pull request is merged when:

- CI passes, including the tests, lints and the privacy audit
- the maintainer, or a regular contributor the maintainer has asked to
  review that area, has approved it
- review comments have been answered, either by a change or an explanation

Small fixes like typos can be merged quickly. Larger changes may take a few
rounds of review. [CONTRIBUTING.md](CONTRIBUTING.md) explains what reviewers
look for.

## Becoming a regular contributor

There's no application form. People who make several good contributions,
help others in issues and discussions, and show good judgement about the
project's privacy rules may be invited to help with triage and reviews in
the areas they know. That can grow into write access to the repository, and
eventually into co-maintaining.

## Changes to this document

If the project grows, this page will change with it, for example to add
more maintainers. Changes go through a pull request like everything else.
