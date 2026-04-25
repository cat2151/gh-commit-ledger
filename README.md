# gh-commit-ledger

A TUI that displays the commit count of your public repositories. Written in Rust.

## Features

- Aggregates yesterday's commit count and total commit count across all repositories
- Copy to clipboard

## Examples

- Gained 20 commits
    - If you submit a few issues to GitHub Copilot Coding Agent, go to sleep, and wake up, you might find about 20 more commits.
- Sometimes 1 commit and 20 commits can have the same value. We recommend operating with the mindset that "there were commits yesterday," without getting too fixated on the numbers.

## Requirements

- Rust
- GitHub CLI (`gh`)
- Logged in to GitHub with `gh`

You can check the status of GitHub CLI with the following command:

```powershell
gh auth status
```

If you are not logged in, please authenticate first.

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

If a configuration file does not exist when launched, `config.toml` will be created with default values.

On Windows, it is saved at `%LOCALAPPDATA%\gh-commit-ledger\config.toml`. The actual path is displayed in the TUI at `config:`.

The configuration directory can be overridden with the `GH_COMMIT_LEDGER_CONFIG_DIR` environment variable. In this case, `config.toml` and `cache.json` will be saved in the specified directory.

Pressing the `e` key opens the configuration file in an editor. The editors listed in `editors` will be tried in order. By default, the order is `fresh`, `zed`, `code`, `edit`, `nano`, `vim`.

You can change the message body copied to the clipboard using `[clipboard].message`. In the TUI, this message is displayed in a separate frame from the existing aggregation display.

```toml
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

[clipboard]
message = "昨日は {commits_yesterday} 件の commit をしました。\nこれまでの総 commit 数は {total_commits} 件です。"
```

The following two placeholders are available:

| Placeholder | Content |
| --- | --- |
| `{commits_yesterday}` | Yesterday's commit count |
| `{total_commits}` | Total commit count |

## Aggregation Specifications

This tool uses "yesterday" and "the day before yesterday" as perceived in the local timezone as its basis.

- Fetches the cumulative commit count of each repository's default branch as of `yesterday 23:59:59`.
- Also fetches the cumulative commit count as of `the day before yesterday 23:59:59`.
- Treats the difference between these two values as "yesterday's commit count".
- Sums the cumulative counts of all target repositories as of `yesterday 23:59:59` and displays it as the "total commit count".

Targets are public repositories of the authenticated GitHub CLI user. Forks are excluded. Repositories without a default branch or where the tip of the default branch cannot be treated as a commit are treated as `0`.

The list of repositories is obtained using `gh repo list <login> --limit 1000 --visibility public --source --no-archived=false` from GitHub CLI.

## Cache

The fetched results are saved under the OS's local configuration directory, same as the settings file. The actual path is displayed in the TUI at `cache:`.

The cache stores the cumulative commit count for each repository at different points in time. During normal launch, it utilizes existing cache and fetches only the missing data from GitHub.

The cache retains up to 16 data points per repository, deleting older points when the limit is exceeded.

To force a re-fetch, press `r` in the TUI.

## Detailed Specifications

- Aggregates public repositories owned by the authenticated GitHub CLI user.
- Excludes forks and includes archived repositories in the aggregation.
- Fetches the cumulative commit count of each repository's default branch using the GraphQL API.
- Displays yesterday's commit count, total commit count, and repositories with activity in the TUI.
- Saves the fetched results to the OS's local configuration directory for reuse on subsequent runs.
- Pressing `c` copies the displayed Clipboard Message to the clipboard.
- Creates `config.toml` in the OS's local configuration directory to customize the copy message.

## Troubleshooting

### `Failed to launch gh command`

GitHub CLI is either not installed or not included in your PATH. Please ensure that `gh --version` can be executed.

### `gh command failed`

Please check GitHub CLI authentication, network connection, and GitHub API limits.

```powershell
gh auth status
gh api user
```

### Slow initial run

The first run makes API calls for all target public repositories. Subsequent runs use the cache, so aggregation for the same date will be faster.