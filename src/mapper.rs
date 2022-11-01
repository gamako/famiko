use std::{fmt::Debug, ops::Range};


pub trait Mapper {
    fn read(&self, addr: usize) -> u8;
    fn read_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8];
    fn write(&mut self, addr: u16, v: u8);
}

pub fn new_mapper(n : u8, prg : Vec::<u8>) -> Box::<dyn Mapper> {
    match n {
        0 => Box::new(Mapper0::new(prg)),
        3 => Box::new(Mapper3::new(prg)),
        _ => panic!("not impl {:}", n)
    }
}

impl Debug for dyn Mapper {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Mapper0 => write!(f, "Mapper0"),
            Mapper3 => write!(f, "Mapper3"),
            _ =>  panic!("not impl"),
        }
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
    fn read(&self, addr: usize) -> u8{
        self.prg[addr as usize]
    }

    fn read_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8] {
        &self.prg[addr]
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
    fn read(&self, addr: usize) -> u8{
        self.prg[addr | self.bank]
    }

    fn read_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8] {
        let r = (addr.start | self.bank)..(addr.end | self.bank);
        &self.prg[r]
    }

    fn write(&mut self, addr: u16, v: u8) {
        //https://www.nesdev.org/wiki/INES_Mapper_003#Bank_select_($8000-$FFFF)
        self.bank = (0x03 & (v as usize)) << 12;
    }
}