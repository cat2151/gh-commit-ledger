# gh-commit-ledger

A TUI that displays the number of commits in your public repositories, written in Rust.

## Features

- Aggregates yesterday's commit count and total commit count across all repositories.
- Copy to clipboard.

## Guideline

- Submitting a few issues to GitHub Copilot Coding Agent and checking back the next day can result in approximately 20 more commits.

## Requirements

- Rust
- GitHub CLI (`gh`)
- Logged in to GitHub with `gh`

You can check the status of GitHub CLI with the following command:

```powershell
gh auth status
```

If you are not logged in, please authenticate first:

```powersershell
gh auth login
```

### Installation

```
cargo install --force --git https://github.com/cat2151/gh-commit-ledger
```

### Running

```
gh-commit-ledger
```

### Update

```
gh-commit-ledger update
```

## Usage

Aggregation starts automatically upon launch. The first time you run it, it fetches all target repositories, which may take some time depending on the number of repositories.

## Configuration

If no configuration file exists upon launch, `config.toml` will be created with default values.

On Windows, it is saved to `%LOCALAPPDATA%\gh-commit-ledger\config.toml`. The actual path is displayed in the TUI at the bottom, next to `config:`.

The configuration directory can be overridden using the `GH_COMMIT_LEDGER_CONFIG_DIR` environment variable. In this case, `config.toml` and `cache.json` will be saved to the specified directory.

Pressing the `e` key opens the configuration file in an editor. The editors listed in `editors` will be tried in order. By default, the order is `fresh`, `zed`, `code`, `edit`, `nano`, `vim`.

You can change the clipboard message body to be copied via `[clipboard].message`. In the TUI, this message is displayed in a separate section from the existing aggregated view.

```toml
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

[clipboard]
message = "Yesterday, I made {commits_yesterday} commits.\nMy total commit count so far is {total_commits}."
```

The following two placeholders are available:

| Placeholder | Description           |
| ----------- | --------------------- |
| `{commits_yesterday}` | Yesterday's commit count |
| `{total_commits}`   | Total commit count    |

## Aggregation Logic

This tool uses 'yesterday' and 'the day before yesterday' based on the local timezone.

- Retrieves the cumulative commit count for the default branch of each repository as of `yesterday 23:59:59`.
- Also retrieves the cumulative commit count as of `the day before yesterday 23:59:59`.
- The difference between these two values is treated as 'yesterday's commit count'.
- The sum of cumulative commit counts for all target repositories as of `yesterday 23:59:59` is displayed as the 'total commit count'.

The targets are public repositories owned by the authenticated GitHub CLI user. Forks are excluded. Repositories without a default branch, or where the tip of the default branch cannot be treated as a commit, are handled as `0`.

The list of repositories is obtained using GitHub CLI's `gh repo list <login> --limit 1000 --visibility public --source --no-archived=false`.

## Cache

The retrieval results are saved in the same OS local configuration directory as the settings file. The actual path is displayed in the TUI at the bottom, next to `cache:`.

The cache stores cumulative commit counts per repository for different points in time. Normal startup uses existing cache, fetching only the missing data from GitHub.

The cache retains up to 16 points in time per repository, deleting older points when the limit is exceeded.

To force re-fetching, press `r` in the TUI.

## Detailed Features

- Aggregates public repositories owned by the authenticated GitHub CLI user.
- Excludes forks, includes archived repositories in the aggregation target.
- Retrieves the cumulative commit count for the default branch of each repository using the GraphQL API.
- Displays yesterday's commit count, total commit count, and repositories with activity in the TUI.
- Saves retrieval results to the OS local configuration directory for reuse on subsequent runs.
- Pressing `c` copies the displayed Clipboard Message to the clipboard.
- Creates `config.toml` in the OS local configuration directory and allows customization of the copy message.

## Troubleshooting

### `Failed to launch gh command`

GitHub CLI is either not installed or not included in your PATH. Ensure that `gh --version` can be executed.

### `gh command failed`

Check your GitHub CLI authentication, network connection, and GitHub API limits.

```powershell
gh auth status
gh api user
```

### Slow initial run

The first run calls the API for all target public repositories. Subsequent runs use the cache, making aggregation for the same date faster.