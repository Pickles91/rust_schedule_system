#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemState {
    pub time: i32,
}

impl SystemState {
    pub fn new() -> Self {
        Self {
            time: 0,
        }
    }
}