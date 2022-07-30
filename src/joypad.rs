use std::collections::HashMap;

pub type PadKey = usize;

pub const A : PadKey = 0;
pub const B : PadKey = 1;
pub const SELECT : PadKey = 2;
pub const START : PadKey = 3;
pub const UP : PadKey = 4;
pub const DOWN : PadKey = 5;
pub const LEFT : PadKey = 6;
pub const RIGHT : PadKey = 7;

#[derive(Debug)]
pub struct Joypad {
    current : PadKey,
    state : HashMap<PadKey, bool>
}

impl Joypad {
    pub fn new() -> Self {
        Self { current: A, state: HashMap::new() }
    }

    pub fn read(&mut self, is_debug: bool) -> bool {
        let v = self.state.get(&self.current).unwrap_or(&false);
        
        if !is_debug {
            self.current = (self.current + 1) % (RIGHT + 1);
        }
        *v
    }

    // ボタンの Up = true / Down = false
    pub fn update_key(&mut self, key: PadKey, b : bool) {
        self.state.insert(key, b);
    }

}