use std::collections::HashMap;

pub type PadKey = usize;

pub const A : PadKey = 0;
pub const B : PadKey = 1;
pub const Select : PadKey = 2;
pub const Start : PadKey = 3;
pub const Up : PadKey = 4;
pub const Down : PadKey = 5;
pub const Left : PadKey = 6;
pub const Right : PadKey = 7;

pub struct Joypad {
    current : PadKey,
    state : HashMap<PadKey, bool>
}

impl Joypad {
    pub fn new() -> Self {
        Self { current: A, state: HashMap::new() }
    }

    pub fn read(&mut self) -> bool {
        let v = self.state.get(&self.current).unwrap_or(&false);
        
        self.current = (self.current + 1) % (Right + 1);
        *v
    }

    // ボタンの Up = true / Down = false
    pub fn update_key(&mut self, key: PadKey, b : bool) {
        self.state.insert(key, b);
    }

}