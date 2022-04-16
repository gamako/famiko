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
static P_MASK_DECIMAL_MODE : u8 = 1 << 3;
static P_MASK_BREAK_COMMAND : u8 = 1 << 4;
static P_MASK_OVERFLOW : u8 = 1 << 5;
static P_MASK_NEGATIVE : u8 = 1 << 6;

#[derive(Debug)]
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

impl AddressingMode {
    fn new_imm(cpu: &mut CPU) -> Self {
        let v = cpu.bus.read(cpu.pc);
        cpu.pc += 1;
        AddressingMode::Imm(v)
    }

    fn new_zero_page(cpu: &mut CPU) -> Self {
        AddressingMode::ZeroPage(cpu.read_byte())
    }

    fn new_zero_page_x(cpu: &mut CPU) -> Self {
        AddressingMode::ZeroPage(cpu.read_byte())
    }

    fn new_zero_page_y(cpu: &mut CPU) -> Self {
        AddressingMode::ZeroPage(cpu.read_byte())
    }

    fn new_absolute(cpu: &mut CPU) -> Self {
        AddressingMode::Absolute(cpu.read_word())
    }

    fn new_absolute_x(cpu: &mut CPU) -> Self {
        AddressingMode::AbsoluteX(cpu.read_word())
    }

    fn new_absolute_y(cpu: &mut CPU) -> Self {
        AddressingMode::AbsoluteY(cpu.read_word())
    }

    fn new_indirect_x(cpu: &mut CPU) -> Self {
        AddressingMode::IndirectX(cpu.read_byte())
    }

    fn new_indirect_y(cpu: &mut CPU) -> Self {
        AddressingMode::IndirectY(cpu.read_byte())
    }

    fn load(&self, cpu: &mut CPU) -> u8 {
        match self {
            AddressingMode::Imm(v) => *v,
            AddressingMode::ZeroPage(addr) => cpu.bus.read(*addr as u16),
            AddressingMode::ZeroPageX(addr) => cpu.bus.read(*addr as u16 + cpu.x as u16),
            AddressingMode::ZeroPageY(addr) => cpu.bus.read(*addr as u16 + cpu.y as u16),
            AddressingMode::Absolute(addr) => cpu.bus.read(*addr),
            AddressingMode::AbsoluteX(addr) => cpu.bus.read(*addr + cpu.x as u16),
            AddressingMode::AbsoluteY(addr) => cpu.bus.read(*addr + cpu.y as u16),
            AddressingMode::IndirectX(h) => cpu.bus.read((*h as u16) << 8 + cpu.x as u16),
            AddressingMode::IndirectY(h) => cpu.bus.read((*h as u16) << 8 + cpu.y as u16),
        }
    }
    fn store(&self, cpu: &mut CPU, v : u8) {
        match self {
            AddressingMode::Imm(_) => { panic!("store imm error"); },
            AddressingMode::ZeroPage(addr) => cpu.bus.write(*addr as u16, v),
            AddressingMode::ZeroPageX(addr) => cpu.bus.write(*addr as u16 + cpu.x as u16, v),
            AddressingMode::ZeroPageY(addr) => cpu.bus.write(*addr as u16 + cpu.y as u16, v),
            AddressingMode::Absolute(addr) => cpu.bus.write(*addr, v),
            AddressingMode::AbsoluteX(addr) => cpu.bus.write(*addr + cpu.x as u16, v),
            AddressingMode::AbsoluteY(addr) => cpu.bus.write(*addr + cpu.y as u16, v),
            AddressingMode::IndirectX(h) => cpu.bus.write((*h as u16) << 8 + cpu.x as u16, v),
            AddressingMode::IndirectY(h) => cpu.bus.write((*h as u16) << 8 + cpu.y as u16, v),
        }
    }

    #[allow(unused)]
    fn desc(&self) -> String {
        match self {
            AddressingMode::Imm(v) => format!("{:#02x}", v),
            AddressingMode::ZeroPage(addr) => format!("#{:#02x}", addr),
            AddressingMode::ZeroPageX(addr) => format!("#{:#02x},x", addr),
            AddressingMode::ZeroPageY(addr) => format!("#{:#02x},y", addr),
            AddressingMode::Absolute(addr) => format!("[{:#02x}]", addr),
            AddressingMode::AbsoluteX(addr) => format!("[{:#02x} + x]", addr),
            AddressingMode::AbsoluteY(addr) => format!("[{:#02x} + y]", addr),
            AddressingMode::IndirectX(h) => format!("({:#02x} , x)", h),
            AddressingMode::IndirectY(h) => format!("({:#02x} , y)", h),
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

#[derive(Debug)]
enum Command {
    STA(AddressingMode),
    LDA(AddressingMode),
    LDX(AddressingMode),
    LDY(AddressingMode),
    TXS,
    DEY,
    INX,
    BPL(i8),
    BNE(i8),
    JMPAbs(u16),
    CL(FlagType),
    SE(FlagType),
    PLP,
}

impl Command {
    #[allow(unused)]
    fn desc(&self) -> String {
        match self {
            Command::STA(a) => format!("STA {}", a.desc()),
            Command::LDA(a) => format!("LDA {}", a.desc()),
            Command::LDX(a) => format!("LDX {}", a.desc()),
            Command::LDY(a) => format!("LDY {}", a.desc()),
            Command::TXS => "TXS".to_string(),
            Command::DEY => "DEY".to_string(),
            Command::INX => "INX".to_string(),
            Command::BPL(v) => format!("BNE rel {}", v),
            Command::BNE(v) => format!("BNE rel {}", v),
            Command::JMPAbs(addr) => format!("JMP {}", addr),
            Command::CL(t) =>
                match t {
                    FlagType::Carry => "CLC".to_string(),
                    FlagType::IntDisable => "CLI".to_string(),
                    FlagType::Decimal => "CLD".to_string(),
                    FlagType::Overflow => "CLV".to_string(),
                    _ => format!("CL (xxx{:?})", t),
                },
            Command::SE(t) =>
                match t {
                    FlagType::Carry => "SEC".to_string(),
                    FlagType::IntDisable => "SEI".to_string(),
                    FlagType::Decimal => "SED".to_string(),
                    _ => format!("CL (xxx{:?})", t),
                },
            Command::PLP => "PLP".to_string(),
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
            0x8d => Command::STA(AddressingMode::new_absolute(self)),

            0xa1 => Command::LDA(AddressingMode::new_indirect_x(self)),
            0xa5 => Command::LDA(AddressingMode::new_zero_page(self)),
            0xa9 => Command::LDA(AddressingMode::new_imm(self)),
            0xad => Command::LDA(AddressingMode::new_absolute(self)),
            0xb1 => Command::LDA(AddressingMode::new_indirect_y(self)),
            0xb5 => Command::LDA(AddressingMode::new_zero_page_x(self)),
            0xb9 => Command::LDA(AddressingMode::new_absolute_y(self)),
            0xbd => Command::LDA(AddressingMode::new_absolute_x(self)),

            0x9a => Command::TXS,
            0xa2 => Command::LDX(AddressingMode::new_imm(self)),
            0xa0 => Command::LDY(AddressingMode::new_imm(self)),
            0x88 => Command::DEY,
            0xe8 => Command::INX,

            0x10 => Command::BPL(self.read_byte() as i8),
            0xd0 => Command::BNE(self.read_byte() as i8),
            
            0x4c => Command::JMPAbs(self.read_word()),
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
            Command::BPL(rel) => self.exec_branch( |p|{ p & P_MASK_NEGATIVE == 0}, *rel ),
            Command::BNE(rel) => self.exec_branch( |p|{ p & P_MASK_ZERO == 0}, *rel ),

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

    fn read_byte(&mut self) -> u8 {
        let v = self.bus.read(self.pc);
        self.pc += 1;
        v
    }

    fn read_word(&mut self) -> u16 {
        let l = self.bus.read(self.pc);
        self.pc += 1;
        let h = self.bus.read(self.pc);
        self.pc += 1;
        (h as u16) << 8 | l as u16
    }

    pub fn step_next(&mut self) {
        let pc = self.pc;
        let command = self.fetch();
        println!("{:#04x} {}", pc, command.desc());
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