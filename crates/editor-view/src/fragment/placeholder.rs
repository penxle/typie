use editor_common::Rect;

/// layout이 renderer에 전달하는 opaque 데이터.
#[derive(Debug, Clone)]
pub enum PlaceholderData {
    None,
    Bool(bool),
    Number(f64),
    Text(String),
}

/// 장식 요소의 위치를 layout으로 잡는 placeholder.
#[derive(Debug, Clone)]
pub struct PlaceholderFragment {
    pub id: u32,
    pub rect: Rect,
    pub data: PlaceholderData,
}
