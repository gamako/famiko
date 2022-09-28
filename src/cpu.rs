use crate::bus::Bus;
use std::{fmt, str};
use crate::hex::dump_bytes;
use std::string::ToString;
use std::fmt::Write as FmtWrite;

static CPU_CLOCK_HZ : u128 = 1_789_773; // 1.789773 MHz
pub static CPU_CLOCK_UNIT_NSEC : u128 = 1_000_000_000 / CPU_CLOCK_HZ;

pub struct CPU {
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    s: u8,
    pc: u16,

    pub bus : Bus,

    cycle : usize
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

#[derive(Clone)]
enum AddressingMode {
    Accumelator,
    Imm(u8),
    ZeroPage(u8),
    ZeroPageX(u8),
    ZeroPageY(u8),
    Absolute(u16),
    AbsoluteX(u16),
    AbsoluteY(u16),
    Indirect(u8, u8),
    IndirectX(u8),
    IndirectY(u8),
    Relative(u8),
}

impl fmt::Debug for AddressingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressingMode::Accumelator => write!(f, ""),
            AddressingMode::Imm(v) => write!(f, "{:02X}", v),
            AddressingMode::ZeroPage(addr) => write!(f, "#{:02X}", addr),
            AddressingMode::ZeroPageX(addr) => write!(f, "#{:02X},x", addr),
            AddressingMode::ZeroPageY(addr) => write!(f, "#{:02X},y", addr),
            AddressingMode::Absolute(addr) => write!(f, "{:02X}", addr),
            AddressingMode::AbsoluteX(addr) => write!(f, "[{:02X} + x]", addr),
            AddressingMode::AbsoluteY(addr) => write!(f, "[{:02X} + y]", addr),
            AddressingMode::Indirect(h, l) => write!(f, "({:02X}{:02X})", h, l),
            AddressingMode::IndirectX(h) => write!(f, "({:02X} , x)", h),
            AddressingMode::IndirectY(h) => write!(f, "({:02X} , y)", h),
            AddressingMode::Relative(rel) => write!(f, "({:02X} , y)", rel),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    TAX,
    TAY,
    TSX,
    TXA,
    TXS,
    TYA,
    DEC(AddressingMode),
    DEX,
    DEY,
    INC(AddressingMode),
    INX,
    INY,
    AND(AddressingMode),
    ORA(AddressingMode),
    EOR(AddressingMode),
    ASL(AddressingMode),
    LSR(AddressingMode),
    ROL(AddressingMode),
    ROR(AddressingMode),
    ADC(AddressingMode),
    SBC(AddressingMode),
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
    RTI,
    CL(FlagType),
    SE(FlagType),
    BIT(AddressingMode),
    PHA,
    PHP,
    PLA,
    PLP,
    NOP,
    NOP_,
    DOP(AddressingMode),
    TOP(AddressingMode),
    LAX(AddressingMode),
    SAX(AddressingMode),
    SBC_(AddressingMode),
    DCP(AddressingMode),
    ISB(AddressingMode),
    SLO(AddressingMode),
    RLA(AddressingMode),
    SRE(AddressingMode),
    RRA(AddressingMode),
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
            Command::TAX => "TAX".to_string(),
            Command::TAY => "TAY".to_string(),
            Command::AND(_) => "AND".to_string(),
            Command::EOR(_) => "EOR".to_string(),
            Command::LSR(_) => "LSR".to_string(),
            Command::ADC(_) => "ADC".to_string(),
            Command::ROL(_) => "ROL".to_string(),
            Command::ROR(_) => "ROR".to_string(),
            Command::SBC(_) => "SBC".to_string(),
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
            Command::NOP_ => "*NOP".to_string(),
            Command::DOP(_) => "*NOP".to_string(),
            Command::TOP(_) => "*NOP".to_string(),
            Command::LAX(_) => "*LAX".to_string(),
            Command::SAX(_) => "*SAX".to_string(),
            Command::SBC_(_) => "*SBC".to_string(),
            Command::DCP(_) => "*DCP".to_string(),
            Command::ISB(_) => "*ISB".to_string(),
            Command::SLO(_) => "*SLO".to_string(),
            Command::RLA(_) => "*RLA".to_string(),
            Command::SRE(_) => "*SRE".to_string(),
            Command::RRA(_) => "*RRA".to_string(),
            _ => self.to_string(),
        }   
    }
}

impl CPU {
    pub fn new(bus : Bus, p: u8) -> Self {
        CPU { a: 0, x: 0, y: 0, p: p, s: 0x00, pc: 0, bus: bus, cycle: 0 }
    }

    pub fn int_reset(&mut self) -> usize {
        self.intrrupt(0xfffc)
    }

    pub fn int_nmi(&mut self) -> usize {
        self.intrrupt(0xfffa)
    }

    pub fn int_irq(&mut self) -> usize {
        self.intrrupt(0xfffa)
    }

    pub fn intrrupt(&mut self, addr: u16) -> usize {
        let l = self.bus.read(addr, false);
        let h = self.bus.read(addr+1, false);
        let handler = (h as u16) << 8 | l as u16;

        self.jmp_int_handler(handler)

    }

    pub fn jmp_int_handler(&mut self, handler: u16) -> usize {
        let sp = self.s as u16 + 0x0100;
        self.bus.write(sp, (self.pc >> 8 & 0xff) as u8);
        let sp = sp -1;
        self.bus.write(sp, (self.pc >> 0 & 0xff) as u8);
        let sp = sp -1;
        self.bus.write(sp, self.p);
        let sp = sp -1;
        self.s = (sp & 0xff) as u8;
        
        self.cycle += 7;
        self.pc = handler;
        7
    }

    pub fn init_pc(&mut self, addr : u16, cycle: usize) {
        self.pc = addr;
        self.cycle = cycle;
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

            0xaa => (Command::TAX, vec![op]),
            0xa8 => (Command::TAY, vec![op]),
            0xba => (Command::TSX, vec![op]),
            0x8a => (Command::TXA, vec![op]),
            0x9a => (Command::TXS, vec![op]),
            0x98 => (Command::TYA, vec![op]),

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

            0x0a => (Command::ASL(AddressingMode::Accumelator), vec![op]),
            0x06 => self.new_command(op, Command::ASL, Self::new_zero_page),
            0x16 => self.new_command(op, Command::ASL, Self::new_zero_page_x),
            0x0e => self.new_command(op, Command::ASL, Self::new_absolute),
            0x1e => self.new_command(op, Command::ASL, Self::new_absolute_x),

            0x4a => (Command::LSR(AddressingMode::Accumelator), vec![op]),
            0x46 => self.new_command(op, Command::LSR, Self::new_zero_page),
            0x56 => self.new_command(op, Command::LSR, Self::new_zero_page_x),
            0x4e => self.new_command(op, Command::LSR, Self::new_absolute),
            0x5e => self.new_command(op, Command::LSR, Self::new_absolute_x),

            0x2a => (Command::ROL(AddressingMode::Accumelator), vec![op]),
            0x26 => self.new_command(op, Command::ROL, Self::new_zero_page),
            0x36 => self.new_command(op, Command::ROL, Self::new_zero_page_x),
            0x2e => self.new_command(op, Command::ROL, Self::new_absolute),
            0x3e => self.new_command(op, Command::ROL, Self::new_absolute_x),

            0x6a => (Command::ROR(AddressingMode::Accumelator), vec![op]),
            0x66 => self.new_command(op, Command::ROR, Self::new_zero_page),
            0x76 => self.new_command(op, Command::ROR, Self::new_zero_page_x),
            0x6e => self.new_command(op, Command::ROR, Self::new_absolute),
            0x7e => self.new_command(op, Command::ROR, Self::new_absolute_x),

            0x61 => self.new_command(op, Command::ADC, Self::new_indirect_x),
            0x65 => self.new_command(op, Command::ADC, Self::new_zero_page),
            0x69 => self.new_command(op, Command::ADC, Self::new_imm),
            0x6d => self.new_command(op, Command::ADC, Self::new_absolute),
            0x71 => self.new_command(op, Command::ADC, Self::new_indirect_y),
            0x75 => self.new_command(op, Command::ADC, Self::new_zero_page_x),
            0x79 => self.new_command(op, Command::ADC, Self::new_absolute_y),
            0x7d => self.new_command(op, Command::ADC, Self::new_absolute_x),

            0xe1 => self.new_command(op, Command::SBC, Self::new_indirect_x),
            0xe5 => self.new_command(op, Command::SBC, Self::new_zero_page),
            0xe9 => self.new_command(op, Command::SBC, Self::new_imm),
            0xed => self.new_command(op, Command::SBC, Self::new_absolute),
            0xf1 => self.new_command(op, Command::SBC, Self::new_indirect_y),
            0xf5 => self.new_command(op, Command::SBC, Self::new_zero_page_x),
            0xf9 => self.new_command(op, Command::SBC, Self::new_absolute_y),
            0xfd => self.new_command(op, Command::SBC, Self::new_absolute_x),

            0xa2 => self.new_command(op, Command::LDX, Self::new_imm),
            0xae => self.new_command(op, Command::LDX, Self::new_absolute),
            0xa6 => self.new_command(op, Command::LDX, Self::new_zero_page),
            0xb6 => self.new_command(op, Command::LDX, Self::new_zero_page_y),
            0xbe => self.new_command(op, Command::LDX, Self::new_absolute_y),

            0xa0 => self.new_command(op, Command::LDY, Self::new_imm),
            0xa4 => self.new_command(op, Command::LDY, Self::new_zero_page),
            0xb4 => self.new_command(op, Command::LDY, Self::new_zero_page_x),
            0xac => self.new_command(op, Command::LDY, Self::new_absolute),
            0xbc => self.new_command(op, Command::LDY, Self::new_absolute_x),

            0xc1 => self.new_command(op, Command::CMP, Self::new_indirect_x),
            0xc5 => self.new_command(op, Command::CMP, Self::new_zero_page),
            0xc9 => self.new_command(op, Command::CMP, Self::new_imm),
            0xcd => self.new_command(op, Command::CMP, Self::new_absolute),
            0xd5 => self.new_command(op, Command::CMP, Self::new_zero_page_x),
            0xdd => self.new_command(op, Command::CMP, Self::new_absolute_x),
            0xd9 => self.new_command(op, Command::CMP, Self::new_absolute_y),
            0xd1 => self.new_command(op, Command::CMP, Self::new_indirect_y),
            0xe0 => self.new_command(op, Command::CPX, Self::new_imm),
            0xe4 => self.new_command(op, Command::CPX, Self::new_zero_page),
            0xec => self.new_command(op, Command::CPX, Self::new_absolute),
            0xc0 => self.new_command(op, Command::CPY, Self::new_imm),
            0xc4 => self.new_command(op, Command::CPY, Self::new_zero_page),
            0xcc => self.new_command(op, Command::CPY, Self::new_absolute),

            0xc6 => self.new_command(op, Command::DEC, Self::new_zero_page),
            0xd6 => self.new_command(op, Command::DEC, Self::new_zero_page_x),
            0xce => self.new_command(op, Command::DEC, Self::new_absolute),
            0xde => self.new_command(op, Command::DEC, Self::new_absolute_x),
            0xca => (Command::DEX, vec![op]),
            0x88 => (Command::DEY, vec![op]),
            0xe6 => self.new_command(op, Command::INC, Self::new_zero_page),
            0xf6 => self.new_command(op, Command::INC, Self::new_zero_page_x),
            0xee => self.new_command(op, Command::INC, Self::new_absolute),
            0xfe => self.new_command(op, Command::INC, Self::new_absolute_x),
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
            0x40 => (Command::RTI, vec![op]),

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
            0x1a => (Command::NOP_, vec![op]),
            0x3a => (Command::NOP_, vec![op]),
            0x5a => (Command::NOP_, vec![op]),
            0x7a => (Command::NOP_, vec![op]),
            0xda => (Command::NOP_, vec![op]),
            0xfa => (Command::NOP_, vec![op]),
            0x04 => self.new_command(op, Command::DOP, Self::new_zero_page),
            0x44 => self.new_command(op, Command::DOP, Self::new_zero_page),
            0x64 => self.new_command(op, Command::DOP, Self::new_zero_page),
            0x14 => self.new_command(op, Command::DOP, Self::new_zero_page_x),
            0x34 => self.new_command(op, Command::DOP, Self::new_zero_page_x),
            0x54 => self.new_command(op, Command::DOP, Self::new_zero_page_x),
            0x74 => self.new_command(op, Command::DOP, Self::new_zero_page_x),
            0xd4 => self.new_command(op, Command::DOP, Self::new_zero_page_x),
            0xf4 => self.new_command(op, Command::DOP, Self::new_zero_page_x),
            0x80 => self.new_command(op, Command::DOP, Self::new_imm),
            0x82 => self.new_command(op, Command::DOP, Self::new_imm),
            0x92 => self.new_command(op, Command::DOP, Self::new_imm),
            0x0c => self.new_command(op, Command::TOP, Self::new_absolute),
            0x1c => self.new_command(op, Command::TOP, Self::new_absolute_x),
            0x3c => self.new_command(op, Command::TOP, Self::new_absolute_x),
            0x5c => self.new_command(op, Command::TOP, Self::new_absolute_x),
            0x7c => self.new_command(op, Command::TOP, Self::new_absolute_x),
            0xdc => self.new_command(op, Command::TOP, Self::new_absolute_x),
            0xfc => self.new_command(op, Command::TOP, Self::new_absolute_x),

            0xa7 => self.new_command(op, Command::LAX, Self::new_zero_page),
            0xb7 => self.new_command(op, Command::LAX, Self::new_zero_page_y),
            0xaf => self.new_command(op, Command::LAX, Self::new_absolute),
            0xbf => self.new_command(op, Command::LAX, Self::new_absolute_y),
            0xa3 => self.new_command(op, Command::LAX, Self::new_indirect_x),
            0xb3 => self.new_command(op, Command::LAX, Self::new_indirect_y),

            0x87 => self.new_command(op, Command::SAX, Self::new_zero_page),
            0x97 => self.new_command(op, Command::SAX, Self::new_zero_page_y),
            0x83 => self.new_command(op, Command::SAX, Self::new_indirect_x),
            0x8f => self.new_command(op, Command::SAX, Self::new_absolute),

            0xeb => self.new_command(op, Command::SBC_, Self::new_imm),
            
            0xc7 => self.new_command(op, Command::DCP, Self::new_zero_page),
            0xd7 => self.new_command(op, Command::DCP, Self::new_zero_page_x),
            0xcf => self.new_command(op, Command::DCP, Self::new_absolute),
            0xdf => self.new_command(op, Command::DCP, Self::new_absolute_x),
            0xdb => self.new_command(op, Command::DCP, Self::new_absolute_y),
            0xc3 => self.new_command(op, Command::DCP, Self::new_indirect_x),
            0xd3 => self.new_command(op, Command::DCP, Self::new_indirect_y),

            0xe7 => self.new_command(op, Command::ISB, Self::new_zero_page),
            0xf7 => self.new_command(op, Command::ISB, Self::new_zero_page_x),
            0xef => self.new_command(op, Command::ISB, Self::new_absolute),
            0xff => self.new_command(op, Command::ISB, Self::new_absolute_x),
            0xfb => self.new_command(op, Command::ISB, Self::new_absolute_y),
            0xe3 => self.new_command(op, Command::ISB, Self::new_indirect_x),
            0xf3 => self.new_command(op, Command::ISB, Self::new_indirect_y),

            0x07 => self.new_command(op, Command::SLO, Self::new_zero_page),
            0x17 => self.new_command(op, Command::SLO, Self::new_zero_page_x),
            0x0f => self.new_command(op, Command::SLO, Self::new_absolute),
            0x1f => self.new_command(op, Command::SLO, Self::new_absolute_x),
            0x1b => self.new_command(op, Command::SLO, Self::new_absolute_y),
            0x03 => self.new_command(op, Command::SLO, Self::new_indirect_x),
            0x13 => self.new_command(op, Command::SLO, Self::new_indirect_y),

            0x27 => self.new_command(op, Command::RLA, Self::new_zero_page),
            0x37 => self.new_command(op, Command::RLA, Self::new_zero_page_x),
            0x2f => self.new_command(op, Command::RLA, Self::new_absolute),
            0x3f => self.new_command(op, Command::RLA, Self::new_absolute_x),
            0x3b => self.new_command(op, Command::RLA, Self::new_absolute_y),
            0x23 => self.new_command(op, Command::RLA, Self::new_indirect_x),
            0x33 => self.new_command(op, Command::RLA, Self::new_indirect_y),

            0x47 => self.new_command(op, Command::SRE, Self::new_zero_page),
            0x57 => self.new_command(op, Command::SRE, Self::new_zero_page_x),
            0x4f => self.new_command(op, Command::SRE, Self::new_absolute),
            0x5f => self.new_command(op, Command::SRE, Self::new_absolute_x),
            0x5b => self.new_command(op, Command::SRE, Self::new_absolute_y),
            0x43 => self.new_command(op, Command::SRE, Self::new_indirect_x),
            0x53 => self.new_command(op, Command::SRE, Self::new_indirect_y),

            0x67 => self.new_command(op, Command::RRA, Self::new_zero_page),
            0x77 => self.new_command(op, Command::RRA, Self::new_zero_page_x),
            0x6f => self.new_command(op, Command::RRA, Self::new_absolute),
            0x7f => self.new_command(op, Command::RRA, Self::new_absolute_x),
            0x7b => self.new_command(op, Command::RRA, Self::new_absolute_y),
            0x63 => self.new_command(op, Command::RRA, Self::new_indirect_x),
            0x73 => self.new_command(op, Command::RRA, Self::new_indirect_y),

            _ => {
                println!("not impl {:#02x}", op);
                panic!("not impl error");
            }
        }
    }
    
    fn exec_branch<F : Fn(u8) -> bool>(&mut self, cond : F, addr : &AddressingMode, l: &mut String) -> usize {
        match addr {
            AddressingMode::Relative(a) => {
                let mut cycle = 0;
                let addr = self.pc.wrapping_add(*a as i8 as u16);
                write!(l, "${:04X}", addr).unwrap();
                if cond(self.p) {
                    cycle += 1;
                    if self.pc.page() != addr.page() {
                        cycle += 1;
                    }
                    self.pc = addr;
                }
                cycle
            },
            _ => { panic!("branch addressing mode error") }
        }
    }

    fn exec_command(&mut self, command: &Command) -> (String, usize, CommandLog) {
        let mut l = String::new();
        let mut command_log = CommandLog::SE(FlagType::IntDisable);

        write!(l, "{:>4} ", command.type_name()).unwrap();
        let cycle : usize = match command {
            Command::STA(a) => { self.store(a, self.a, Some(&mut l)) },
            Command::STX(a) => { self.store(a, self.x, Some(&mut l)) },
            Command::STY(a) => { self.store(a, self.y, Some(&mut l)) },
            Command::LDA(a) => {
                let (v, _, cycle) = self.load(a, &mut l);
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
                command_log = CommandLog::LDA(LogAddressingMode::new(a, &self));
                cycle
            },
            Command::LDX(a) => {
                let (v, _, cycle) = self.load(a, &mut l);
                self.x = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
                cycle
            },
            Command::LDY(a) => {
                let (v, _, cycle) = self.load(a, &mut l);
                self.y = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
                cycle
            },
            Command::TSX => {
                self.x = self.s;
                self.update_status_zero(self.x);
                self.update_status_negative(self.x);
                1
            },
            Command::TAX => {
                self.x = self.a;
                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
                1
            },
            Command::TAY => {
                self.y = self.a;
                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
                1
            },
            Command::TXA => {
                self.a = self.x;
                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
                1
            },
            Command::TXS => {self.s = self.x; 1},
            Command::TYA => {
                self.a = self.y;
                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
                1
            },
            
            Command::AND(a) => {
                let (v, _, cycle) = self.load(a, &mut l);
                let v = v & self.a;
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
                cycle
            },
            Command::ORA(a) => {
                let (v, _, cycle) = self.load(a, &mut l);
                let v = v | self.a;
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
                cycle
            },
            Command::EOR(a) => {
                let (v, _, cycle) = self.load(a, &mut l);
                let v = v ^ self.a;
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
                cycle
            },
            Command::ASL(a) => {
                let (v, addr1, load_cycle) = self.load_(a, &mut l, true);
                self.update_status_carry(v & 0x80 != 0);
                let v = v.wrapping_shl(1);
                let store_cycle = self.store(&addr1, v, None);
                self.update_status_zero(v);
                self.update_status_negative(v);
                load_cycle + store_cycle + 1
            },
            Command::LSR(a) => {
                let (v, addr1, load_cycle) = self.load_(a, &mut l, true);
                self.update_status_carry(v & 0x01 != 0);
                let v = v.wrapping_shr(1);
                let store_cycle = self.store(&addr1, v, None);
                self.update_status_zero(v);
                self.update_status_negative(v);
                load_cycle + store_cycle + 1
            },
            Command::ROL(a) => {
                let (v0, addr1, load_cycle) = self.load_(a, &mut l, true);
                let v1 = v0.wrapping_shl(1) | (self.p & 0x01);
                let store_cycle = self.store(&addr1, v1, None);
                self.update_status_carry(v0 & 0x80 != 0);
                self.update_status_zero(v1);
                self.update_status_negative(v1);
                load_cycle + store_cycle + 1
            },
            Command::ROR(a) => {
                let (v0, addr1, load_cycle) = self.load_(a, &mut l, true);
                let v1 = v0.wrapping_shr(1) | ((self.p & 0x01) << 7);
                let store_cycle = self.store(&addr1, v1, None);
                self.update_status_carry(v0 & 0x01 != 0);
                self.update_status_zero(v1);
                self.update_status_negative(v1);
                load_cycle + store_cycle + 1
            },
            Command::ADC(addr) => {
                let a = self.a;
                let (b, _, cycle) = self.load(addr, &mut l);
                let c = self.p & P_MASK_CARRY;
                let d = a  as u16 + b  as u16 + c  as u16;
                self.a = (d & 0xff) as u8;

                self.update_status_carry(d > 0xff);
                self.update_status_overflow_of((a ^ b) & 0x80 == 0 && (self.a ^ a) & 0x80 != 0);

                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
                cycle
            },
            Command::SBC(addr) => {
                let a = self.a;
                let (b, _, cycle) = self.load(addr, &mut l);
                let c = self.p & P_MASK_CARRY;
                let d = (a as u16).wrapping_sub(b  as u16).wrapping_sub((1 - c) as u16);
                self.a = (d & 0xff) as u8;

                self.update_status_carry(!d > 0xff);
                self.update_status_overflow_of((a ^ b) & 0x80 != 0 && (self.a ^ a) & 0x80 != 0);

                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
                cycle
            },
            Command::DEC(a) => {
                let (v0, addr1, load_cycle) = self.load_(a, &mut l, true);
                let v1 = v0.wrapping_sub(1);
                let store_cycle = self.store(&addr1, v1, None);
                self.update_status_zero(v1);
                self.update_status_negative(v1);
                load_cycle + store_cycle + 1
            }
            Command::DEX => {
                self.x = self.x.wrapping_sub(1u8);
                self.update_status_zero(self.x);
                self.update_status_negative(self.x);
                1
            },
            Command::DEY => {
                self.y = self.y.wrapping_sub(1u8);
                self.update_status_zero(self.y);
                self.update_status_negative(self.y);
                1
            },
            Command::INC(a) => {
                let (v0, addr1, load_cycle) = self.load_(a, &mut l, true);
                let v1 = v0.wrapping_add(1);
                let store_cycle = self.store(&addr1, v1, None);
                self.update_status_zero(v1);
                self.update_status_negative(v1);
                load_cycle + store_cycle + 1
            }
            Command::INX => {
                self.x = self.x.wrapping_add(1u8);
                self.update_status_zero(self.x);
                self.update_status_negative(self.x);
                1
            },
            Command::INY => {
                self.y = self.y.wrapping_add(1u8);
                self.update_status_zero(self.y);
                self.update_status_negative(self.y);
                1
            },
            Command::CMP(a) => {
                let (m, _, cycle) = self.load(a, &mut l);
                let v = self.a.wrapping_sub(m);
                self.update_status_carry(self.a >= m);
                self.update_status_zero(v);
                self.update_status_negative(v);
                cycle
            }
            Command::CPX(a) => {
                let (m, _, cycle) = self.load(a, &mut l);
                let (v, _b) = self.x.overflowing_sub(m);
                self.update_status_carry(self.x >= m);
                self.update_status_zero(v);
                self.update_status_negative(v);
                cycle
            }
            Command::CPY(a) => {
                let (m, _, cycle) = self.load(a, &mut l);
                let (v, _) = self.y.overflowing_sub(m);
                self.update_status_carry(self.y >= m);
                self.update_status_zero(v);
                self.update_status_negative(v);
                cycle
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
                self.pc = *addr;
                0
            }
            Command::JMP(AddressingMode::Indirect(a_h, a_l)) => {
                let addr1 = self.read_word_in_page(*a_h, *a_l);
                write!(l, "(${:02X}{:02X}) = {:04X}", *a_h, *a_l, addr1).unwrap();
                self.pc = addr1;
                2
            },
            Command::JSR(AddressingMode::Absolute(addr)) => {
                write!(l, "${:04X}", addr).unwrap();
                self.push_stack_word(self.pc-1);
                self.pc = *addr;
                3
            }
            Command::RTS => {
                self.pc = self.pop_stack_word()+1;
                5
            }
            Command::RTI => {
                self.p = self.pop_stack() | 0x20u8;
                self.pc = self.pop_stack_word();
                5
            }
            Command::CL(f) => {
                self.p &= !f.mask(); 
                command_log = CommandLog::CL(f.clone());
                1
            },
            Command::SE(f) => {
                self.p |= f.mask(); 
                command_log = CommandLog::SE(f.clone());
                1
            },
            Command::BIT(a) => {
                let (m, _, cycle) = self.load(a, &mut l);
                let r = m & self.a;
                self.update_status_zero(r);
                self.update_status_overflow(m);
                self.update_status_negative(m);
                cycle
            }
            Command::PHA => {
                self.bus.write(self.s as u16 + 0x0100, self.a);
                self.s -= 1;
                2
            },
            Command::PHP => {
                let v = self.p | P_MASK_BREAK_COMMAND;
                self.bus.write(self.s as u16 + 0x0100, v);
                self.s -= 1;
                2
            },
            Command::PLP => {
                self.s += 1;
                let v = self.bus.read(self.s as u16 + 0x0100, false);
                self.p = (self.p & 0x30) | (v & 0xcf);
                3
            },
            Command::PLA => {
                self.s += 1;
                let v = self.bus.read(self.s as u16 + 0x0100, false);
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
                3
            },
            Command::NOP => 1,
            Command::NOP_ => 1,
            Command::DOP(a) => {
                let (_, _, cycle) = self.load(a, &mut l);
                cycle
            }
            Command::TOP(a) => {
                let (_, _, cycle) = self.load(a, &mut l);
                cycle
            }
            Command::LAX(a) => {
                let (v, _, cycle) = self.load(a, &mut l);
                self.x = v;
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
                cycle
            },
            Command::SAX(a) => {
                let v1 = self.a & self.x;
                self.store(a, v1, Some(&mut l))
            },
            Command::SBC_(addr) => {
                let a = self.a;
                let (b, _, cycle) = self.load(addr, &mut l);
                let c = self.p & P_MASK_CARRY;
                let d = (a as u16).wrapping_sub(b  as u16).wrapping_sub((1 - c) as u16);
                self.a = (d & 0xff) as u8;

                self.update_status_carry(!d > 0xff);
                self.update_status_overflow_of((a ^ b) & 0x80 != 0 && (self.a ^ a) & 0x80 != 0);

                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
                cycle
            },
            Command::DCP(a) => {
                let (m, addr1, load_cycle) = self.load(a, &mut l);
                let m = m.wrapping_sub(1);
                let store_cycle = self.store(&addr1, m, None);
                let (v, _) = self.a.overflowing_sub(m);
                self.update_status_carry(self.a >= m);
                self.update_status_zero(v);
                self.update_status_negative(v);
                load_cycle + store_cycle + 1
            }
            Command::ISB(addr) => {
                let a = self.a;
                let (b, addr1, load_cycle) = self.load(addr, &mut l);
                let b = b.wrapping_add(1);
                let store_cycle = self.store(&addr1, b, None);
                let c = self.p & P_MASK_CARRY;
                let d = (a as u16).wrapping_sub(b  as u16).wrapping_sub((1 - c) as u16);
                self.a = (d & 0xff) as u8;

                self.update_status_carry(!d > 0xff);
                self.update_status_overflow_of((a ^ b) & 0x80 != 0 && (self.a ^ a) & 0x80 != 0);

                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
                load_cycle + store_cycle + 1
            }
            Command::SLO(addr) => {
                let (v, addr1, load_cycle) = self.load(addr, &mut l);
                self.update_status_carry(v & 0x80 != 0);
                let v = v.wrapping_shl(1);
                let store_cycle = self.store(&addr1, v, None);

                self.a = v | self.a;
                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
                load_cycle + store_cycle + 1
            }
            Command::RLA(addr) => {
                let (v0,addr1,  load_cycle) = self.load(addr, &mut l);
                let v1 = v0.wrapping_shl(1) | (self.p & 0x01);
                let store_cycle = self.store(&addr1, v1, None);
                self.update_status_carry(v0 & 0x80 != 0);

                self.a = v1 & self.a;
                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
                load_cycle + store_cycle + 1
            }
            Command::SRE(addr) => {
                let (v, addr1, load_cycle) = self.load(addr, &mut l);
                self.update_status_carry(v & 0x01 != 0);
                let v = v.wrapping_shr(1);
                let store_cycle = self.store(&addr1, v, None);

                self.a = v ^ self.a;
                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
                load_cycle + store_cycle + 1
            }
            Command::RRA(addr) => {
                let (v0, addr1, load_cycle) = self.load(addr, &mut l);
                let v1 = v0.wrapping_shr(1) | ((self.p & 0x01) << 7);
                let store_cycle = self.store(&addr1, v1, None);
                self.update_status_carry(v0 & 0x01 != 0);

                let a = self.a;
                let b = v1;
                let c = self.p & P_MASK_CARRY;
                let d = a  as u16 + b  as u16 + c  as u16;
                self.a = (d & 0xff) as u8;

                self.update_status_carry(d > 0xff);
                self.update_status_overflow_of((a ^ b) & 0x80 == 0 && (self.a ^ a) & 0x80 != 0);

                self.update_status_zero(self.a);
                self.update_status_negative(self.a);
                load_cycle + store_cycle + 1
            }
            _ => { panic!("xx") }
        };
        return (l, cycle, command_log);
    }

    fn read_byte(&mut self, addr: u16, is_debug: bool) -> u8 {
        self.bus.read(addr, is_debug)
    }

    fn read_byte_pc(&mut self) -> u8 {
        let v = self.read_byte(self.pc, false);
        self.pc += 1;
        v
    }

    #[allow(dead_code)]
    fn read_word(&mut self, addr: u16) -> u16 {
        let l = self.read_byte(addr, false);
        let h = self.read_byte(addr + 1, false);
        (h as u16) << 8 | l as u16
    }
    fn read_word_in_page(&mut self, addr_h: u8, addr_l: u8) -> u16 {
        let addr_h = (addr_h as u16) << 8;
        let addr_l = addr_l as u16;
        let l = self.read_byte(addr_h | addr_l, false);
        let h = self.read_byte(addr_h | ((addr_l + 1) & 0xffu16), false);
        (h as u16) << 8 | l as u16
    }
    fn read_word_zeropage(&mut self, addr: u8) -> u16 {
        let l = self.read_byte(addr as u16, false);
        let h = self.read_byte(addr.wrapping_add(1) as u16, false);
        (h as u16) << 8 | l as u16
    }

    #[allow(dead_code)]
    fn read_word_pc(&mut self) -> u16 {
        let v = self.read_word(self.pc);
        self.pc += 2;
        v
    }

    fn write_byte(&mut self, addr: u16, v: u8) {
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
        self.read_byte(addr, false)
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
    fn new_addr_and_u8_2<F: FnOnce(u8, u8) -> AddressingMode>(&mut self, f : F) -> (AddressingMode, Vec<u8>) {
        let l = self.read_byte_pc();
        let h = self.read_byte_pc();
        (f(h, l), vec![l, h])
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
        self.new_addr_and_u8(AddressingMode::ZeroPageX)
    }

    fn new_zero_page_y(&mut self) -> (AddressingMode, Vec<u8>) {
        self.new_addr_and_u8(AddressingMode::ZeroPageY)
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
        self.new_addr_and_u8_2(AddressingMode::Indirect)
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

    fn load(&mut self, addr_mode: &AddressingMode, l: &mut String) -> (u8, AddressingMode, usize) {
        self.load_(addr_mode, l, false)
    }

    fn load_(&mut self, addr_mode: &AddressingMode, l: &mut String, is_store: bool) -> (u8, AddressingMode, usize) {
        match *addr_mode {
            AddressingMode::Accumelator => {
                write!(l, "A").unwrap();
                (self.a, AddressingMode::Accumelator, 0)
            },
            AddressingMode::Imm(v) => {
                write!(l, "#${:02X}", v).unwrap();
                (v, AddressingMode::Accumelator, 0)
            }
            AddressingMode::ZeroPage(addr) => {
                let addr = addr as u16;
                let v = self.read_byte(addr, false);
                write!(l, "${:02X} = {:02X}", addr, v).unwrap();
                (v, AddressingMode::Absolute(addr), 1)
            }
            AddressingMode::ZeroPageX(addr) => {
                let addr1 = addr.wrapping_add(self.x) as u16;
                let v = self.read_byte(addr1, false);
                write!(l, "${:02X},X @ {:02X} = {:02X}", addr, addr1, v).unwrap();
                (v, AddressingMode::Absolute(addr1), 2)
            },
            AddressingMode::ZeroPageY(addr) => {
                let addr1 = addr.wrapping_add(self.y) as u16;
                let v = self.read_byte(addr1, false);
                write!(l, "${:02X},Y @ {:02X} = {:02X}", addr, addr1, v).unwrap();
                (v, AddressingMode::Absolute(addr1), 2)
            },
            AddressingMode::Absolute(addr) => {
                let v = self.read_byte(addr, false);
                write!(l, "${:04X} = {:02X}", addr, v).unwrap();
                (v, AddressingMode::Absolute(addr), 1)
            },
            AddressingMode::AbsoluteX(addr) => {
                let addr1 = addr.wrapping_add(self.x as u16);
                let v = self.read_byte(addr1, false);
                write!(l, "${:04X},X @ {:04X} = {:02X}", addr, addr1, v).unwrap();
                
                (v, AddressingMode::Absolute(addr1), if addr.page() == addr1.page() && !is_store {1} else {2})
            },
            AddressingMode::AbsoluteY(addr) => {
                let addr1 = addr.wrapping_add(self.y as u16);
                let v = self.read_byte(addr1, false);
                write!(l, "${:04X},Y @ {:04X} = {:02X}", addr, addr1, v).unwrap();
                (v, AddressingMode::Absolute(addr1), if addr.page() == addr1.page() && !is_store {1} else {2})
            },
            AddressingMode::Indirect(_h, _l) => panic!("load indirect"),
            AddressingMode::IndirectX(m) => {
                let addr = m.wrapping_add(self.x);
                let addr1 = self.read_word_zeropage(addr);
                let v = self.read_byte(addr1, false);
                write!(l, "(${:02X},X) @ {:02X} = {:04X} = {:02X}", m, addr, addr1, v).unwrap();
                (v, AddressingMode::Absolute(addr1), 4)
            },
            AddressingMode::IndirectY(m) => {
                let addr0 = self.read_word_zeropage(m);
                let addr1 = addr0.wrapping_add(self.y as u16);
                let v = self.read_byte(addr1, false);
                write!(l, "(${:02X}),Y = {:04X} @ {:04X} = {:02X}", m, addr0, addr1, v).unwrap();
                
                (v, AddressingMode::Absolute(addr1), if addr0.page() == addr1.page() {3} else {4})

            },
            AddressingMode::Relative(_) => panic!("load rel"),
        }
    }

    fn store(&mut self, addr_mode: &AddressingMode, v : u8, l: Option<&mut String>) -> usize {
        match *addr_mode {
            AddressingMode::Accumelator => { self.a = v; 0 },
            AddressingMode::Imm(_) => { self.a = v; 0 },
            AddressingMode::ZeroPage(addr) => {
                let old = self.read_byte(addr as u16, true);
                if let Some(l) = l {
                    write!(l, "${:02X} = {:02X}", addr, old).unwrap();
                }
                self.write_byte(addr as u16, v);
                1
            }
            AddressingMode::ZeroPageX(addr) => {
                let addr1 = addr.wrapping_add(self.x) as u16;
                let old_v = self.read_byte(addr1, true);
                self.write_byte(addr1, v);
                if let Some(l) = l {
                    write!(l, "${:02X},X @ {:02X} = {:02X}", addr, addr1, old_v).unwrap();
                }
                2
            },
            AddressingMode::ZeroPageY(addr) => {
                let addr1 = addr.wrapping_add(self.y) as u16;
                let old_v = self.read_byte(addr1, true);
                self.write_byte(addr1, v);
                if let Some(l) = l {
                    write!(l, "${:02X},Y @ {:02X} = {:02X}", addr, addr1, old_v).unwrap();
                }
                2
            },
            AddressingMode::Absolute(addr) => {
                let old = self.read_byte(addr as u16, true);
                self.write_byte(addr, v);
                if let Some(l) = l {
                    write!(l, "${:04X} = {:02X}", addr, old).unwrap();
                }
                1
            },
            AddressingMode::AbsoluteX(addr) => {
                let addr1 = addr.wrapping_add(self.x as u16);
                let old_v = self.read_byte(addr1, true);
                if let Some(l) = l {
                    write!(l, "${:04X},X @ {:04X} = {:02X}", addr, addr1, old_v).unwrap();
                }
                self.write_byte(addr1, v);
                2
            },
            AddressingMode::AbsoluteY(addr) => {
                let addr1 = addr.wrapping_add(self.y as u16);
                let old_v = self.read_byte(addr1, true);
                if let Some(l) = l {
                    write!(l, "${:04X},Y @ {:04X} = {:02X}", addr, addr1, old_v).unwrap();
                }
                self.write_byte(addr1, v);
                2
            },
            AddressingMode::Indirect(_h, _l) => panic!("store indirect"),
            AddressingMode::IndirectX(m) => {
                let addr = m.wrapping_add(self.x);
                let addr1 = self.read_word_zeropage(addr);
                let old_v = self.read_byte(addr1, true);
                if let Some(l) = l {
                    write!(l, "(${:02X},X) @ {:02X} = {:04X} = {:02X}", m, addr, addr1, old_v).unwrap();
                }

                self.write_byte(addr1, v);
                4
            },
            AddressingMode::IndirectY(m) => {
                let addr0 = self.read_word_zeropage(m);
                let addr1 = addr0.wrapping_add(self.y as u16);
                let old_v = self.read_byte(addr1, true);
                if let Some(l) = l {
                    write!(l, "(${:02X}),Y = {:04X} @ {:04X} = {:02X}", m, addr0, addr1, old_v).unwrap();
                }
                self.write_byte(addr1, v);
                4
            },
            AddressingMode::Relative(_) => panic!("store rel"),
        }
    }

    pub fn step_next(&mut self, log : &mut CpuDebugLog, fceux_log: &mut Option<FceuxLog>) -> usize {
        if self.bus.read_nmi() {
            //println!("interruption nmi");
            return self.int_nmi();
        }
        if self.bus.read_irq() & (self.p & P_MASK_INT_DISABLE == 0) {
            self.p &= !P_MASK_BREAK_COMMAND;
            return self.int_irq();
        }
        if let Some(l) = fceux_log {
            l.cpu = Some(CpuState::from_cpu(&self))
        }

        log.addr = Some(self.pc);
        log.cpu_register = Some(format!("{}", self.log_str()));
        log.cpu_cycle = self.cycle;

        let (command, bytes) = self.fetch();
        let fetch_cycle = bytes.len();
        log.bytes = Some(bytes.clone());

        let (command_log_str, exec_cycle, command_log) = self.exec_command(&command);

        if let Some(l) = fceux_log {
            l.mem = Some(bytes);
            l.command_log = Some(command_log);
        }

        let cycle = exec_cycle + fetch_cycle;
        self.cycle += cycle;
        log.command = Some(command_log_str);
        cycle
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
pub struct CpuDebugLog {
    pub addr : Option<u16>,
    pub bytes : Option<Vec<u8>>,
    pub command : Option<String>,
    pub cpu_register : Option<String>,
    pub ppu_line: usize,
    pub ppu_x: usize,
    pub cpu_cycle: usize,
}

impl CpuDebugLog {
    pub fn new() -> CpuDebugLog {
        return CpuDebugLog {
            addr: None,
            bytes: None,
            command: None,
            cpu_register: None,
            ppu_line: 0,
            ppu_x: 0,
            cpu_cycle: 0,
        }
    }
    pub fn log(&self) {
        if let None = self.addr {
            return;
        }
        println!(
            "{:04X}  {: <9}{: <32} {} PPU:{: >3},{: >3} CYC:{}",
            self.addr.unwrap(),
            dump_bytes(&self.bytes.as_ref().unwrap()),
            self.command.as_ref().unwrap(),
            self.cpu_register.as_ref().unwrap(),
            self.ppu_line,
            self.ppu_x,
            self.cpu_cycle
        );

    }
}

#[derive(Debug, Copy, Clone)]
pub struct CpuState {
    a : u8,
    x : u8,
    y : u8,
    s : u8,
    p : u8,
    pc : u16,
}

impl CpuState {
    fn from_cpu(cpu : &CPU) -> Self{
        Self {
            a : cpu.a,
            x : cpu.x,
            y : cpu.y,
            s : cpu.s,
            p : cpu.p,
            pc : cpu.pc,
        }
    }
}

pub struct FceuxLog {
    frame_num : u64,
    cpu : Option<CpuState>,
    mem : Option<Vec<u8>>,
    command_log : Option<CommandLog>,
    // f1      A:10 X:FF Y:00 S:FF P:nvubdIzc  $800A: AD 02 20 LDA $2002 PPU_STATUS = #$10
}

impl FceuxLog {
    pub fn new(frame_num : u64) -> Self {
        Self {
            frame_num: frame_num,
            cpu: None,
            mem: None,
            command_log: None,
        }
    }

    pub fn log_str(&self) -> String {
        let cpu = self.cpu.unwrap();
        
        let command_str = self.command_log.as_ref().unwrap().fceux_log_str();
        let str = format!(
            "f{: <6} A:{:02X} X:{:02X} Y:{:02X} S:{:02X} P:{}  ${:04X}: {: <8} {:}",
            self.frame_num,
            cpu.a,
            cpu.x,
            cpu.y,
            cpu.s,
            fceux_flag_str(cpu.p),
            cpu.pc,
            dump_bytes(&self.mem.as_ref().unwrap()),
            command_str,
        );
        str
    }

}

fn fceux_flag_str(p:u8) -> String {
    let table = &['n','v','u','b','d','i','z','c'];

    let mut buf : [u8; 8] = [0u8; 8];
    for i in 0..8usize {
        let c = table[i] as u8;
        let c = if p >> (7-i) & 1 != 0{
            c - 0x20
        } else {
            c 
        };
        buf[i] = c;
    }
    str::from_utf8(&buf).unwrap().to_string()
}

#[cfg(test)]
mod fceux_flag_str_tests {
    use super::*;

    #[test]
    fn フラグテスト() {
        assert_eq!(fceux_flag_str(0b00000000), "nvubdizc");
        assert_eq!(fceux_flag_str(0b11111111), "NVUBDIZC");
        assert_eq!(fceux_flag_str(0b00000001), "nvubdizC");
        assert_eq!(fceux_flag_str(0b00000010), "nvubdiZc");
        assert_eq!(fceux_flag_str(0b00000100), "nvubdIzc");
        assert_eq!(fceux_flag_str(0b00001000), "nvubDizc");
        assert_eq!(fceux_flag_str(0b00010000), "nvuBdizc");
        assert_eq!(fceux_flag_str(0b00100000), "nvUbdizc");
        assert_eq!(fceux_flag_str(0b01000000), "nVubdizc");
        assert_eq!(fceux_flag_str(0b01000000), "Nvubdizc");
    }
}

trait Address {
    fn page(&self) -> u8;
}
impl Address for u16 {
    fn page(&self) -> u8 {
        (*self >> 8) as u8
    }
}

#[derive(Debug, Clone)]
enum CommandLog{
    LDA(LogAddressingMode),
    CL(FlagType),
    SE(FlagType),

}

impl CommandLog {
    fn fceux_log_str(&self) -> String {
        match self {
            CommandLog::LDA(a) => format!("LDA {}", a.fceux_log_str()),
            CommandLog::CL(t) =>
                match t {
                    FlagType::Carry => "CLC".to_string(),
                    FlagType::IntDisable => "CLI".to_string(),
                    FlagType::Decimal => "CLD".to_string(),
                    FlagType::Overflow => "CLV".to_string(),
                    _ => "CL?".to_string(),
                },
            CommandLog::SE(t) =>
                match t {
                    FlagType::Carry => "SEC".to_string(),
                    FlagType::IntDisable => "SEI".to_string(),
                    FlagType::Decimal => "SED".to_string(),
                    _ => "SE?".to_string(),
                },
        }   
    }
}

#[derive(Debug, Clone)]
enum LogAddressingMode {
    Accumelator,
    Imm(u8),
}

impl LogAddressingMode {
    fn new(a: &AddressingMode, cpu: &CPU) -> Self {
        match *a {
            AddressingMode::Accumelator => {
                Self::Accumelator
            },
            AddressingMode::Imm(v) => {
                Self::Imm(v)
            }
            _ => panic!("not match"),
        }
    }

    fn fceux_log_str(&self) -> String {
        match self {
            LogAddressingMode::Accumelator => {
                "Self::Accumelator".to_string()
            },
            LogAddressingMode::Imm(v) => {
                format!("#${:02X}", v)
            }
        }
    } 
}