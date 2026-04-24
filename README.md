# gh-commit-ledger

A TUI that displays the commit count of your public repositories. Written in Rust.

## Features
- Aggregates yesterday's commit count and total commit count across all repositories.
- Copy to clipboard.

## Requirements

- Rust
- GitHub CLI (`gh`)
- Authenticated with GitHub via `gh`

You can check the status of GitHub CLI with the following command:

```powershell
gh auth status
```

If not logged in, please authenticate first:

```powershell
gh auth login
```

### Installation

```
cargo install --force --git https://github.com/cat2151/gh-commit-ledger
```

### Execution

```
gh-commit-ledger
```

### Update

```
gh-commit-ledger update
```

## Usage

Aggregation starts automatically upon launch. The first run will fetch all target repositories, which may take some time depending on the number of repositories.

## Configuration

If a configuration file is not found on startup, `config.toml` will be created with default values.

On Windows, it is saved to `%LOCALAPPDATA%\gh-commit-ledger\config.toml`. The actual path is displayed in the TUI at `config:`.

The configuration directory can be overridden with the `GH_COMMIT_LEDGER_CONFIG_DIR` environment variable. In this case, `config.toml` and `cache.json` will be saved to the specified directory.

Pressing the `e` key opens the configuration file in an editor. The editors listed in `editors` will be tried in order. By default, the order is `fresh`, `zed`, `code`, `edit`, `nano`, `vim`.

You can change the message copied to the clipboard with `[clipboard].message`. In the TUI, this message is displayed in a separate frame from the existing aggregation display.

```toml
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

[clipboard]
message = "昨日は {commits_yesterday} 件の commit をしました。\nこれまでの総 commit 数は {total_commits} 件です。"
```

The following two placeholders are available:

| Placeholder           | Description             |
| ---                   | ---                     |
| `{commits_yesterday}` | Yesterday's commit count |
| `{total_commits}`     | Total commit count      |

## Aggregation Specification

This tool uses 'yesterday' and 'the day before yesterday' based on the local timezone.

- Retrieves the cumulative commit count of each repository's default branch as of `yesterday 23:59:59`.
- Also retrieves the cumulative commit count as of `the day before yesterday 23:59:59`.
- The difference between these two values is treated as 'yesterday's commit count'.
- The sum of the cumulative commit counts for all target repositories as of `yesterday 23:59:59` is displayed as the 'total commit count'.

The targets are public repositories owned by the authenticated GitHub CLI user. Forks are excluded. Repositories without a default branch or where the tip of the default branch cannot be treated as a commit will be counted as `0`.

The list of repositories is retrieved using GitHub CLI's `gh repo list <login> --limit 1000 --visibility public --source --no-archived=false`.

## Caching

The retrieved results are saved in the same OS local configuration directory as the settings file. The actual path is displayed in the TUI at `cache:`.

The cache stores cumulative commit counts for each repository at different points in time. During normal startup, existing cache entries are used, and only missing data is fetched from GitHub.

The cache holds up to 16 time points per repository, and older entries are removed when the limit is exceeded.

If you want to force a re-fetch, press `r` in the TUI.

## Detailed Specifications

- Aggregates public repositories owned by the authenticated GitHub CLI user.
- Excludes forks, but includes archived repositories in the aggregation.
- Retrieves the cumulative commit count of each repository's default branch using the GraphQL API.
- Displays yesterday's commit count, total commit count, and repositories with activity in the TUI.
- Saves retrieved results to the OS local configuration directory for reuse on subsequent runs.
- Pressing `c` copies the displayed Clipboard Message to the clipboard.
- Creates `config.toml` in the OS local configuration directory to customize the clipboard message.

## Troubleshooting

### Failed to launch gh command

GitHub CLI is either not installed or not included in your PATH. Please ensure `gh --version` can be executed.

### gh command failed

Check your GitHub CLI authentication, network connection, and GitHub API rate limits.

```powershell
gh auth status
gh api user
```

### First run is slow

The first run calls the API for all target public repositories. Subsequent runs use the cache, so aggregation for the same date will be faster.