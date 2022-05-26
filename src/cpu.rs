use crate::bus::Bus;
use std::{fmt::{self, format}, time::{Instant, Duration}, thread::sleep, ops::Add};
use crate::hex::dump_bytes;
use std::string::ToString;
use strum_macros;
use std::fmt::Write as FmtWrite;

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

impl CPU {
    fn log_str(&self) -> String {
        format!("A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}", 
            self.a,
            self.x,
            self.y,
            self.p,
            self.s,
        )
    }
}

impl fmt::Debug for CPU {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PC:{:04X}", 
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
static P_MASK_OVERFLOW : u8 = 1 << 6;
static P_MASK_NEGATIVE : u8 = 1 << 7;

enum AddressingMode {
    Imm(u8),
    ZeroPage(u8),
    ZeroPageX(u8),
    ZeroPageY(u8),
    Absolute(u16),
    AbsoluteX(u16),
    AbsoluteY(u16),
    Indirect(u16),
    IndirectX(u8),
    IndirectY(u8),
    Relative(u8),
}

impl AddressingMode {
    fn len(&self) -> usize {
        match self {
            AddressingMode::Imm(_) => 1,
            AddressingMode::ZeroPage(_) => 1,
            AddressingMode::ZeroPageX(_) => 1,
            AddressingMode::ZeroPageY(_) => 1,
            AddressingMode::Absolute(_) => 2,
            AddressingMode::AbsoluteX(_) => 1,
            AddressingMode::AbsoluteY(_) => 1,
            AddressingMode::Indirect(_) => 2,
            AddressingMode::IndirectX(_) => 1,
            AddressingMode::IndirectY(_) => 1,
            AddressingMode::Relative(_) => 1,
        }

    }
}

impl fmt::Debug for AddressingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressingMode::Imm(v) => write!(f, "{:02X}", v),
            AddressingMode::ZeroPage(addr) => write!(f, "#{:02X}", addr),
            AddressingMode::ZeroPageX(addr) => write!(f, "#{:02X},x", addr),
            AddressingMode::ZeroPageY(addr) => write!(f, "#{:02X},y", addr),
            AddressingMode::Absolute(addr) => write!(f, "{:02X}", addr),
            AddressingMode::AbsoluteX(addr) => write!(f, "[{:02X} + x]", addr),
            AddressingMode::AbsoluteY(addr) => write!(f, "[{:02X} + y]", addr),
            AddressingMode::Indirect(addr) => write!(f, "({:04X} , x)", addr),
            AddressingMode::IndirectX(h) => write!(f, "({:02X} , x)", h),
            AddressingMode::IndirectY(h) => write!(f, "({:02X} , y)", h),
            AddressingMode::Relative(rel) => write!(f, "({:02X} , y)", rel),
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

#[derive(strum::Display, Debug)]
enum Command {
    STA(AddressingMode),
    STX(AddressingMode),
    STY(AddressingMode),
    LDA(AddressingMode),
    LDX(AddressingMode),
    LDY(AddressingMode),
    TXS,
    DEX,
    DEY,
    INX,
    INY,
    AND(AddressingMode),
    ORA(AddressingMode),
    EOR(AddressingMode),
    ADC(AddressingMode),
    CMP(AddressingMode),
    CPX(AddressingMode),
    CPY(AddressingMode),
    BPL(AddressingMode),
    BCC(AddressingMode),
    BCS(AddressingMode),
    BEQ(AddressingMode),
    BVS(AddressingMode),
    BVC(AddressingMode),
    BMI(AddressingMode),
    BNE(AddressingMode),
    JMP(AddressingMode),
    JSR(AddressingMode),
    RTS,
    CL(FlagType),
    SE(FlagType),
    BIT(AddressingMode),
    PHA,
    PHP,
    PLA,
    PLP,
    NOP,
}
impl Command {
    fn type_name(&self) -> String {
        match self {
            Command::STA(_) => "STA".to_string(),
            Command::STX(_) => "STX".to_string(),
            Command::STY(_) => "STY".to_string(),
            Command::LDA(_) => "LDA".to_string(),
            Command::LDX(_) => "LDX".to_string(),
            Command::LDY(_) => "LDY".to_string(),
            Command::AND(_) => "AND".to_string(),
            Command::EOR(_) => "EOR".to_string(),
            Command::ADC(_) => "ADC".to_string(),
            Command::ORA(_) => "ORA".to_string(),
            Command::CMP(_) => "CMP".to_string(),
            Command::CPX(_) => "CPX".to_string(),
            Command::CPY(_) => "CPY".to_string(),
            Command::BPL(_) => "BPL".to_string(),
            Command::BMI(_) => "BMI".to_string(),
            Command::BNE(_) => "BNE".to_string(),
            Command::BEQ(_) => "BEQ".to_string(),
            Command::BCC(_) => "BCC".to_string(),
            Command::BCS(_) => "BCS".to_string(),
            Command::BVS(_) => "BVS".to_string(),
            Command::BVC(_) => "BVC".to_string(),
            Command::JMP(_) => "JMP".to_string(),
            Command::JSR(_) => "JSR".to_string(),
            Command::CL(t) =>
                match t {
                    FlagType::Carry => "CLC".to_string(),
                    FlagType::IntDisable => "CLI".to_string(),
                    FlagType::Decimal => "CLD".to_string(),
                    FlagType::Overflow => "CLV".to_string(),
                    _ => "CL?".to_string(),
                },
            Command::SE(t) =>
                match t {
                    FlagType::Carry => "SEC".to_string(),
                    FlagType::IntDisable => "SEI".to_string(),
                    FlagType::Decimal => "SED".to_string(),
                    _ => "SE?".to_string(),
                },
            Command::BIT(_) => "BIT".to_string(),
            Command::PHA => "PHA".to_string(),
            Command::PHP => "PHP".to_string(),
            _ => self.to_string(),
        }
    
    }
}

impl CPU {
    pub fn new(bus : Bus) -> Self {
        CPU { a: 0, x: 0, y: 0, p: 0x24, s: 0xfd, pc: 0, bus: bus, clock: Clock::new() }
    }

    pub fn int_reset(&mut self) {
        let l = self.bus.read(0xFFFC);
        let h = self.bus.read(0xFFFD);
        let addr = (h as u16) << 8 | l as u16;

        self.pc = addr;
    }

    pub fn init_pc(&mut self, addr : u16) {
        self.pc = addr;
    }

    fn new_command<
        FC : FnOnce(AddressingMode) -> Command,
        FA : FnOnce(&mut Self) -> (AddressingMode, Vec<u8>),
        >(&mut self, op: u8, f_c:FC, f_a:FA) -> (Command, Vec<u8>) {

        let v = vec![op];
        let (a, b) = f_a(self);
        let command = f_c(a);
        (command, [v, b].concat())
    }

    fn fetch(&mut self) -> (Command, Vec<u8>) {
        let op = self.read_byte_pc();

        match op {
            0x81 => self.new_command(op, Command::STA, Self::new_indirect_x),
            0x85 => self.new_command(op, Command::STA, Self::new_zero_page),
            0x8d => self.new_command(op, Command::STA, Self::new_absolute),
            0x91 => self.new_command(op, Command::STA, Self::new_indirect_y),
            0x95 => self.new_command(op, Command::STA, Self::new_zero_page_x),
            0x99 => self.new_command(op, Command::STA, Self::new_absolute_y),
            0x9d => self.new_command(op, Command::STA, Self::new_absolute_x),
            0x86 => self.new_command(op, Command::STX, Self::new_zero_page),
            0x8e => self.new_command(op, Command::STX, Self::new_absolute),
            0x96 => self.new_command(op, Command::STX, Self::new_zero_page_y),
            0x84 => self.new_command(op, Command::STY, Self::new_zero_page),
            0x8c => self.new_command(op, Command::STY, Self::new_absolute),
            0x94 => self.new_command(op, Command::STY, Self::new_zero_page_x),
            0xa1 => self.new_command(op, Command::LDA, Self::new_indirect_x),
            0xa5 => self.new_command(op, Command::LDA, Self::new_zero_page),
            0xa9 => self.new_command(op, Command::LDA, Self::new_imm),
            0xad => self.new_command(op, Command::LDA, Self::new_absolute),
            0xb1 => self.new_command(op, Command::LDA, Self::new_indirect_y),
            0xb5 => self.new_command(op, Command::LDA, Self::new_zero_page_x),
            0xb9 => self.new_command(op, Command::LDA, Self::new_absolute_y),
            0xbd => self.new_command(op, Command::LDA, Self::new_absolute_x),

            0x01 => self.new_command(op, Command::ORA, Self::new_indirect_x),
            0x05 => self.new_command(op, Command::ORA, Self::new_zero_page),
            0x09 => self.new_command(op, Command::ORA, Self::new_imm),
            0x0d => self.new_command(op, Command::ORA, Self::new_absolute),
            0x11 => self.new_command(op, Command::ORA, Self::new_indirect_y),
            0x15 => self.new_command(op, Command::ORA, Self::new_zero_page_x),
            0x19 => self.new_command(op, Command::ORA, Self::new_absolute_y),
            0x1d => self.new_command(op, Command::ORA, Self::new_absolute_x),

            0x41 => self.new_command(op, Command::EOR, Self::new_indirect_x),
            0x45 => self.new_command(op, Command::EOR, Self::new_zero_page),
            0x49 => self.new_command(op, Command::EOR, Self::new_imm),
            0x4d => self.new_command(op, Command::EOR, Self::new_absolute),
            0x51 => self.new_command(op, Command::EOR, Self::new_indirect_y),
            0x55 => self.new_command(op, Command::EOR, Self::new_zero_page_x),
            0x59 => self.new_command(op, Command::EOR, Self::new_absolute_y),
            0x5d => self.new_command(op, Command::EOR, Self::new_absolute_x),

            0x21 => self.new_command(op, Command::AND, Self::new_indirect_x),
            0x25 => self.new_command(op, Command::AND, Self::new_zero_page),
            0x29 => self.new_command(op, Command::AND, Self::new_imm),
            0x2d => self.new_command(op, Command::AND, Self::new_absolute),
            0x31 => self.new_command(op, Command::AND, Self::new_indirect_y),
            0x35 => self.new_command(op, Command::AND, Self::new_zero_page_x),
            0x39 => self.new_command(op, Command::AND, Self::new_absolute_y),
            0x3d => self.new_command(op, Command::AND, Self::new_absolute_x),

            0x61 => self.new_command(op, Command::ADC, Self::new_indirect_x),
            0x65 => self.new_command(op, Command::ADC, Self::new_zero_page),
            0x69 => self.new_command(op, Command::ADC, Self::new_imm),
            0x6d => self.new_command(op, Command::ADC, Self::new_absolute),
            0x71 => self.new_command(op, Command::ADC, Self::new_indirect_y),
            0x75 => self.new_command(op, Command::ADC, Self::new_zero_page_x),
            0x79 => self.new_command(op, Command::ADC, Self::new_absolute_y),
            0x7d => self.new_command(op, Command::ADC, Self::new_absolute_x),

            0x9a => (Command::TXS, vec![op]),
            0xa2 => self.new_command(op, Command::LDX, Self::new_imm),
            0xa0 => self.new_command(op, Command::LDY, Self::new_imm),
            0xca => (Command::DEX, vec![op]),
            0x88 => (Command::DEY, vec![op]),

            0xc1 => self.new_command(op, Command::CMP, Self::new_indirect_x),
            0xc5 => self.new_command(op, Command::CMP, Self::new_zero_page),
            0xc9 => self.new_command(op, Command::CMP, Self::new_imm),
            0xcd => self.new_command(op, Command::CMP, Self::new_absolute),
            0xd5 => self.new_command(op, Command::CMP, Self::new_zero_page_x),
            0xdd => self.new_command(op, Command::CMP, Self::new_absolute_x),
            0xd9 => self.new_command(op, Command::CMP, Self::new_absolute_y),
            0xd1 => self.new_command(op, Command::CMP, Self::new_indirect_x),
            0xe0 => self.new_command(op, Command::CPX, Self::new_imm),
            0xe4 => self.new_command(op, Command::CPX, Self::new_zero_page),
            0xec => self.new_command(op, Command::CPX, Self::new_absolute),
            0xc0 => self.new_command(op, Command::CPY, Self::new_imm),
            0xc4 => self.new_command(op, Command::CPY, Self::new_zero_page),
            0xcc => self.new_command(op, Command::CPY, Self::new_absolute),

            0xe8 => (Command::INX, vec![op]),
            0xc8 => (Command::INY, vec![op]),

            0x10 => self.new_command(op, Command::BPL, Self::new_relative),
            0x50 => self.new_command(op, Command::BVC, Self::new_relative),
            0x70 => self.new_command(op, Command::BVS, Self::new_relative),
            0x90 => self.new_command(op, Command::BCC, Self::new_relative),
            0xb0 => self.new_command(op, Command::BCS, Self::new_relative),
            0x30 => self.new_command(op, Command::BMI, Self::new_relative),
            0xd0 => self.new_command(op, Command::BNE, Self::new_relative),
            0xf0 => self.new_command(op, Command::BEQ, Self::new_relative),

            0x4c => self.new_command(op, Command::JMP, Self::new_absolute),
            0x6c => self.new_command(op, Command::JMP, Self::new_indirect),
            0x20 => self.new_command(op, Command::JSR, Self::new_absolute),
            0x60 => (Command::RTS, vec![op]),

            0x18 => (Command::CL(FlagType::Carry), vec![op]),
            0x58 => (Command::CL(FlagType::IntDisable), vec![op]),
            0xb8 => (Command::CL(FlagType::Overflow), vec![op]),
            0xd8 => (Command::CL(FlagType::Decimal), vec![op]),
            0x38 => (Command::SE(FlagType::Carry), vec![op]),
            0x78 => (Command::SE(FlagType::IntDisable), vec![op]),
            0xf8 => (Command::SE(FlagType::Decimal), vec![op]),

            0x24 => self.new_command(op, Command::BIT, Self::new_zero_page),
            0x2c => self.new_command(op, Command::BIT, Self::new_absolute),

            0x48 => (Command::PHA, vec![op]),
            0x08 => (Command::PHP, vec![op]),
            0x68 => (Command::PLA, vec![op]),
            0x28 => (Command::PLP, vec![op]),
            0xea => (Command::NOP, vec![op]),
            _ => {
                println!("not impl {:#02x}", op);
                panic!("not impl error");
            }
        }
    }
    
    fn exec_branch<F : Fn(u8) -> bool>(&mut self, cond : F, addr : &AddressingMode, l: &mut String) {
        match addr {
            AddressingMode::Relative(a) => {
                let addr = self.pc.wrapping_add(*a as i8 as u16);
                write!(l, "${:04X}", addr).unwrap();
                if cond(self.p) {
                    self.pc = addr;
                }
            },
            _ => { panic!("branch addressing mode error") }
        };
    }

    fn exec_command(&mut self, command: &Command) -> String {
        let mut l = String::new();
        
        write!(l, "{} ", command.type_name()).unwrap();
        match command {
            Command::STA(a) => { self.store(a, self.a, &mut l) },
            Command::STX(a) => { self.store(a, self.x, &mut l) },
            Command::STY(a) => { self.store(a, self.y, &mut l) },
            Command::LDA(a) => {
                let v = self.load(a, &mut l);
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
                //format!("{}", "STA")
            },
            Command::LDX(a) => {
                let v = self.load(a, &mut l);
                self.x = v;
                self.update_status_zero(v);
                self.update_status_negative(v);

            },
            Command::LDY(a) => {
                let v = self.load(a, &mut l);
                self.y = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            },
            Command::AND(a) => {
                let v = self.load(a, &mut l) & self.a;
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            },
            Command::ORA(a) => {
                let v = self.load(a, &mut l) | self.a;
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            },
            Command::EOR(a) => {
                let v = self.load(a, &mut l) ^ self.a;
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            },
            Command::ADC(addr) => {
                // let c = (self.p & P_MASK_CARRY) as i8;
                // let (v, b1) = (self.load(a, &mut l) as i8).overflowing_add(self.a as i8);
                // let (v, b2) = v.overflowing_add(c);
                // let b = b1 | b2;
                // self.a = v as u8;

                let a = self.a;
                let b = self.load(addr, &mut l);
                let c = self.p & P_MASK_CARRY;
                let d = a  as u16 + b  as u16 + c  as u16;
                self.a = (d & 0xff) as u8;

                self.update_status_carry(d > 0xff);
                self.update_status_overflow_of((a ^ b) & 0x80 == 0 && (self.a ^ a) & 0x80 != 0);

                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
            },
            Command::TXS => self.s = self.x,
            Command::DEX => {
                self.x = self.x.wrapping_sub(1u8);
                self.update_status_zero(self.x);
                self.update_status_negative(self.x);
            },
            Command::DEY => {
                self.y = self.y.wrapping_sub(1u8);
                self.update_status_zero(self.y);
                self.update_status_negative(self.y);
            },
            Command::INX => {
                self.x = self.x.wrapping_add(1u8);
                self.update_status_zero(self.x);
                self.update_status_negative(self.x);
            },
            Command::INY => {
                self.y = self.y.wrapping_add(1u8);
                self.update_status_zero(self.y);
                self.update_status_negative(self.y);
            },
            Command::CMP(a) => {
                let r = -(self.load(a, &mut l) as i8) as u8;
                let (v, b) = self.a.overflowing_add(r);
                self.update_status_carry(b);
                self.update_status_zero(v);
                self.update_status_negative(v);
            }
            Command::CPX(a) => {
                let (v, b) = self.x.overflowing_sub(self.load(a, &mut l));
                self.update_status_carry(b);
                self.update_status_zero(v);
                self.update_status_negative(v);
            }
            Command::CPY(a) => {
                let (v, b) = self.y.overflowing_sub(self.load(a, &mut l));
                self.update_status_carry(b);
                self.update_status_zero(v);
                self.update_status_negative(v);
            }
            Command::BPL(a) => self.exec_branch( |p|{ (p & P_MASK_NEGATIVE) == 0}, a, &mut l),
            Command::BMI(a) => self.exec_branch( |p|{ (p & P_MASK_NEGATIVE) != 0}, a, &mut l),
            Command::BNE(a) => self.exec_branch( |p|{ (p & P_MASK_ZERO) == 0}, a, &mut l),
            Command::BEQ(a) => self.exec_branch( |p|{ (p & P_MASK_ZERO) != 0}, a, &mut l),
            Command::BCC(a) => self.exec_branch( |p|{ (p & P_MASK_CARRY) == 0}, a, &mut l),
            Command::BCS(a) => self.exec_branch( |p|{ (p & P_MASK_CARRY) != 0}, a, &mut l),
            Command::BVS(a) => self.exec_branch( |p|{ (p & P_MASK_OVERFLOW) != 0}, a, &mut l),
            Command::BVC(a) => self.exec_branch( |p|{ (p & P_MASK_OVERFLOW) == 0}, a, &mut l),

            Command::JMP(AddressingMode::Absolute(addr)) => {
                write!(l, "${:04X}", addr).unwrap();
                self.pc = *addr
            }
            Command::JMP(AddressingMode::Indirect(addr)) => self.pc = self.read_word(*addr),
            Command::JSR(AddressingMode::Absolute(addr)) => {
                write!(l, "${:04X}", addr).unwrap();
                self.push_stack_word(self.pc);
                self.pc = *addr
            }
            Command::RTS => {
                self.pc = self.pop_stack_word();
            }
            Command::CL(f) => self.p &= !f.mask(),
            Command::SE(f) => self.p |= f.mask(),
            Command::BIT(a) => {
                let m = self.load(a, &mut l);
                let r = m & & self.a;
                self.update_status_zero(r);
                self.update_status_overflow(m);
                self.update_status_negative(m);

            }
            Command::PHA => {
                self.s -= 1;
                self.bus.write(self.s as u16 + 0x0100, self.a);
            },
            Command::PHP => {
                self.s -= 1;
                let v = self.p | P_MASK_BREAK_COMMAND;
                self.bus.write(self.s as u16 + 0x0100, v);
            },
            Command::PLP => {
                let v = self.bus.read(self.s as u16 + 0x0100);
                self.s += 1;
                self.p = (self.p & 0x30) | (v & 0xcf);
            },
            Command::PLA => {
                let v = self.bus.read(self.s as u16 + 0x0100);
                self.s += 1;
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            },
            Command::NOP => {}
            _ => { panic!("xx") }
        };
        return l;
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

    fn push_stack(&mut self, v: u8) {
        let addr = 0x100u16 | (self.s as u16);
        self.s = self.s.wrapping_sub(1);
        self.write_byte(addr, v);
    }

    fn push_stack_word(&mut self, v: u16) {
        self.push_stack(((v >> 8) & 0x00ff) as u8);
        self.push_stack((v & 0x00ff) as u8);
    }

    fn pop_stack(&mut self) -> u8 {
        self.s = self.s.wrapping_add(1);
        let addr = 0x100u16 | (self.s as u16);
        self.read_byte(addr)
    }

    fn pop_stack_word(&mut self) -> u16 {
        let l = self.pop_stack();
        let h = self.pop_stack();
        (h as u16) << 8 | l as u16 
    }

    fn new_addr_and_u8<F: FnOnce(u8) -> AddressingMode>(&mut self, f : F) -> (AddressingMode, Vec<u8>) {
        let v = self.read_byte_pc();
        (f(v), vec![v])
    }
    fn new_addr_and_u16<F: FnOnce(u16) -> AddressingMode>(&mut self, f : F) -> (AddressingMode, Vec<u8>) {
        let l = self.read_byte_pc();
        let h = self.read_byte_pc();
        let v = (h as u16) << 8 | l as u16;
        (f(v), vec![l, h])
    }

    fn new_imm(&mut self) -> (AddressingMode, Vec<u8>) {
        self.new_addr_and_u8(AddressingMode::Imm)
    }

    fn new_zero_page(&mut self) -> (AddressingMode, Vec<u8>) {
        self.new_addr_and_u8(AddressingMode::ZeroPage)
    }

    fn new_zero_page_x(&mut self) -> (AddressingMode, Vec<u8>) {
        self.new_addr_and_u8(AddressingMode::ZeroPage)
    }

    fn new_zero_page_y(&mut self) -> (AddressingMode, Vec<u8>) {
        self.new_addr_and_u8(AddressingMode::ZeroPage)
    }

    fn new_absolute(&mut self) -> (AddressingMode, Vec<u8>) {
        self.new_addr_and_u16(AddressingMode::Absolute)
    }

    fn new_absolute_x(&mut self) -> (AddressingMode, Vec<u8>) {
        self.new_addr_and_u16(AddressingMode::AbsoluteX)
    }

    fn new_absolute_y(&mut self) -> (AddressingMode, Vec<u8>) {
        self.new_addr_and_u16(AddressingMode::AbsoluteY)
    }

    fn new_indirect(&mut self) -> (AddressingMode, Vec<u8>) {
        self.new_addr_and_u16(AddressingMode::Indirect)
    }

    fn new_indirect_x(&mut self) -> (AddressingMode, Vec<u8>) {
        self.new_addr_and_u8(AddressingMode::IndirectX)
    }

    fn new_indirect_y(&mut self) -> (AddressingMode, Vec<u8>) {
        self.new_addr_and_u8(AddressingMode::IndirectY)
    }

    fn new_relative(&mut self) -> (AddressingMode, Vec<u8>) {
        self.new_addr_and_u8(AddressingMode::Relative)
    }

    fn load(&mut self, addr_mode: &AddressingMode, l: &mut String) -> u8 {
        match *addr_mode {
            AddressingMode::Imm(v) => {
                write!(l, "#${:02X}", v).unwrap();
                v
            }
            AddressingMode::ZeroPage(addr) => {
                let v = self.read_byte(addr as u16);
                write!(l, "${:02X} = {:02X}", addr, v).unwrap();
                v
            }
            AddressingMode::ZeroPageX(addr) => self.read_byte(addr as u16 + self.x as u16),
            AddressingMode::ZeroPageY(addr) => self.read_byte(addr as u16 + self.y as u16),
            AddressingMode::Absolute(addr) => self.read_byte(addr),
            AddressingMode::AbsoluteX(addr) => self.read_byte(addr + self.x as u16),
            AddressingMode::AbsoluteY(addr) => self.read_byte(addr + self.y as u16),
            AddressingMode::Indirect(h) => panic!("load indirect"),
            AddressingMode::IndirectX(h) => self.read_byte((h as u16) << 8 + self.x as u16),
            AddressingMode::IndirectY(h) => self.read_byte((h as u16) << 8 + self.y as u16),
            AddressingMode::Relative(rel) => panic!("load rel"),
        }
    }

    fn store(&mut self, addr_mode: &AddressingMode, v : u8, l: &mut String) {
        match *addr_mode {
            AddressingMode::Imm(_) => { panic!("store imm error"); },
            AddressingMode::ZeroPage(addr) => {
                let old = self.read_byte(addr as u16);
                write!(l, "${:02X} = {:02X}", addr, old).unwrap();
                self.write_byte(addr as u16, v)
            }
            AddressingMode::ZeroPageX(addr) => self.write_byte(addr as u16 + self.x as u16, v),
            AddressingMode::ZeroPageY(addr) => self.write_byte(addr as u16 + self.y as u16, v),
            AddressingMode::Absolute(addr) => self.write_byte(addr, v),
            AddressingMode::AbsoluteX(addr) => self.write_byte(addr + self.x as u16, v),
            AddressingMode::AbsoluteY(addr) => self.write_byte(addr + self.y as u16, v),
            AddressingMode::Indirect(h) => panic!("store indirect"),
            AddressingMode::IndirectX(h) => self.write_byte((h as u16) + self.x as u16, v),
            AddressingMode::IndirectY(h) => self.write_byte((h as u16) << 8 + self.y as u16, v),
            AddressingMode::Relative(_) => panic!("store rel"),
        }
    }

    pub fn step_next(&mut self) {
        let mut debug = CpuDebugLog::new();
        debug.addr = Some(self.pc);
        debug.cpu_register = Some(format!("{}", self.log_str()));

        let (command, bytes) = self.fetch();

        debug.bytes = Some(bytes);

        let command_log = self.exec_command(&command);

        debug.command = Some(command_log);

        debug.log();
    }

    fn update_status_zero(&mut self, v : u8) {
        if v == 0 {
            self.p |= P_MASK_ZERO
        } else {
            self.p &= !P_MASK_ZERO
        }
    }   
    fn update_status_overflow(&mut self, v : u8) {
        if v & 0x70 != 0 {
            self.p |= P_MASK_OVERFLOW
        } else {
            self.p &= !P_MASK_OVERFLOW
        }
    }
    fn update_status_overflow_of(&mut self, b : bool) {
        if b {
            self.p |= P_MASK_OVERFLOW
        } else {
            self.p &= !P_MASK_OVERFLOW
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

// nestestのログと同じフォーマットのログを出力するためのオブジェクト
struct CpuDebugLog {
    addr : Option<u16>,
    bytes : Option<Vec<u8>>,
    command : Option<String>,
    cpu_register : Option<String>,
}

impl CpuDebugLog {
    fn new() -> CpuDebugLog {
        return CpuDebugLog {
            addr: None,
            bytes: None,
            command: None,
            cpu_register: None
        }
    }
    fn log(&self) {
        println!(
            "{:04X}  {: <9} {: <31} {}",
            self.addr.unwrap(),
            dump_bytes(&self.bytes.as_ref().unwrap()),
            self.command.as_ref().unwrap(),
            self.cpu_register.as_ref().unwrap());

    }
}