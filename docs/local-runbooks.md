# Local Markdown runbooks

Click **New Runbook** in the toolbar (or run **New local runbook** from the
command palette) to create a templated `untitled-runbook.md` in the first
configured folder and open it in the notebook viewer. Existing files are never
overwritten; further runbooks become `untitled-runbook-2.md` and so on.

Open the command palette and choose **Add local runbook folder**. Choose a folder
containing `.md` or `.markdown` documents, then choose **Browse local runbooks**.
Repeat Add to include more folders. Subfolders are included and paths distinguish
files with the same name. Opening a result displays the existing Markdown
notebook viewer. Browsing and opening documents never runs their commands.

A fenced block tagged `sh`, `shell`, `bash`, `zsh`, `fish`, `pwsh`, `powershell`
or `ps1` gets a **Run in terminal** button (Ctrl+Enter when the block is
selected). It copies the command into the active terminal's input. Nothing
executes until you press Enter there.

````markdown
# Inspect the working tree

Check changes before deploying:

```sh
 git status --short
```
````

Existing Markdown files stay in their original folders.
The viewer's source/edit control opens the same file in the editor.

Folders are stored in your local `settings.toml`:

```toml
[runbooks]
folders = ["~/Runbooks", "/absolute/path/to/project/operations"]
```

Edit this list to remove or reorder folders. Relative paths other than `~/` are
ignored. Folder contents are refreshed on search; missing folders are skipped.
Hidden subdirectories and nested symbolic links are skipped. Scans are bounded
to 20,000 directory entries and 16 levels; search shows up to 250 matches. For
large repositories, choose the documentation subfolder instead of the whole tree.

This feature does not enable Warp Drive, YAML workflows, accounts, or cloud sync.
