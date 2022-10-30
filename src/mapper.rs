
trait Mapper {
    fn read(&self, addr: u16) -> u8;
    fn write(&self, addr: u16, v: u8);
}

struct Mapper1 {
    prg : Vec::<u8>
}

impl Mapper1 {
    fn new(prg : Vec::<u8>) -> Self {
        Self {
           prg: prg
        }
    }
}

impl Mapper for Mapper1 {
    fn read(&self, addr: u16) -> u8{
        self.prg[addr as usize]
    }
    fn write(&self, addr: u16, v: u8) {
    }
}

struct Mapper3 {
    prg : Vec::<u8>,
    bank : usize
}

impl Mapper3 {
    fn new(prg : Vec::<u8>) -> Self {
        Self {
           prg: prg,
           bank: 0,
        }
    }
}

impl Mapper for Mapper3 {
    fn read(&self, addr: u16) -> u8{
        self.prg[addr as usize]
    }
    fn write(&self, addr: u16, v: u8) {
        
    }
}