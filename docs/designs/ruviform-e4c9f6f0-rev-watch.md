---
id: "DESIGN-zNicJhxcoRoEDAbP9aMrTQ"
type: "design"
status: "approved"
title: "ruviform e4c9f6f0 pin 追従の設計"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/designs/ruviform-e4c9f6f0-rev-watch.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/ruviform-e4c9f6f0-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/specs/ruviform-e4c9f6f0-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# ruviform e4c9f6f0 pin 追従の設計

## 方針

lockfile-primary 方式を維持する — manifest に `rev` を書かず、
`node tools/update-ruviform-rev.mjs <sha>` が全 `ruviform-*` package を
単一 resolved SHA に原子解決する（失敗時 lockfile 不変の fail-closed）。
`ui/vendor/ruviform/` は `mise run sync-ui-deps` の生成物で commit しない
（gitignore 済み）。

## 手順

1. `git ls-remote https://github.com/scarecrow173/ruviform.git
   refs/heads/main` で candidate を再解決 →
   `e4c9f6f0715da3ac479f8c437d21a021327e51da`（探索時点から drift なし）。
2. `node tools/update-ruviform-rev.mjs
   e4c9f6f0715da3ac479f8c437d21a021327e51da` → 内部で
   `cargo update -p ruviform-* --precise` が `Cargo.lock` を更新
   （26 package・source fragment のみの差分）。
3. `mise run sync-ui-deps` で vendor 追随（REV 一致確認・commit しない）。
4. `mise exec -- cargo fetch --locked`。
5. 検証: `check-ruviform-lock` / `check-ruviform-rev` / `check-ui` /
   `test-ui` / `verify-toolchain` + slice 共通 lane 全て。

## 互換性判断

- `b6d0cc6a..e4c9f6f0` の AL 依存木内差分は additive:
  - `ruviform-audio-test`: `RunMetadata::for_environment` は新規
    `#[must_use]` constructor の追加のみ。既存 `deterministic` の
    シグネチャ・serde 形状は不変。新規 dev-dep
    （audio-device/midi-device）は dependency の dev-dep であり
    consumer の lockfile/依存解決に入らない — `Cargo.lock` 差分が
    SHA fragment 26 行のみであることを実測確認。
  - `ruviform-plugin-core`: `GuiSize` は新規 `Copy` 型。`Plugin` trait
    の 3 method は全て default 実装（`HostError::Unsupported`）付きで
    既存実装を壊さない。`ErasedPlugin` の新規 method は crate 内の
    唯一の実装 `Erased<P>` で forwarding 済み — AL は両 trait を
    実装・直接参照しない。
  - `ruviform-plugin-host`: `HostCommand` enum への variant 追加と
    `InProcessHost` dispatch・`EditorSessionOps` の
    `set_size`/`set_visibility`/`reattach` 実装化。旧実装は常に
    `Unsupported` を返す refusal だったため、verb を呼ばない
    consumer への影響はない。AL は plugin-host API を未消費
    （`crates/` 内に plugin 系の参照なし — grep 実測）。
- serde/永続化形式の意味論差分なし（`RunMetadata` の serde 形状・
  `HostCommand` wire 語彙は不変 — wire encoding は Issue #115 scope で
  未追加のまま）。

## 新 API の採用/見送り

- `RunMetadata::for_environment`・os_endpoint 系: 見送り/NotApplicable
  （AL は device boundary・RunMetadata を非消費）。
- L0 editor verbs: 見送り（AL は plugin host を提供しない）。
- いずれも pin 追従で自動的に利用可能になるだけで、AL 側の消費コードは
  本 slice では追加しない（消費が必要と判明した場合は別 slice/`ask`）。

## リスク

- upstream main は今後も進む — 本 bump は `e4c9f6f0` 時点の snapshot
  であり、以降の追従は別 rev-watch で扱う。
- `e4c9f6f0` 自体に upstream covering CI run がない — merge 後の
  AL 側対象 SHA の CI 証拠が統合検証の正本（Coordinator 確認）。
- `scarecrow173/audio-lens#189`（legacy WebView 撤去）が将来
  root `Cargo.toml`/`Cargo.lock` に触れ得る — 同時並行時の
  rebase/merge 順序は Coordinator が調整。
