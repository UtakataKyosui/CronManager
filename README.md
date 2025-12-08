# Cron Manager

LinuxとmacOSのためのスケジュールタスク管理TUI（Terminal User Interface）アプリケーション。Cronエントリを視覚的に管理し、簡単に追加・編集・削除できます。

**OS自動判定機能**: LinuxではCron、macOSではLaunchdを自動的に選択して使用します。

## 特徴

- **OS自動判定**: LinuxではCron、macOSではLaunchdを自動選択
- **表形式UI**: 名前、スケジュール、コマンドを見やすい表形式で表示
- **簡単な編集**: キーボードショートカットで直感的に操作
- **有効/無効の切り替え**: エントリを削除せずに一時的に無効化可能
- **安全な管理**: ローカルファイルで管理（オプションでシステムスケジューラも使用可能）
- **名前付き管理**: 各エントリに分かりやすい名前を付けて管理

## インストール

### 前提条件

- Rust（1.70以上）

### ビルド

```bash
# リポジトリのクローン
git clone <repository-url>
cd CronManager

# リリースビルド
cargo build --release

# バイナリは target/release/cron-manager に生成されます
```

### インストール（オプション）

```bash
# システムにインストール
cargo install --path .
```

## 使い方

### 基本的な起動

```bash
# デフォルト: システムスケジューラを直接編集
# Linux: crontab、macOS: launchd
cargo run

# または、ビルド済みバイナリを使用
./target/release/cron-manager
```

### ローカルファイルモード

システムスケジューラに影響を与えずにテストする場合：

```bash
# ローカルファイルモード（~/.cron-manager-crontab を使用）
./target/release/cron-manager --local
```

**注意**:
- デフォルトモードでは実際のシステムスケジューラが変更されます
- **Linux**: システムのcrontabが更新されます
- **macOS**: `~/Library/LaunchAgents/` にplistファイルが作成・管理されます

## 操作方法

### ナビゲーション

- `↑` / `k`: 上に移動
- `↓` / `j`: 下に移動

### エントリの管理

- `a`: 新しいエントリを追加
  1. 名前を入力してEnter
  2. Cronスケジュール（例: `0 2 * * *`）を入力してEnter
  3. 実行コマンドを入力してEnter
- `d`: 選択中のエントリを削除
- `Space`: エントリの有効/無効を切り替え

### エントリの編集

- `n`: 選択中のエントリの名前を編集
- `s`: 選択中のエントリのスケジュールを編集
- `c`: 選択中のエントリのコマンドを編集

### その他

- `Enter`: 入力を確定
- `Esc`: 入力をキャンセル
- `q`: アプリケーションを終了

## Cronスケジュールの書式

Cronスケジュールは5つのフィールドからなります：

```
分 時 日 月 曜日
```

### 例

- `0 2 * * *`: 毎日午前2時
- `*/15 * * * *`: 15分ごと
- `0 9 * * 1-5`: 平日の午前9時
- `0 0 1 * *`: 毎月1日の午前0時
- `30 3 * * 0`: 毎週日曜日の午前3時30分

### フィールドの値

- **分**: 0-59
- **時**: 0-23
- **日**: 1-31
- **月**: 1-12
- **曜日**: 0-7（0と7は日曜日）

特殊文字：
- `*`: 全ての値
- `/`: 間隔（例: `*/5` = 5ごと）
- `-`: 範囲（例: `1-5` = 1から5まで）
- `,`: リスト（例: `1,3,5` = 1,3,5）

## ファイル形式

Cron Managerは、各エントリに名前を付けるために特別なコメント形式を使用します：

```bash
# NAME: Daily Backup
0 2 * * * /path/to/backup.sh

# NAME: Hourly Check
0 * * * * /path/to/check.sh

# 無効化されたエントリ（コメントアウト）
# NAME: Disabled Job
# 0 3 * * * /path/to/disabled.sh
```

## macOSでの動作

macOSでは、Cronの代わりにLaunchdを使用します：

- **自動変換**: Cron式を自動的にLaunchdのCalendarIntervalに変換
- **Plist生成**: `~/Library/LaunchAgents/com.cronmanager.*.plist` ファイルを自動生成
- **無効化**: エントリを無効にするとplistファイルが削除され、launchctlからアンロードされます
- **ログ**: 各ジョブのログは `/tmp/com.cronmanager.*.stdout` と `/tmp/com.cronmanager.*.stderr` に保存されます

## プロジェクト構造

```
CronManager/
├── src/
│   ├── main.rs           # エントリーポイント、イベントループ
│   ├── app.rs            # アプリケーション状態管理
│   ├── cron_entry.rs     # Cronエントリのデータ構造
│   ├── cron_parser.rs    # Crontab解析ロジック
│   ├── storage.rs        # ストレージ抽象化レイヤー
│   ├── scheduler/        # スケジューラバックエンド
│   │   ├── mod.rs        # スケジューラトレイト定義
│   │   ├── file.rs       # ローカルファイルバックエンド
│   │   ├── cron.rs       # Cronバックエンド（Linux/Unix）
│   │   └── launchd.rs    # Launchdバックエンド（macOS）
│   └── ui.rs             # TUI描画ロジック
├── Cargo.toml            # 依存関係設定
└── README.md             # このファイル
```

## 技術スタック

- **Rust**: プログラミング言語
- **Ratatui**: TUIフレームワーク
- **Crossterm**: ターミナル操作
- **Cron**: Cron式の解析
- **Serde**: シリアライゼーション

## アーキテクチャ

このアプリケーションは、プラットフォーム間の違いを抽象化する設計になっています：

1. **Schedulerトレイト**: 異なるスケジューラバックエンド（Cron、Launchd、ファイル）を統一的に扱うためのトレイト
2. **OS自動判定**: コンパイル時に`target_os`を使用してプラットフォームを判定し、適切なバックエンドを選択
3. **Storage抽象化**: ユーザーコードはスケジューラの実装詳細を意識せず、統一されたAPIで操作
4. **再利用可能なコンポーネント**: `CronEntry`や`CronParser`は他のプロジェクトでも使用可能

## ライセンス

MIT License

## 貢献

バグ報告や機能要望は、GitHubのIssueでお願いします。
