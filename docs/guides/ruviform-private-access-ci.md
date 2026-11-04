---
id: "GUID-01M67AACCJX4TSV21GS0FCT8S7"
type: "guide"
status: "active"
title: "Ruviform private 依存への CI read-only アクセス設定手順"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/ruviform-dependency-ci.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-dependency-ci.md"
    - "https://github.com/scarecrow173/audio-lens/issues/99"
links:
  self: "./ruviform-private-access-ci.md"
---

# Ruviform private 依存への CI read-only アクセス設定手順

AudioLens の `Cargo.toml` は `scarecrow173/ruviform` の crate を
git 依存で参照する(`docs/designs/ruviform-dependency-ci.md` §4)。
`.cargo/config.toml` の `net.git-fetch-with-cli = true` により
cargo の fetch は system git 経由になるため、ローカルは Git
Credential Manager の既存 HTTPS credential がそのまま使える。

CI(`.github/workflows/ci.yml`)は Issue #142 で dual-mode 化され、
`ruviform-access` job が run ごとにアクセス経路を解決する:

- repo が **public** の間は**匿名 HTTPS** が主経路
  (最小権限 = 秘密を持たない。credential 設定は不要)
- repo が **private** に戻った場合は、本書の **read-only
  Deploy Key** が fallback 経路になる(`RUVIFORM_DEPLOY_KEY`
  secret が設定済みなら自動で SSH 経路を使う)
- 両方不可なら lane は明示失敗する(緑扱いしない)

本書は private 時の Deploy Key 一次設定・ローカル検証・運用
手順をまとめる。

## 前提

- `scarecrow173/ruviform` への admin 権限(Deploy key 登録)
- `scarecrow173/audio-lens` への admin 権限(Actions secret 登録)
- `gh` CLI が両 repo に対して認証済みであること

## 一次設定(運用者が 1 回だけ実施)

### 1. 専用鍵ペアの生成

```bash
ssh-keygen -t ed25519 -C "audio-lens-ci-ruviform-read" \
  -f ruviform_deploy_key -N ""
```

- passphrase は空(CI 専用鍵。他用途と分離するため流用しない)
- comment には用途名のみを入れる。repo URL・owner 名は含めない

### 2. ruviform に Deploy Key を read-only で登録

```bash
gh api repos/scarecrow173/ruviform/keys \
  -f title="audio-lens-ci-read" \
  -f key="$(cat ruviform_deploy_key.pub)" \
  -F read_only=true
```

登録結果を確認し、`read_only: true` になっていることを見る:

```bash
gh api repos/scarecrow173/ruviform/keys --jq '.[] | {title, read_only}'
```

### 3. audio-lens に秘密鍵を secret 登録

```bash
gh secret set RUVIFORM_DEPLOY_KEY \
  --repo scarecrow173/audio-lens < ruviform_deploy_key
```

- secret 名は workflow の `secrets.RUVIFORM_DEPLOY_KEY` と一致
  させる
- 登録後、**ローカルの鍵ファイルは安全に破棄する**
  (`rm` ではなく `srm`/暗号化領域等、環境の規約に従う)

### 4. 動作確認

`.github/workflows/ci.yml` は Issue #142 で `pull_request` /
main `push` / `workflow_dispatch` の自動起動済み。Deploy Key を
設定した場合は、次回の run の `ruviform-access` job が
`mode=deploy_key` を出力し、各 lane の
`Configure read-only access` → `Probe private ruviform access` →
`Fetch locked dependencies` が SSH 経路で通ることを確認する。

secret 未設定かつ ruviform が public の間は `mode=anonymous` で
認証なしの HTTPS fetch になる。ruviform が private で secret も
未設定なら、各 lane の gate step が `::error::` で明示失敗する
(not-run 扱いの緑にはしない)。

## workflow 側の仕組み(`.github/workflows/ci.yml`)

以下は `mode=deploy_key` 時の経路。`mode=anonymous` では認証
設定自体を skip し、cargo の git fetch は匿名 HTTPS で行う
(追加 credential なし)。

third-party の SSH/鍵管理 action は使わない。秘密情報を外部
action に渡す供給網上の信頼辺を増やさないため。

1. `secrets.RUVIFORM_DEPLOY_KEY` を env 経由でのみ受け取り、
   `RUNNER_TEMP` 配下に mode 600 の鍵ファイルとして展開する
   (echo・artifact 化しない)
2. GitHub が公開している SSH host key
   (`https://api.github.com/meta` の `ssh_keys`。認証不要)を
   `known_hosts` に `github.com` 付きで書き出し、
   `StrictHostKeyChecking=yes` で照合する(TOFU ではなく公開値と
   突合する)
3. `GIT_SSH_COMMAND` に
   `ssh -i <key> -o IdentitiesOnly=yes -o UserKnownHostsFile=<kh>
   -o StrictHostKeyChecking=yes` を `GITHUB_ENV` 経由で後続 step
   に設定する
4. `git config --global
   url."git@github.com:scarecrow173/ruviform.git".insteadOf
   "https://github.com/scarecrow173/ruviform.git"` で、
   Cargo.toml の HTTPS URL のみを SSH に書き換える(他の
   github.com アクセスには影響しない)
5. `git ls-remote git@github.com:scarecrow173/ruviform.git HEAD`
   の probe step で、認証失敗と manifest/lock 失敗を切り分ける

`cargo fetch --locked` step では `GIT_LFS_SKIP_SMUDGE=1` を
設定する。ruviform の LFS 実体(fixture 等)は crate のコンパイルに
不要であり、smudge を止めることで private repo 上の LFS 認証にも
依存しない。

## ローカルでの手順検証

CI step と同じ手順をローカルの一時ディレクトリで再現できる
(鍵は秘密情報のため既存の Deploy Key を使うか、テスト用に新規
発行して検証後に破棄・失効させる):

```bash
tmp=$(mktemp -d)
key="$tmp/ruviform_deploy_key"
kh="$tmp/known_hosts"
# (検証用鍵が手元にある前提。本番 secret の内容をローカルに
#  置かないこと)
curl -fsSL https://api.github.com/meta \
  | jq -r '.ssh_keys[] | "github.com " + .' > "$kh"
test -s "$kh"
GIT_SSH_COMMAND="ssh -i \"$key\" -o IdentitiesOnly=yes \
  -o UserKnownHostsFile=\"$kh\" -o StrictHostKeyChecking=yes" \
  git ls-remote git@github.com:scarecrow173/ruviform.git HEAD
```

HEAD の SHA が `Cargo.toml` に pin した rev と一致することを
確認する。

## 認証情報のローテーション / 失効

- 鍵が漏洩した・不要になった場合は ruviform の Deploy keys から
  公開鍵を削除するだけで失効する
  (`gh api repos/scarecrow173/ruviform/keys` で id を取得し
  `gh api -X DELETE repos/scarecrow173/ruviform/keys/<id>`)
- audio-lens 側の `RUVIFORM_DEPLOY_KEY` secret も併せて削除する
- 新鍵への更新は §1-§3 と同じ手順で差し替える

## 代替 credential(Deploy Key が使えない環境向け)

**fine-grained PAT**(owner 個人または bot アカウントで発行、
Resource owner = scarecrow173、Repository access =
`scarecrow173/ruviform` のみ、Permissions = Contents: Read):

- secret 名を `RUVIFORM_PAT` として登録し、workflow の認証
  step を次に置き換える:

  ```bash
  git config --global \
    url."https://x-access-token:${RUVIFORM_PAT}@github.com/scarecrow173/ruviform.git".insteadOf \
    "https://github.com/scarecrow173/ruviform.git"
  ```

- SSH 設定(known_hosts・GIT_SSH_COMMAND)は不要になる
- PAT は有効期限の管理が必須。発行は Web UI 手動のため自動化
  しにくい。URL に token を埋め込む形になるため、insteadOf 対象を
  ruviform 単体に限定し、ログへ remote URL を出す step を追加
  しないこと

## トラブルシュート

| 症状 | 原因候補 | 対処 |
| --- | --- | --- |
| `RUVIFORM_DEPLOY_KEY secret is not set` | secret 未登録・repo 違い | §3 を確認。Environment secret ではなく repository secret かを見る |
| probe step で `Permission denied (publickey)` | Deploy Key 未登録・別鍵を参照 | §2 の登録確認。`IdentitiesOnly=yes` で指定鍵のみ使われるため鍵 path を確認 |
| probe step で `Host key verification failed` | known_hosts 生成失敗 | `api.github.com/meta` 応答と `test -s` を確認。GitHub の host key ローテーション時は meta 再取得で追従する |
| `cargo fetch` で `authentication required` | insteadOf 未適用・LFS smudge | `git config --global --get-regexp insteadOf` を debug step で確認(値に秘密は含まれない)。`GIT_LFS_SKIP_SMUDGE` の env 設定を確認 |
| Windows runner のみ `Permissions for ... are too open` | ssh が Win32-OpenSSH で鍵 ACL を厳格判定 | 手元の msys ssh を指すよう PATH を確認。再発する場合は `-o StrictModes=no` 追加を検討(別 Issue で判断) |
