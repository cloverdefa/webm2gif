//! 轉換流程與 UI 之間傳遞的資料型別。

use std::path::PathBuf;

#[derive(Clone)]
pub struct Entry {
    pub text: String,
    pub kind: &'static str,
}

pub enum Event {
    Log(Entry, Option<PathBuf>),
    Done,
}
