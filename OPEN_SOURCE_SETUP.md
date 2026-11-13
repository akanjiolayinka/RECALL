# Open source setup: steps for the maintainer

Things that can only be done by hand on GitHub. Tick them off, then delete
this file before merging `docs/open-source-setup`.

## Fill in the placeholders

- [x] Put a contact email in `CODE_OF_CONDUCT.md`.
- [x] Put a security contact email in `SECURITY.md`.
- [ ] Check you're happy with the promises made on your behalf:
      acknowledging security reports "within a week" (`SECURITY.md`), and
      the merge rules in `GOVERNANCE.md`.

## Repository settings

- [ ] **Default branch.** It's currently `claude/happy-cori-i8kn5t`. Rename
      it to `main` (Settings → General → Default branch, pencil icon).
      GitHub redirects the old name, but update any bookmarks.
- [ ] **Description and website.** On the repository page, click the gear
      next to "About". Suggested description: "Search the files on your
      computer by meaning. Local AI, no cloud, no API keys."
- [ ] **Topics** (same dialog): `desktop-app`, `tauri`, `rust`, `react`,
      `typescript`, `semantic-search`, `local-first`, `privacy`, `ocr`,
      `sqlite`, `embeddings`.
- [ ] **Discussions.** Settings → General → Features → tick Discussions.
      The issue forms and SUPPORT.md send questions there.
- [ ] **Private vulnerability reporting.** Settings → Security → "Private
      vulnerability reporting" → Enable. SECURITY.md depends on it.
- [ ] **Labels.** Create the labels in `.github/labels.yml` (Issues →
      Labels → New label), or sync them with a tool such as
      `github-label-sync`. The issue forms use `bug` and `enhancement`.

## Branch protection

Once the default branch is `main`: Settings → Branches (or Rules →
Rulesets) → add a rule for `main` that:

- [ ] requires a pull request before merging
- [ ] requires the status checks **Tests and privacy audit** and
      **Windows installer** to pass
- [ ] blocks force pushes and deletion

Do the history clean-up below *before* turning on the force-push block, or
allow yourself to bypass it.

## History clean-up (agreed for the end)

The first 23 commits list "Claude" as the author. Rewriting them is a
force-push to the default branch, so do it before others fork the
repository. The commands from earlier in our conversation still apply.

## First release

- [ ] Push this branch, let CI run, and check the real-model tests pass
      (the calibration set's wording changed, see OPEN_SOURCE_PROGRESS.md).
- [ ] Do the Windows tests that are still open: meaning search in the
      installed app, and using it with Wi-Fi off.
- [ ] Move the "Unreleased" notes in `CHANGELOG.md` under a version heading,
      e.g. `## [0.1.0] - YYYY-MM-DD`.
- [ ] Tag it: `git tag v0.1.0 && git push origin v0.1.0`.
- [ ] Create a GitHub release from the tag (Releases → Draft a new
      release), paste the changelog section, and attach the installers from
      that commit's CI run. They're unsigned, so say so in the notes.
- [ ] Update the README's "Getting started" section to point at the
      release instead of CI artifacts.

## After merging

- [ ] Delete `OPEN_SOURCE_PROGRESS.md` and this file.
- [ ] Open a few issues from `docs/GOOD_FIRST_ISSUES.md` and label them
      `good first issue`, so newcomers have something to pick up.
