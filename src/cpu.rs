
use crate::bus::Bus;

#[derive(Debug)]
pub struct CPU {
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    s: u8,
    pc: u16,

    pub bus : Bus
}

static P_MASK_CARRY : u8 = 1 << 0;
static P_MASK_ZERO : u8 = 1 << 1;
static P_MASK_INT_DISABLE : u8 = 1 << 2;
// static P_MASK_DECIMAL_MODE : u8 = 1 << 3;
// static P_MASK_BREAK_COMMAND : u8 = 1 << 4;
// static P_MASK_OVERFLOW : u8 = 1 << 5;
static P_MASK_NEGATIVE : u8 = 1 << 6;



impl CPU {

    pub fn new(bus : Bus) -> Self {
        CPU { a: 0, x: 0, y: 0, p: 0, s: 0, pc: 0, bus: bus }
    }

    pub fn int_reset(&mut self) {
        let l = self.bus.read(0xFFFC);
        let h = self.bus.read(0xFFFD);
        let addr = (h as u16) << 8 | l as u16;

        self.pc = addr;
    }

    // 1命令の実行
    pub fn step_next(&mut self) {
        let op = self.bus.read(self.pc);
        self.pc += 1;

        match op {
            0x78 => {
                // SEI : set i flag
                self.p |= P_MASK_INT_DISABLE;
            }
            0x8d => {
                // STA absolute
                let l = self.bus.read(self.pc);
                self.pc += 1;
                let h = self.bus.read(self.pc);
                self.pc += 1;
                let addr = (h as u16) << 8 | l as u16;
                self.bus.write(addr, self.a);

            }
            0x9a => {
                // TXS
                self.s = self.x;
            }
            0xa0 => {
                // LDY imm
                let v = self.bus.read(self.pc);
                self.pc += 1;
                self.y = v;
                self.update_status_zero(v);
            }
            0xa2 => {
                // LDX imm
                let v = self.bus.read(self.pc);
                self.pc += 1;
                self.x = v;
                self.update_status_zero(v);
            }
            0xa9 => {
                // LDA imm
                let v = self.bus.read(self.pc);
                self.pc += 1;
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            }
            0xbd => {
                // LDA Absolute,X
                let l = self.bus.read(self.pc);
                self.pc += 1;
                let h = self.bus.read(self.pc);
                self.pc += 1;
                let addr = (h as u16) << 8 | l as u16 + self.x as u16;
                let v = self.bus.read(addr);
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            }
            0x88 => {
                // DEY
                self.y -= 1;
                self.update_status_zero(self.y);
                self.update_status_negative(self.y);
            }
            0xe8 => {
                // INX
                self.x += 1;
                self.update_status_zero(self.x);
                self.update_status_negative(self.x);
            }
            0xd0 => {
                // BNE Rel
                let rel = self.bus.read(self.pc) as i8 as u16;
                self.pc += 1;
                if self.p & P_MASK_ZERO == 0 {
                    println!("branch {}", rel);
                    println!("branch {:#04x} {:#04x}", self.pc, self.pc.wrapping_add(rel));
                    
                    self.pc = self.pc.wrapping_add(rel);
                }
            }
            0x4c => {
                // JMP Absolute
                let l = self.bus.read(self.pc);
                self.pc += 1;
                let h = self.bus.read(self.pc);
                self.pc += 1;
                let addr = (h as u16) << 8 | l as u16;
                self.pc = addr;
            }
            0x18 => {
                // CLC
                self.p &= !P_MASK_CARRY
            }
            0x28 => {
                // PLP

            }
            _ => {
                println!("not impl {:#02x}", op);
                panic!("not impl error");
            }
        }
    }

    fn update_status_zero(&mut self, v : u8) {
        if v == 0 {
            self.p |= P_MASK_ZERO
        } else {
            self.p &= !P_MASK_ZERO
        }
    }
    fn update_status_negative(&mut self, v : u8) {
        if v & 0x80 != 0 {
            self.p |= P_MASK_NEGATIVE
        } else {
            self.p &= !P_MASK_NEGATIVE
        }
    }


}