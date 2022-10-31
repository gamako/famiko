use std::rc::Rc;


trait Mapper {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, v: u8);
}

pub fn new_mapper(n : u8, prg : Vec::<u8>) -> Box::<dyn Mapper> {
        match n {
            0 => Box::new(Mapper0::new(prg)),
            3 => Box::new(Mapper3::new(prg)),
            _ => panic!("not impl {:}", n)
        }
    
}

struct Mapper0 {
    prg : Vec::<u8>
}

impl Mapper0 {
    fn new(prg : Vec::<u8>) -> Self {
        Self {
           prg: prg
        }
    }
}

impl Mapper for Mapper0 {
    fn read(&self, addr: u16) -> u8{
        self.prg[addr as usize]
    }
    fn write(&mut self, addr: u16, v: u8) {
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
        self.prg[addr as usize | self.bank]
    }
    fn write(&mut self, addr: u16, v: u8) {
        //https://www.nesdev.org/wiki/INES_Mapper_003#Bank_select_($8000-$FFFF)
        self.bank = (0x03 & (v as usize)) << 12;
    }
}