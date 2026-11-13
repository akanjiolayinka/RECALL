# Security policy

Recall's whole promise is that your files never leave your computer. A bug
that breaks that promise is the most serious kind of bug this project can
have, so please report security problems privately and give us a chance to
fix them before they're public.

## How to report

Use GitHub's private vulnerability reporting: go to the
[Security tab](https://github.com/akanjiolayinka/RECALL/security) and click
**Report a vulnerability**. Only the maintainer can see the report. If that
doesn't work for you, email
akanjiolayinka01@gmail.com.

Please don't open a public issue, discussion or pull request for a security
problem.

A useful report says what you found, how to reproduce it (the smaller the
example, the better), which version or commit you tested, and your OS. If
you have an idea for a fix, include it, but you don't need one to report.

## What happens next

Recall is maintained by one person in their spare time, so there's no
formal response-time promise. You can expect an acknowledgement within a
week, and updates as the fix progresses. Once a fix is out, we'll credit you
in the changelog unless you'd rather stay anonymous.

## What's in scope

Anything that could make data leave the machine is treated as **high
severity**, however unlikely it seems. That includes:

- any code path that sends file contents, file names, search queries or
  index data over the network
- a way for a file being indexed (a crafted PDF, Word document or image) to
  make the app run code, read files outside the folders you chose, or reach
  the internet
- ways around the window's Content Security Policy or Tauri's permissions
- a dependency that phones home or adds telemetry
- the model download script accepting a file whose checksum doesn't match

Other bugs in scope include crashes or hangs caused by a malicious file,
writes to your files (Recall should only ever read them), and problems in
the installers.

Out of scope: the security of your own computer and OS, and attacks that
need someone to already control your user account. The CI-only developer
script `scripts/bge_reference.py`, which downloads a model to produce test
data, is also out of scope unless it affects what ships in the app.

## Supported versions

There are no releases yet. Security fixes go into the default branch and
will be in the next release.
