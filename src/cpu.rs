use crate::bus::Bus;
use std::{fmt, time::{Instant, Duration}, thread::sleep};
use log::debug;

static CPU_CLOCK_HZ : u128 = 1789773;
static CPU_CLOCK_UNIT_NSEC : u128 = 1 * 1000 * 1000 * 1000 / CPU_CLOCK_HZ;

#[derive(Debug)]
pub struct Clock {
    start: Instant,
    speed_nsec : u128
}

impl Clock {
    pub fn new() -> Self {
        Clock { start: Instant::now(), speed_nsec: CPU_CLOCK_UNIT_NSEC }
    }

    pub fn new_with(speed_nsec: u128) -> Self {
        Clock { start: Instant::now(), speed_nsec }
    }

    // 次のクロック時間まで待つ
    pub fn wait(&mut self, n: usize) {
        let now = Instant::now();
        let spend = (now - self.start).as_nanos();

        let wait_time = self.speed_nsec - (spend % self.speed_nsec);
        // println!("wait_time : {} {}", spend / self.speed_nsec , wait_time);
        sleep(Duration::from_nanos(wait_time as u64));
    }
}

pub struct CPU {
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    s: u8,
    pc: u16,

    pub bus : Bus,

    clock: Clock,
}

impl fmt::Debug for CPU {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cpu[a:{:#02x},x:{:#02x},y:{:#02x},p:{:#08b},s:{:#02x},pc:{:#04x}]", 
            self.a,
            self.x,
            self.y,
            self.p,
            self.s,
            self.pc,
        )
    }
}

static P_MASK_CARRY : u8 = 1 << 0;
static P_MASK_ZERO : u8 = 1 << 1;
static P_MASK_INT_DISABLE : u8 = 1 << 2;
static P_MASK_DECIMAL_MODE : u8 = 1 << 3;
static P_MASK_BREAK_COMMAND : u8 = 1 << 4;
static P_MASK_OVERFLOW : u8 = 1 << 5;
static P_MASK_NEGATIVE : u8 = 1 << 6;

enum AddressingMode {
    Imm(u8),
    ZeroPage(u8),
    ZeroPageX(u8),
    ZeroPageY(u8),
    Absolute(u16),
    AbsoluteX(u16),
    AbsoluteY(u16),
    IndirectX(u8),
    IndirectY(u8),
}

impl fmt::Debug for AddressingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressingMode::Imm(v) => write!(f, "{:#02x}", v),
            AddressingMode::ZeroPage(addr) => write!(f, "#{:#02x}", addr),
            AddressingMode::ZeroPageX(addr) => write!(f, "#{:#02x},x", addr),
            AddressingMode::ZeroPageY(addr) => write!(f, "#{:#02x},y", addr),
            AddressingMode::Absolute(addr) => write!(f, "[{:#02x}]", addr),
            AddressingMode::AbsoluteX(addr) => write!(f, "[{:#02x} + x]", addr),
            AddressingMode::AbsoluteY(addr) => write!(f, "[{:#02x} + y]", addr),
            AddressingMode::IndirectX(h) => write!(f, "({:#02x} , x)", h),
            AddressingMode::IndirectY(h) => write!(f, "({:#02x} , y)", h),
        }
    }
}

#[derive(Debug)]
enum FlagType {
    Carry,
    Zero,
    IntDisable,
    Decimal,
    BreakCommand,
    Overflow,
    Negative,
}

impl FlagType {
    fn mask(&self) -> u8 {
        match self {
            Self::Carry => P_MASK_CARRY,
            Self::Zero => P_MASK_ZERO,
            Self::IntDisable => P_MASK_INT_DISABLE,
            Self::Decimal => P_MASK_DECIMAL_MODE,
            Self::BreakCommand => P_MASK_BREAK_COMMAND,
            Self::Overflow => P_MASK_OVERFLOW,
            Self::Negative => P_MASK_NEGATIVE,
        }
    }
}

enum Command {
    STA(AddressingMode),
    STX(AddressingMode),
    LDA(AddressingMode),
    LDX(AddressingMode),
    LDY(AddressingMode),
    TXS,
    DEX,
    DEY,
    INX,
    CPX(AddressingMode),
    CPY(AddressingMode),
    BPL(i8),
    BNE(i8),
    JMPAbs(u16),
    CL(FlagType),
    SE(FlagType),
    PLP,
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::STA(a) => write!(f, "STA {:?}", a),
            Command::STX(a) => write!(f, "STX {:?}", a),
            Command::LDA(a) => write!(f, "LDA {:?}", a),
            Command::LDX(a) => write!(f, "LDX {:?}", a),
            Command::LDY(a) => write!(f, "LDY {:?}", a),
            Command::TXS => write!(f, "TXS"),
            Command::DEX => write!(f, "DEX"),
            Command::DEY => write!(f, "DEY"),
            Command::INX => write!(f, "INX"),
            Command::CPX(a) => write!(f, "CPX {:?}", a),
            Command::CPY(a) => write!(f, "CPY {:?}", a),
            Command::BPL(v) => write!(f, "BNE rel {}", v),
            Command::BNE(v) => write!(f, "BNE rel {}", v),
            Command::JMPAbs(addr) => write!(f, "JMP {}", addr),
            Command::CL(t) =>
                match t {
                    FlagType::Carry => write!(f, "CLC"),
                    FlagType::IntDisable => write!(f, "CLI"),
                    FlagType::Decimal => write!(f, "CLD"),
                    FlagType::Overflow => write!(f, "CLV"),
                    _ => write!(f, "CL (xxx{:?})", t),
                },
            Command::SE(t) =>
                match t {
                    FlagType::Carry => write!(f, "SEC"),
                    FlagType::IntDisable => write!(f, "SEI"),
                    FlagType::Decimal => write!(f, "SED"),
                    _ => write!(f, "CL (xxx{:?})", t),
                },
            Command::PLP => write!(f, "PLP"),
        }
    }
}

impl CPU {
    pub fn new(bus : Bus) -> Self {
        CPU { a: 0, x: 0, y: 0, p: 0, s: 0, pc: 0, bus: bus, clock: Clock::new() }
    }

    pub fn int_reset(&mut self) {
        let l = self.bus.read(0xFFFC);
        let h = self.bus.read(0xFFFD);
        let addr = (h as u16) << 8 | l as u16;

        self.pc = addr;
    }

    fn fetch(&mut self) -> Command {
        let op = self.read_byte_pc();

        match op {
            0x8d => Command::STA(self.new_absolute()),

            0x86 => Command::STX(self.new_zero_page()),
            0x8e => Command::STX(self.new_absolute()),
            0x96 => Command::STX(self.new_zero_page_y()),

            0xa1 => Command::LDA(self.new_indirect_x()),
            0xa5 => Command::LDA(self.new_zero_page()),
            0xa9 => Command::LDA(self.new_imm()),
            0xad => Command::LDA(self.new_absolute()),
            0xb1 => Command::LDA(self.new_indirect_y()),
            0xb5 => Command::LDA(self.new_zero_page_x()),
            0xb9 => Command::LDA(self.new_absolute_y()),
            0xbd => Command::LDA(self.new_absolute_x()),

            0x9a => Command::TXS,
            0xa2 => Command::LDX(self.new_imm()),
            0xa0 => Command::LDY(self.new_imm()),
            0xca => Command::DEX,
            0x88 => Command::DEY,
            0xe0 => Command::CPX(self.new_imm()),
            0xe8 => Command::INX,

            0x10 => Command::BPL(self.read_byte_pc() as i8),
            0xd0 => Command::BNE(self.read_byte_pc() as i8),
            
            0x4c => Command::JMPAbs(self.read_word_pc()),
            0x18 => Command::CL(FlagType::Carry),
            0x58 => Command::CL(FlagType::IntDisable),
            0xb8 => Command::CL(FlagType::Overflow),
            0xd8 => Command::CL(FlagType::Decimal),
            0x38 => Command::SE(FlagType::Carry),
            0x78 => Command::SE(FlagType::IntDisable),
            0xf8 => Command::SE(FlagType::Decimal),
            0x28 => Command::PLP,
            _ => {
                println!("not impl {:#02x}", op);
                panic!("not impl error");
            }
        }
    }
    
    fn exec_branch<F : Fn(u8) -> bool>(&mut self, cond : F, rel : i8) {
        if cond(self.p) {
            println!("branch {}", rel);
            self.pc = self.pc.wrapping_add(rel as u16);
        }
    }

    fn exec_command(&mut self, command: &Command) {
        match command {
            Command::STA(a) => { self.store(a, self.a) },
            Command::STX(a) => { self.store(a, self.x) },
            Command::LDA(a) => {
                let v = self.load(a);
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            },
            Command::LDX(a) => {
                let v = self.load(a);
                self.x = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            },
            Command::LDY(a) => {
                let v = self.load(a);
                self.y = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            },
            Command::TXS => self.s = self.x,
            Command::DEX => {
                self.x = self.x.wrapping_add(-1 as i8 as u8);
                self.update_status_zero(self.x);
                self.update_status_negative(self.x);
            },
            Command::DEY => {
                self.y = self.y.wrapping_add(-1 as i8 as u8);
                self.update_status_zero(self.y);
                self.update_status_negative(self.y);
            },
            Command::INX => {
                self.x = self.x.wrapping_add(1 as i8 as u8);
                self.update_status_zero(self.x);
                self.update_status_negative(self.x);
            },
            Command::CPX(a) => {
                let (v, b) = self.x.overflowing_sub(self.load(a));
                self.update_status_carry(b);
                self.update_status_zero(v);
                self.update_status_negative(v);
            }
            Command::CPY(a) => {
                let (v, b) = self.y.overflowing_sub(self.load(a));
                self.update_status_carry(b);
                self.update_status_zero(v);
                self.update_status_negative(v);
            }
            Command::BPL(rel) => self.exec_branch( |p|{ (p & P_MASK_NEGATIVE) == 0}, *rel ),
            Command::BNE(rel) => self.exec_branch( |p|{ (p & P_MASK_ZERO) == 0}, *rel ),

            Command::JMPAbs(addr) => self.pc = *addr,
            Command::CL(f) => self.p &= !f.mask(),
            Command::SE(f) => self.p |= f.mask(),
            Command::PLP => {
                let v = self.bus.read(self.s as u16 + 0x0100);
                self.s += 1;
                self.p = v;
            },
        };
    }

    fn read_byte(&mut self, addr: u16) -> u8 {
        self.clock.wait(1);
        self.bus.read(addr)
    }

    fn read_byte_pc(&mut self) -> u8 {
        let v = self.read_byte(self.pc);
        self.pc += 1;
        v
    }

    fn read_word(&mut self, addr: u16) -> u16 {
        let l = self.read_byte(addr);
        let h = self.read_byte(addr + 1);
        (h as u16) << 8 | l as u16
    }

    fn read_word_pc(&mut self) -> u16 {
        let v = self.read_word(self.pc);
        self.pc += 2;
        v
    }

    fn write_byte(&mut self, addr: u16, v: u8) {
        self.clock.wait(1);
        self.bus.write(addr, v);
    }

    fn new_imm(&mut self) -> AddressingMode {
        AddressingMode::Imm(self.read_byte_pc())
    }

    fn new_zero_page(&mut self) -> AddressingMode {
        AddressingMode::ZeroPage(self.read_byte_pc())
    }

    fn new_zero_page_x(&mut self) -> AddressingMode {
        AddressingMode::ZeroPage(self.read_byte_pc())
    }

    fn new_zero_page_y(&mut self) -> AddressingMode {
        AddressingMode::ZeroPage(self.read_byte_pc())
    }

    fn new_absolute(&mut self) -> AddressingMode {
        AddressingMode::Absolute(self.read_word_pc())
    }

    fn new_absolute_x(&mut self) -> AddressingMode {
        AddressingMode::AbsoluteX(self.read_word_pc())
    }

    fn new_absolute_y(&mut self) -> AddressingMode {
        AddressingMode::AbsoluteY(self.read_word_pc())
    }

    fn new_indirect_x(&mut self) -> AddressingMode {
        AddressingMode::IndirectX(self.read_byte_pc())
    }

    fn new_indirect_y(&mut self) -> AddressingMode {
        AddressingMode::IndirectY(self.read_byte_pc())
    }

    fn load(&mut self, addr_mode: &AddressingMode) -> u8 {
        match *addr_mode {
            AddressingMode::Imm(v) => v,
            AddressingMode::ZeroPage(addr) => self.read_byte(addr as u16),
            AddressingMode::ZeroPageX(addr) => self.read_byte(addr as u16 + self.x as u16),
            AddressingMode::ZeroPageY(addr) => self.read_byte(addr as u16 + self.y as u16),
            AddressingMode::Absolute(addr) => self.read_byte(addr),
            AddressingMode::AbsoluteX(addr) => self.read_byte(addr + self.x as u16),
            AddressingMode::AbsoluteY(addr) => self.read_byte(addr + self.y as u16),
            AddressingMode::IndirectX(h) => self.read_byte((h as u16) << 8 + self.x as u16),
            AddressingMode::IndirectY(h) => self.read_byte((h as u16) << 8 + self.y as u16),
        }
    }

    fn store(&mut self, addr_mode: &AddressingMode, v : u8) {
        match *addr_mode {
            AddressingMode::Imm(_) => { panic!("store imm error"); },
            AddressingMode::ZeroPage(addr) => self.write_byte(addr as u16, v),
            AddressingMode::ZeroPageX(addr) => self.write_byte(addr as u16 + self.x as u16, v),
            AddressingMode::ZeroPageY(addr) => self.write_byte(addr as u16 + self.y as u16, v),
            AddressingMode::Absolute(addr) => self.write_byte(addr, v),
            AddressingMode::AbsoluteX(addr) => self.write_byte(addr + self.x as u16, v),
            AddressingMode::AbsoluteY(addr) => self.write_byte(addr + self.y as u16, v),
            AddressingMode::IndirectX(h) => self.write_byte((h as u16) << 8 + self.x as u16, v),
            AddressingMode::IndirectY(h) => self.write_byte((h as u16) << 8 + self.y as u16, v),
        }
    }

    pub fn step_next(&mut self) {
        let pc = self.pc;
        let command = self.fetch();
        println!("{:#04x} {:?}", pc, command);
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
    fn update_status_carry(&mut self, b : bool) {
        if b {
            self.p |= P_MASK_CARRY
        } else {
            self.p &= !P_MASK_CARRY
        }
    }

}