# Roadmap

This is what's unfinished in Recall and where it could go next. There are
no dates: Recall is a spare-time project, and things get done when someone
picks them up. If one of these interests you, comment on the related issue
or open a discussion before starting, especially for the bigger items.

Every item here has to fit Recall's rules: it runs locally, needs no
network, account or API key, and any AI model has at most 500 million
parameters. Some good ideas don't fit those rules, and that's fine.

For smaller, self-contained tasks with pointers into the code, see
[docs/GOOD_FIRST_ISSUES.md](docs/GOOD_FIRST_ISSUES.md).

## Testing and packaging

These come first, because they decide whether people can actually use
Recall.

- **Published releases.** Installers only exist today as CI artifacts that
  expire after 14 days and need a GitHub login. A release workflow that
  builds installers for a tagged version and attaches them to a GitHub
  release would make Recall installable by anyone.
- **macOS.** The macOS build has never been tried. Someone needs to run it,
  fix what breaks, and add a macOS job to CI.
- **Linux packages.** The `.deb` was tested once, offline, before meaning
  search was added. CI should build and keep `.deb` and AppImage packages,
  and they need testing on a few distributions.
- **Windows offline test.** CI checks the indexing and search code with
  networking switched off, but nobody has yet used the installed Windows app
  with Wi-Fi off since meaning search was added.
- **Code signing.** Unsigned installers trigger SmartScreen and Gatekeeper
  warnings. Signing needs certificates and some cost, so it's a decision for
  the maintainer rather than a contribution.

## Search quality

- **Real-world tuning of meaning search.** The thresholds come from 24
  synthetic documents and 34 searches. A larger, more varied evaluation set
  (longer documents, different writing styles, more near-misses), and
  feedback from people using Recall on real libraries, would show whether
  they hold up.
- **Other languages.** The embedding model and the keyword stemmer are
  English-only. Supporting other languages means choosing and verifying a
  multilingual embedding model that fits the parameter limit, letting people
  pick it, and handling keyword search for languages the English stemmer
  doesn't suit.
- **Unicode-aware file-name matching.** Matching ignores case for ASCII
  letters only, so "École" and "école" don't match.

## Reading more files

- **OCR for scanned PDFs.** Today a PDF with no text layer is detected and
  reported, but not read. Doing this properly means turning PDF pages into
  images, which is hard to do in pure Rust, so the first step is choosing an
  approach that doesn't add a heavy native dependency.
- **PDFs that are partly scanned.** Pages without text in an otherwise
  normal PDF are skipped silently today. At minimum Recall should say so.
- **More file types.** Candidates include OpenDocument text (.odt), rich
  text (.rtf), and plain-text formats like .csv or .log. Each one needs
  extraction code, tests and a sample file in `test-data/`.

## Performance

- **Large folders.** When a file changes, Recall re-walks the whole folder
  it belongs to, though it only re-reads changed files. For folders with
  hundreds of thousands of files, scanning just the changed paths would be
  much cheaper.
- **Bigger libraries.** Meaning search compares the query with every stored
  passage. That's fast enough for tens of thousands of passages (about
  56 ms for 50,000), but very large libraries may eventually need an index.

## Interface

- **Accessibility and keyboard use.** Moving through results with the arrow
  keys, clearer focus handling in dialogs, and a check with a screen reader.
- **First-run guidance.** A short welcome that explains what Recall does
  with your files and helps you add a first folder.
- **Theme choice.** Recall follows the system's light or dark setting; some
  people will want to choose.

## Exploring

- **Ask Recall.** An optional, fully offline mode where a small local
  language model answers questions using your documents. It would have to
  quote the passages it relies on, so answers can be checked the same way
  search results can, and stay within the 500-million-parameter limit. This
  is an open question, not a promise: a model that makes things up would go
  against what Recall stands for.

## Project health

- **More tests.** The frontend has unit tests for its helper functions but
  none for components, and some backend modules are lightly tested.
- **CI upkeep.** Updating GitHub Actions versions as old ones are
  deprecated, and keeping CI quick as the project grows.
