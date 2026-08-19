# Clarification

<!--
This document records a settled change to the existing planning record.

Never use a clarification file to ask a question or preserve undecided
alternatives.
-->

## Trigger

The planned DejaVu 2.37 download source returned 404 and the worktree host has
no installed DejaVu files. The fixture implementation still requires two
redistribution-safe TrueType inputs available without network access during
generation.

## Decision

Use the host's Liberation Sans and Liberation Serif version 2.1.5 TrueType
files, distributed under the SIL Open Font License 1.1, as the committed
fixture inputs. Copy their exact bytes into `tests/fixtures/fonts/` and record
their source package, version, license, and checksums. This preserves the
embedded TrueType coverage while making generation offline and reproducible.

## Supersedes

The earlier decision to use DejaVu Sans and DejaVu Serif 2.37 fixture inputs is
replaced. The PDF structure, fixture scope, and runtime boundaries are
unchanged.

## Effect on the record

`01-idea.md` now names Liberation Sans and Liberation Serif 2.1.5 and the SIL
Open Font License 1.1. `02-plan.md` and `03-tasks.md` use the corresponding
filenames and provenance requirements. No converter behavior is changed.
