# gh-commit-ledger

A TUI that displays the commit count of your public repositories. Written in Rust.

## Features

- Aggregates yesterday's commit count and total commit count for all repositories.
- Copy to clipboard.

## Example Scenarios

- 20 commits added.
    - For instance, after submitting a few issues to GitHub Copilot Coding Agent, you might see about 20 commits added overnight.

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

### Running

```
gh-commit-ledger
```

### Updating

```
gh-commit-ledger update
```

## Usage

When launched, aggregation starts automatically. The first time it runs, it fetches all target repositories, which may take time depending on the number of repositories.

## Configuration

If a configuration file is not found on startup, `config.toml` will be created with default values.

On Windows, it is saved to `%LOCALAPPDATA%\gh-commit-ledger\config.toml`. The actual path is displayed in the TUI at the bottom under `config:`.

The configuration directory can be overridden with the `GH_COMMIT_LEDGER_CONFIG_DIR` environment variable. In this case, `config.toml` and `cache.json` will be saved in the specified directory.

Pressing the `e` key opens the configuration file in an editor. The editors listed in `editors` will be tried in order. By default, the order is `fresh`, `zed`, `code`, `edit`, `nano`, `vim`.

The body message copied to the clipboard can be changed via `[clipboard].message`. In the TUI, this message is displayed in a separate frame from the existing aggregation display.

```toml
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

[clipboard]
message = "Yesterday I made {commits_yesterday} commits.\nThe total commit count to date is {total_commits}."
```

The following two placeholders are available:

| Placeholder | Description |
| --- | --- |
| `{commits_yesterday}` | Yesterday's commit count |
| `{total_commits}` | Total commit count |

## Aggregation Logic

This tool bases its calculations on "yesterday" and "the day before yesterday" according to the local timezone.

- Retrieves the cumulative commit count of each repository's default branch as of `yesterday 23:59:59`.
- Also retrieves the cumulative commit count as of `the day before yesterday 23:59:59`.
- The difference between these two values is treated as "yesterday's commit count".
- The cumulative total of all target repositories as of `yesterday 23:59:59` is summed and displayed as "total commit count".

The targets are public repositories owned by the GitHub CLI authenticated user. Forks are excluded. Repositories without a default branch or where the tip of the default branch cannot be treated as a commit are counted as `0`.

The list of repositories is obtained using GitHub CLI's `gh repo list <login> --limit 1000 --visibility public --source --no-archived=false`.

## Caching

The fetched results are saved in the same OS local configuration directory as the configuration file. The actual path is displayed in the TUI at the bottom under `cache:`.

The cache stores cumulative commit counts for each repository at different points in time. During normal startup, it uses existing cache data and fetches only the missing data from GitHub.

The cache retains up to 16 historical data points per repository, and older points are removed when the limit is exceeded.

To force a re-fetch, press `r` in the TUI.

## Detailed Specifications

- Aggregates public repositories owned by the GitHub CLI authenticated user.
- Excludes forks but includes archived repositories in the aggregation target.
- Retrieves the cumulative commit count of each repository's default branch using the GraphQL API.
- Displays yesterday's commit count, total commit count, and repositories with activity in the TUI.
- Saves fetched results to the OS's local configuration directory for reuse on subsequent runs.
- Copies the currently displayed Clipboard Message to the clipboard by pressing the `c` key.
- Creates `config.toml` in the OS's local configuration directory to customize the clipboard message.

## Troubleshooting

### `Failed to launch gh command`

GitHub CLI is either not installed or not included in your PATH. Please ensure `gh --version` can be executed.

### `gh command failed`

Check your GitHub CLI authentication, network connection, and GitHub API rate limits.

```powershell
gh auth status
gh api user
```

### Slow First Run

The first run calls the API for all target public repositories. Subsequent runs for the same date will be faster as the cache will be utilized.