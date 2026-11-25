# rf#1014 evidence — audio-lens ポリシー反映

対象 Issue: `scarecrow173/ruviform#1014`  
関連 Issue: `scarecrow173/audio-lens#274`  
実施日: 2026-11-25  
base SHA: `7b3be0d9a735620a922e09492129364ae959f31a`  
最終検証対象 SHA: `1e5bbf9787d8b531a652aef6890bace1fa491666`（ポリシー・skill・mise・参照更新のコミット。本ファイルは後続の evidence コミットに入るため、最終 PR head SHA は PR 本文に記録する）

## 環境

- OS: Windows 11（10.0.26300.9974）、x64
- mise: 2026.11.2 windows-x64
- prek: 0.5.7 (8569a4ac5 2026-10-21)
- canonical source: `gh issue view 1014 -R scarecrow173/ruviform --json body --jq .body` で取得した rf#1014 本文

## 実走した検証

| 検証 | コマンド | OS | exit code | 結果 |
| --- | --- | --- | --- | --- |
| fmt lane | `mise run fmt` | Windows 11 | 0 | Pass。`cargo fmt --all -- --check` が完了した |
| pre-commit 軽量チェック | `mise exec -- prek run --all-files` | Windows 11 | 0 | Pass。trailing-whitespace、end-of-file、mixed line ending、UTF-8 BOM、YAML、TOML、private key、biome、workflow 検査がすべて Pass または対象外 Skip |
| formal proptest lane | `mise run test-formal` | Windows 11 | 未実施 | NotRun。文書・ポリシーと実行意味論を変えない description/comment 同期だけの変更であり、POLICIES.md §5 の軽量チェック原則に従った。任意確認としての実走は省略した |
| Workspace 全体 Build/Test | `check-full`/`lint-full`/`test-full`/`build-full`/`verify-full` | Windows 11 | 未実施 | NotRun。文書・ポリシーのみの変更であり、POLICIES.md §5 で必須ではない |

## 手動整合性レビュー

- [x] `POLICIES.md` §6 の転記照合。rf#1014 本文の規範 6 節から、共通原則 10 件と各規範節の本文を構成する 23 行を抽出し、すべてが `POLICIES.md` に完全一致で存在することをスクリプトで確認した（missing 0）。§6 内に canonical 本文以外の規則文を追加していない。見出しと節番号の構造化のみを行った。
- [x] `AGENTS.md` と `.agents/skills/verify-audio-lens/SKILL.md` の有効な案内に、proptest → Kani → TLA+ → Lean を必須の昇格路として扱う記述が残っていない。grep では skill の否定文 `rather than as the source of a fixed tool promotion path` のみが見つかり、利用者へ旧昇格路を送る記述ではないことを確認した。
- [x] Timeout、OOM、環境不在、exit 0、CI green を Proof Pass と扱う記述が `AGENTS.md` と skill に残っていない。grep で見つかったのは `POLICIES.md` §6.5 の禁止規則だけだった。
- [x] 未移行の個別検証が `scarecrow173/audio-lens#274` に明示的に紐付けられている。`POLICIES.md` §7 と skill の Formal verification 節の両方に、現行 WAV/MP4/AVI proof 群が未移行であり移行は同 Issue で行う旨を記した。
- [x] renumber 参照を確認した。旧 `POLICIES.md` §6 の「AudioLens 独自 Oracle」を参照していた `docs/specs/issue-189-native-gui-feature-matrix.md` を §7 へ更新した。lease 外に未更新の `POLICIES.md` §6 参照は見つからなかった。skill にある新しい §6 参照は新原則への意図した導線である。
- [x] `AGENTS.md` を編集しない判断を確認した。同ファイルには proptest、Kani、TLA+、Lean、昇格路に関する記述がなく、Issue・設計・実装・テスト・レビュー・受入判定で `POLICIES.md` を読む既存の必須導線がある。新原則は `POLICIES.md` に入ったため、重複記載を増やさず既存導線で十分と判断した。
- [x] `docs/**/README.md` の docs index、製品コード、proof、runner、CI 設定には触れていない。

## 受入条件との対応（本 slice 担当分）

| rf#1014 受入条件 | 本 slice の状態 |
| --- | --- |
| 1. 3リポジトリの POLICIES.md へ共通規則を反映し意味上の差異がない | audio-lens 側は転記済み。canonical source からの完全一致転記で ruviform・synvelis の並行 slice と収束させる前提。3 リポジトリ間の最終相互整合性レビューは Coordinator の統合時作業 |
| 2. Harness/Input Domain/Boundary 分割の区別、必要十分な境界、必須記録項目 | `POLICIES.md` §6.2 に転記済み |
| 3. 契約分解・合成、上位契約の保証判定、Stub/assume/Bound/Unwinding Assertions/手法変更の規則 | `POLICIES.md` §6.3〜§6.5 に転記済み |
| 4. 要求状態・verdict・保証種別・同期状態の分離、未証明や CI 成功の Proof Pass 転用禁止 | `POLICIES.md` §6.5 に転記済み。既存 §3 は維持した |
| 5. AGENTS.md・verify スキルの導線、変更影響評価・再検証・証跡同期、旧昇格路の除去、未移行検証の紐付け | skill を現行実態へ同期し、影響評価・`AL_KANI_HARNESSES` による選択再実行・sync commit 更新抑制の手順を追加した。`AGENTS.md` は既存導線で十分と判断して未編集。未移行検証は `scarecrow173/audio-lens#274` へ紐付けた |
| 6. 既存 OS 別受入、仮想優先、通常開発/Daily Full CI、RF/Consumer 責務の維持 | 既存 §1〜§5 を変更せず維持した。canonical 本文の責務規則を §6.6 に転記した |
| 7. 固定階層・独自 DSL/フレームワーク・全 proof 必須 CI を無条件追加しない | 追加していない。canonical 本文でも固定階層否定と DSL/フレームワーク不要を §6.2・§6.6 に転記した |
| 8. 3リポジトリ反映、対象 SHA、相互整合性レビュー、必要 CI 結果の記録 | 本ファイルに base SHA と最終検証対象 SHA、ローカル検証結果を記録した。3 リポジトリ統合時レビューと merge 後 CI 証拠確認は Coordinator が行う |
| 9. 移行 Issue の相互リンクと責務・着手条件 | audio-lens 側は `scarecrow173/audio-lens#274` への導線を `POLICIES.md`・skill・PR 本文に記す。Issue 同士の正式な依存登録は Coordinator が行う |

## 補足

- ローカル検証は変更範囲の検証である。統合検証は merge 後の対象 SHA に対する必須 CI 結果によって確認する。
- 共通原則の copy-verbatim 互換を優先し、audio-lens 固有の未移行注記は共通 §6 本文ではなく §7 側に分離した。
