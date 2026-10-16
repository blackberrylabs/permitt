# Contributing

Thank you for looking. Permitt is small and deliberately narrow — it reads, shows and
explains; it never changes a configuration. Contributions that keep it that way are
welcome.

## Adding or fixing a host

A host is one file in `hosts/`, read by every binary. Copy the closest existing rule and
fill in each location, each setting, the defaults and the precedence **with their
source**: the vendor's documentation page or the source file that gives the path, the key,
the meaning of its values, the date you read it, and the words you read it from. If you only know a path because you saw it on your machine, say so with
`kind = "observed"` — the output will tell users as much. A rule without a source is
refused at load time, on purpose. The schema is in [`docs/hosts.md`](docs/hosts.md).

Then add the host to the fake home in `crates/core/tests/fixtures/home/` (use `__HOME__`
for the home directory; it is replaced when the tests copy the fixture) and adjust the
expected states in `crates/core/tests/inventory.rs`. `cargo test` must stay green, and the
table in `docs/hosts.md` is kept in step with the rules by hand.

The bypass flags Permitt looks for in shell files are the closed list in
`hosts/_shell.toml`; a new flag needs the documentation sentence that describes it.

## What will not be merged

- Anything that writes to, moves or deletes a host's configuration file, even with a
  confirmation.
- Anything that opens a network connection: no update check, no telemetry, no remote
  probe.
- A path, key, value meaning or default "from memory": each claim needs its source.
- A score, a grade or a colour that judges: Permitt states facts, the user judges.
- Showing a secret in clear anywhere, including in debug output.

## Running

```bash
cargo build --release
cargo test --workspace
./target/release/permitt --home crates/core/tests/fixtures/home        # fixture paths are placeholders; the tests copy it with real ones
packaging/build.sh deb                                                   # needs cargo-deb
```

`cargo fmt --all` and `cargo clippy --workspace --all-targets` before a pull request.
Code, comments, commit messages and documentation are in English; the project's own
brief, plan and journal (`BRIEF.md`, `PLAN.md`, `JOURNAL.md`) are in French.

## Reporting

Open an issue with the output of `permitt files` and `permitt --json findings` —
secrets are masked by construction, but read it once before pasting.
