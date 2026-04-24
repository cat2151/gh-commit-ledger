# gh-commit-ledger

あなたの公開リポジトリの commit数 を表示するTUIです。Rustで書かれています。

## 特徴

- 昨日のcommit数と、総commit数を、全リポジトリについて集計
- クリップボードにコピー

## 参考例

- 20commit増えてた
    - GitHub Copilot Coding Agentにいくつかissueを投げて寝て起きると20commitくらい増えます

## 必要なもの

- Rust
- GitHub CLI (`gh`)
- `gh` で GitHub にログイン済みであること

GitHub CLI の状態は次のコマンドで確認できます。

```powershell
gh auth status
```

未ログインの場合は、先に認証してください。

```powershell
gh auth login
```

### インストール

```
cargo install --force --git https://github.com/cat2151/gh-commit-ledger
```

### 実行

```
gh-commit-ledger
```

### アップデート

```
gh-commit-ledger update
```

## 使い方

起動すると自動で集計が始まります。初回実行時は対象リポジトリをすべて取得するため、リポジトリ数によっては時間がかかります。

## 設定

起動時に設定ファイルがない場合は、デフォルト値で `config.toml` を作成します。

Windows では `%LOCALAPPDATA%\gh-commit-ledger\config.toml` に保存されます。実際のパスは TUI 下部の `config:` に表示されます。

設定ディレクトリは `GH_COMMIT_LEDGER_CONFIG_DIR` 環境変数で上書きできます。この場合、`config.toml` と `cache.json` は指定したディレクトリに保存されます。

`e` キーを押すと設定ファイルを editor で開きます。利用する editor は `editors` に書かれた順に試します。デフォルトでは `fresh`、`zed`、`code`、`edit`、`nano`、`vim` の順です。

`[clipboard].message` で、クリップボードへコピーする本文メッセージを変更できます。TUI ではこのメッセージが既存の集計表示とは別枠で表示されます。

```toml
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

[clipboard]
message = "昨日は {commits_yesterday} 件の commit をしました。\nこれまでの総 commit 数は {total_commits} 件です。"
```

利用できるプレースホルダーは次の 2 つです。

| プレースホルダー | 内容 |
| --- | --- |
| `{commits_yesterday}` | 昨日の commit 数 |
| `{total_commits}` | 総 commit 数 |

## 集計仕様

このツールは、ローカルタイムゾーンで見た「昨日」と「一昨日」を基準にします。

- `昨日 23:59:59` 時点の各リポジトリのデフォルトブランチ累計 commit 数を取得
- `一昨日 23:59:59` 時点の累計 commit 数も取得
- 2 つの値の差分を「昨日の commit 数」として扱う
- すべての対象リポジトリの `昨日 23:59:59` 時点の累計を合計して「総 commit 数」として表示

対象は GitHub CLI の認証ユーザーの public repository です。fork は除外されます。デフォルトブランチがないリポジトリや、デフォルトブランチの先端が commit として扱えないリポジトリは `0` として扱われます。

リポジトリ一覧は GitHub CLI の `gh repo list <login> --limit 1000 --visibility public --source --no-archived=false` で取得します。

## キャッシュ

取得結果は設定ファイルと同じ OS のローカル設定ディレクトリ配下に保存されます。実際のパスは TUI 下部の `cache:` に表示されます。

キャッシュには、リポジトリごとに時点別の累計 commit 数が保存されます。通常の起動では既存キャッシュを利用し、不足している分だけ GitHub から取得します。

キャッシュはリポジトリごとに最大 16 時点分を保持し、上限を超えると古い時点から削除します。

強制的に再取得したい場合は、TUI 上で `r` を押してください。

## 細かい仕様

- GitHub CLI の認証ユーザーが所有する public repository を集計
- fork を除外し、archived repository は集計対象に含める
- 各リポジトリのデフォルトブランチの累計 commit 数を GraphQL API で取得
- 昨日の commit 数、総 commit 数、動きがあったリポジトリを TUI で表示
- 取得結果を OS のローカル設定ディレクトリへ保存し、再実行時に再利用
- `c` キーで表示中の Clipboard Message をクリップボードへコピー
- OS のローカル設定ディレクトリに `config.toml` を作成し、コピー用メッセージをカスタマイズ

## トラブルシューティング

### `gh コマンドの起動に失敗しました`

GitHub CLI がインストールされていないか、PATH に含まれていません。`gh --version` が実行できる状態にしてください。

### `gh コマンドが失敗しました`

GitHub CLI の認証、ネットワーク接続、GitHub API の制限を確認してください。

```powershell
gh auth status
gh api user
```

### 初回実行が遅い

初回はすべての対象 public repository に対して API を呼び出します。2 回目以降はキャッシュが使われるため、同じ日付の集計は速くなります。
