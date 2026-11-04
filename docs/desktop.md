# Desktop app

`cerbo-desktop` is a Tauri 2 app with a Svelte 5 frontend and the Carta
Markdown editor. It opens the vaults registered with `cerbo vault add` (or from
its own vault selector) and edits the same `.cerbo/` store the CLI uses.

## Layout

- **Left sidebar**: the pages of the open vault as a folder tree. A page's
  folder is its `cerbo:virtualPath` (the same path `cerbo symlink` nests it
  under); folders are not stored, one exists while a page is in it. Click a
  folder to collapse or expand it (remembered per vault), drag a page onto a
  folder or onto the root zone below the list to move it, or right-click a row
  for **Move to…**, **Rename**, **Delete** on a page and **New page here**,
  **Rename folder** on a folder. `J`/`K` move, `Tab` cycles, `Ctrl+P`
  searches by title.
- **Editor**: a write tab and a preview tab. Saves are queued and written
  atomically; a page changed on disk by another program shows a diff and asks
  before overwriting.
- **Right sidebar**: the **backlinks** of the open page and, once semantic
  search is enabled with the panel's **Enable** button, its most **similar
  notes** with a score. Enabling downloads the embedding model once (about
  490 MB) into the user cache; page text never leaves the machine.

## Mentions

`[Berlin]{schema:Place}` renders as a highlighted "Berlin" with its type in a
tooltip. An unknown prefix is underlined as unresolved. The syntax is described
in [links-and-annotations.md](links-and-annotations.md).

## SPARQL console

`Ctrl+Shift+E` opens a console for read-only queries over the vault's
knowledge graph. Page IRIs in the results show as titles and open the page.
The last query is kept between runs. The prefixes and result formats are the
same as for `cerbo sparql` ([cli.md](cli.md#query-with-sparql)).

## Window

The window has no native frame. Cerbo draws its own title bar (vault · page):
drag it to move, double-click to maximize. The only window button is close,
which asks for confirmation like `Ctrl+Q`. The window size is restored on
start; desktop settings live in `$XDG_CONFIG_HOME/cerbo/ui.toml`.

## Keyboard shortcuts

Press `F1` in the app for the full list. The main ones (`⌘` instead of `Ctrl`
on macOS):

| Keys | Action |
|------|--------|
| `Ctrl+P` | Open page search |
| `Ctrl+N` | Create a new page |
| `Ctrl+O` / `Ctrl+Shift+O` | Open vault selector / add a vault |
| `Ctrl+T` | Toggle light/dark theme |
| `Ctrl+Shift+E` | SPARQL console |
| `Ctrl+Q` | Quit (with confirmation) |
| `Alt+←` / `Alt+→` | Back / forward in history |
| `J` / `K`, `Tab` | Next / previous page, cycle pages in the sidebar |
| `j` / `k` (preview) | Scroll down / up |
| `r`, `m`, `Del` (preview) | Rename / move / delete the current page |
| `j` `k` `h` `l` (sidebar) | Next / previous row, collapse or go to parent, expand or step in |
| `Enter`, `m`, `r` (sidebar) | Open page or toggle folder, move page or rename folder, rename |
| `Esc` | Close the active dialog |

## Startup

The packaged launcher runs `cerbo migrate vocabulary --if-needed` on every
registered vault before the window opens, so vaults written by older builds are
brought to the current metadata format first. While the app runs it holds
`$XDG_CACHE_HOME/cerbo/desktop.lock`, and no CLI migration touches the vault.

## Testing

The end-to-end suite in `e2e/` drives the built app through WebDriver; see
[development.md](development.md#end-to-end).
