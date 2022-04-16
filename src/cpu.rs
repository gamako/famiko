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

trait CpuCommand {
    fn exec(&self, cpu: &mut CPU);
    fn desc(&self) -> String;
}

enum AddressingMode {
    Imm(u8),
    Absolute(u16),
    AbsoluteX(u16),
}

impl AddressingMode {
    fn new_imm(cpu: &mut CPU) -> Self {
        let v = cpu.bus.read(cpu.pc);
        cpu.pc += 1;
        AddressingMode::Imm(v)
    }

    fn new_absolute(cpu: &mut CPU) -> Self {
        AddressingMode::Absolute(cpu.read_word())
    }

    fn new_absolute_x(cpu: &mut CPU) -> Self {
        AddressingMode::AbsoluteX(cpu.read_word())
    }

    fn load(&self, cpu: &mut CPU) -> u8 {
        match self {
            AddressingMode::Imm(v) => *v,
            AddressingMode::Absolute(addr) => cpu.bus.read(*addr),
            AddressingMode::AbsoluteX(addr) => cpu.bus.read(*addr + cpu.x as u16),
        }
    }
    fn store(&self, cpu: &mut CPU, v : u8) {
        match self {
            AddressingMode::Imm(_) => { panic!("store imm error"); },
            AddressingMode::Absolute(addr) => cpu.bus.write(*addr, v),
            AddressingMode::AbsoluteX(addr) => cpu.bus.write(*addr + cpu.x as u16, v),
        }
    }

    #[allow(unused)]
    fn desc(&self) -> String {
        match self {
            AddressingMode::Imm(v) => format!("{:#02x}", v),
            AddressingMode::Absolute(addr) => format!("[{:#02x}]", addr),
            AddressingMode::AbsoluteX(addr) => format!("[{:#02x} + x]", addr),
        }
    }
}

enum Command {
    SEI,
    STA(AddressingMode),
    LDA(AddressingMode),
    LDX(AddressingMode),
    LDY(AddressingMode),
    TXS,
    DEY,
    INX,
    BNE(i8),
    JMPAbs(u16),
    CLC,
    PLP,
}

impl Command {
    #[allow(unused)]
    fn desc(&self) -> String {
        match self {
            Command::SEI => { "SEI".to_string() },
            Command::STA(a) => { format!("STA {}", a.desc()) },
            Command::LDA(a) => { format!("LDA {}", a.desc()) },
            Command::LDX(a) => { format!("LDX {}", a.desc()) },
            Command::LDY(a) => { format!("LDY {}", a.desc()) },
            Command::TXS => { "TXS".to_string() },
            Command::DEY => { "DEY".to_string() },
            Command::INX => { "INX".to_string() },
            Command::BNE(v) => { format!("BNE rel {}", v) }
            Command::JMPAbs(addr) => { format!("JMP {}", addr) }
            Command::CLC => { "CLC".to_string() },
            Command::PLP => { "PLP".to_string() },
        }
    }
}

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

    fn fetch(&mut self) -> Command {
        let op = self.bus.read(self.pc);
        self.pc += 1;

        match op {
            0x78 => Command::SEI,
            0x8d => Command::STA(AddressingMode::new_absolute(self)),
            0xa9 => Command::LDA(AddressingMode::new_imm(self)),
            0xbd => Command::LDA(AddressingMode::new_absolute_x(self)),
            0x9a => Command::TXS,
            0xa2 => Command::LDX(AddressingMode::new_imm(self)),
            0xa0 => Command::LDY(AddressingMode::new_imm(self)),
            0x88 => Command::DEY,
            0xe8 => Command::INX,
            0xd0 => {
                let rel = self.bus.read(self.pc) as i8;
                self.pc += 1;
                Command::BNE(rel)
            }
            0x4c => Command::JMPAbs(self.read_word()),
            0x18 => Command::CLC, 
            0x28 => Command::PLP,
            _ => {
                println!("not impl {:#02x}", op);
                panic!("not impl error");
            }
        }
    }

    fn exec_command(&mut self, command: &Command) {
        match command {
            Command::SEI => { self.p |= P_MASK_INT_DISABLE; },
            Command::STA(a) => { a.store(self, self.a) },
            Command::LDA(a) => {
                let v = a.load(self);
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            },
            Command::LDX(a) => {
                let v = a.load(self);
                self.x = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            },
            Command::LDY(a) => {
                let v = a.load(self);
                self.y = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            },
            Command::TXS => self.s = self.x,
            Command::DEY => {
                self.y -= 1;
                self.update_status_zero(self.y);
                self.update_status_negative(self.y);
            },
            Command::INX => {
                self.x += 1;
                self.update_status_zero(self.x);
                self.update_status_negative(self.x);
            },
            Command::BNE(rel) => {
                if self.p & P_MASK_ZERO == 0 {
                    println!("branch {}", rel);
                    println!("branch {:#04x} {:#04x}", self.pc, self.pc.wrapping_add(*rel as u16));
                    
                    self.pc = self.pc.wrapping_add(*rel as u16);
                }
            },
            Command::JMPAbs(addr) => self.pc = *addr,
            Command::CLC => self.p &= !P_MASK_CARRY,
            Command::PLP => {},
        };
    }

    fn read_word(&mut self) -> u16 {
        let l = self.bus.read(self.pc);
        self.pc += 1;
        let h = self.bus.read(self.pc);
        self.pc += 1;
        (h as u16) << 8 | l as u16
    }

    pub fn step_next(&mut self) {
        let command = self.fetch();
        self.exec_command(&command);
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