---
name: cerbo-mcp-management
description: "Use when changing a Cerbo vault through the cerbo MCP tools: creating, editing, moving, renaming, linking and deleting pages safely with base_version, and when changes wait for approval. Examples: \"Add a page for this meeting\", \"Move these notes into a folder\""
---

# Managing the vault: changing pages, folders and links

You may change this vault. Every change goes through the daemon, which may ask the user first. Work in small, reviewable steps and say what you changed.

## The rules of safe editing

1. Read the page first with `read_page`; it returns a `version`.
2. `edit_page` replaces the whole markdown of the page. Pass the `version` as `base_version`. If the page changed since you read it, the edit is refused with a conflict and nothing is overwritten: read the page again and redo your edit on the new text.
3. Keep every line you did not mean to change, including links (`cerbo://<uuid>`) and annotations (`[Text]{prefix:Type}`). Do not rewrite a page to "tidy" it unless asked.
4. A change may wait for the user's approval, which can take minutes. Do not repeat the call while it waits. If it is rejected or times out, nothing changed: tell the user, and do not try the same change again unless they ask.

## Pages, folders and links

- `create_page`: a title, optional markdown and a folder `path` such as `projects/2026`. It returns the new UUID. Search first so you do not duplicate a page that exists.
- Folders are virtual paths, not directories: `move_page` sets a page's folder (no path means the root), and a folder exists as long as a page is in it.
- `rename_page` changes the title. Links are by UUID, so renaming never breaks them.
- `link_pages` appends a link to page `to` at the end of page `from`. To link inside a sentence, use `edit_page` and write `[words](cerbo://<uuid>)`.
- `delete_page` moves a page to the vault trash, from where the user can recover it. It is the only kind of delete you have. Check `backlinks` first and tell the user what linked to the page.

## After a change

Reading the page again with `read_page` confirms the result. Page changes are entered in the agent change log with your name, so the user can see what you did.

## Files outside the vault

The user opened access to files outside the vault. `host_list` lists a folder, `host_read` reads a text file in chunks (continue with `next_offset`), `host_write` creates or replaces a whole text file. Use absolute paths. Paths the user listed work at once, reading or writing as listed; any other path waits for the user's approval for that one call. Some places are never available (credentials, Cerbo's own files, other vaults), and files inside the vault are changed with the page tools. There is no way to delete or run anything outside the vault. Ask for the narrowest path you need, and never read more than the task needs.
