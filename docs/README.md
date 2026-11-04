# Cerbo documentation

Start with the [project README](../README.md) for what Cerbo is and how to
build it. These pages go deeper.

| Page | What it covers |
|------|----------------|
| [development.md](development.md) | Dev environment, building, running the test suites, releases, repository layout |
| [cli.md](cli.md) | The `cerbo` command line: vaults, pages, backlinks, SPARQL, search, indexing, symlinks, imports |
| [desktop.md](desktop.md) | The desktop app: panels, mentions, SPARQL console, window, shortcuts |
| [links-and-annotations.md](links-and-annotations.md) | `cerbo://` links and HackMD-style `[Text]{prefix:Type}` annotations |
| [storage.md](storage.md) | On-disk layout: the vault's `.cerbo/` store, the symlink projection, file formats, the user cache and config, older vault formats |

The full command reference is the man page, written in
[cli/man/cerbo.md](../cli/man/cerbo.md) and installed as `man cerbo` with the
CLI.

Design history lives in [`openspec/`](../openspec/) (one change per feature,
archived once shipped) and in [archive/](archive/) for older reviews.
