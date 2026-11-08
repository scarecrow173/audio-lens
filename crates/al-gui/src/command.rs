//! Desktop 画面の command intent 契約(al-gui 内部の語彙)。
//!
//! view の `Table`/`Button` が発行する `CommandIntent::Action` を、
//! service と session が同じ名前で解釈する。GPUI driver の op 名
//! (`ruviform-ui-gpui::COMMAND_INTENT_OP`)は gpui feature の内側にあるため、
//! headless の層ではここに同じ値を持ち、gpui-test で一致を検査する。

use ruviform_ui::behavior::CommandIntent;
use ruviform_ui::view::TAB_SELECT_ACTION;
use serde_json::Value;

/// view の command intent を運ぶ Action op。
pub const COMMAND_INTENT_OP: &str = "rvf.ui.command.v1";
/// Jobs 行選択と Run 操作の論理 target。
pub const JOB_TARGET: &str = "al.job";
/// Desktop 画面の root Tabs の選択の論理 target。
pub const DESKTOP_TARGET: &str = "al.desktop";
/// Jobs 行選択(params: `{"row": n}`)。
pub const ACTION_SELECT: &str = "select";
/// 選択 job の再開(params: `{"job_id": "..."}`)。
pub const ACTION_RESUME: &str = "resume";
/// 実行中 job の取消(params なし)。
pub const ACTION_CANCEL: &str = "cancel";
/// Tab の選択(params: `{"index": n}`)。action 名は ruviform の契約どおり。
pub const ACTION_TAB_SELECT: &str = TAB_SELECT_ACTION;

/// `Action` intent の target・action・params。
pub struct Action {
    /// 論理 target。
    pub target: String,
    /// action 名。
    pub action: String,
    /// action の引数。
    pub params: Option<Value>,
}

/// `rvf.ui.command.v1` の params(`{command_id, expect_receipt, intent}`)から
/// `Action` を取り出す。Action 以外の gesture intent は `None`。
#[must_use]
pub fn decode_action(params: &Value) -> Option<Action> {
    match serde_json::from_value::<CommandIntent>(params.get("intent")?.clone()).ok()? {
        CommandIntent::Action {
            target,
            action,
            params,
            ..
        } => Some(Action {
            target,
            action,
            params,
        }),
        _ => None,
    }
}
