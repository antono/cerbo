<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Cerbo icon" width="128" height="128">
</p>

<h1 align="center">Cerbo</h1>

<p align="center">
  A local-first Markdown wiki with a knowledge graph underneath.<br>
  <em>Cerbo</em> (/ˈt͡serbo/) is Esperanto for "brain".
</p>

Cerbo keeps your notes as plain files you own, gives every page a permanent
UUID so links never break, and indexes the result as an RDF graph you can
query with SPARQL or search by meaning. It ships as a desktop app and as a
`cerbo` command-line tool that share the same vault format.

## Features

- **Local-first, plain files.** Every note is a Markdown file in a `.cerbo/`
  directory you can commit, sync and grep. No cloud, no database lock-in.
- **Stable links.** Pages link with `cerbo://<uuid>`; renaming never breaks a
  link. Backlinks come from a local index, not from files in the vault.
- **Semantic annotations.** `[Bob]{schema:Person}` marks what a page mentions,
  using schema.org and any ontology you import.
- **SPARQL.** Read-only queries over the vault's knowledge graph, from the CLI
  or the desktop console.
- **Search by meaning.** `cerbo search` and `cerbo similar` find related notes
  across languages with a local embedding model; text never leaves your machine.
- **Desktop and CLI.** A Tauri + Svelte editor with backlinks, similar notes
  and keyboard-driven navigation, and a scriptable CLI with `--json` output.
- **Multiple vaults.** Isolated knowledge bases anywhere on disk, found
  Git-style from the current directory.

## Quick start

```bash
nix develop                        # dev shell with Rust, Bun, Tauri (or `direnv allow`)
nix build .#cerbo-desktop          # desktop app  -> result/bin/cerbo-desktop
nix build .#cerbo                  # CLI          -> result/bin/cerbo
```

```bash
cerbo init                         # turn the current directory into a vault
cerbo page create "My first page"  # prints the page's UUID
cerbo sparql 'SELECT ?t WHERE { ?p schema:name ?t }'
```

Prebuilt Linux releases are on the
[Releases](https://github.com/antono/cerbo/releases) page.

## Documentation

| | |
|---|---|
| [docs/development.md](docs/development.md) | Dev environment, building, tests, releases, repository layout |
| [docs/cli.md](docs/cli.md) | The `cerbo` command line, with the full reference in [`man cerbo`](cli/man/cerbo.md) |
| [docs/desktop.md](docs/desktop.md) | The desktop app: panels, SPARQL console, window, shortcuts |
| [docs/links-and-annotations.md](docs/links-and-annotations.md) | `cerbo://` links and `[Text]{prefix:Type}` annotations |
| [docs/storage.md](docs/storage.md) | What is on disk: the vault store, file formats, cache, config, older formats |

The index of all pages is [docs/README.md](docs/README.md). Feature designs
live in [openspec/](openspec/).

## Tech stack

Rust workspace (`core` library, `cli`, `src-tauri` desktop) ·
[Svelte 5](https://svelte.dev/) + [Carta](https://github.com/BearToCode/carta) +
Tailwind CSS · [Bun](https://bun.sh/) · [Nix Flakes](https://nixos.org/explore/#nix-flakes) ·
[Oxigraph](https://github.com/oxigraph/oxigraph) for the RDF store.

## License

LGPL-3.0-or-later
