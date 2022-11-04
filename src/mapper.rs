use std::{fmt::Debug, ops::Range};


pub trait Mapper {
    fn read_prg(&self, addr: usize) -> u8;
    fn read_prg_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8];
    fn write_prg(&mut self, addr: u16, v: u8);

    fn read_chr(&self, addr: usize) -> u8;
    fn read_chr_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8];
    fn write_chr(&mut self, addr: u16, v: u8);
}

pub fn new_mapper(n : u8, prg : Vec::<u8>, chr: Vec::<u8>) -> Box::<dyn Mapper> {
    match n {
        0 => Box::new(Mapper0::new(prg, chr)),
        3 => Box::new(Mapper3::new(prg, chr)),
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
    prg : Vec::<u8>,
    chr : Vec::<u8>,
}

impl Mapper0 {
    fn new(prg : Vec::<u8>, chr : Vec::<u8>) -> Self {
        Self {
            prg: prg,
            chr: chr
        }
    }
}

impl Mapper for Mapper0 {
    fn read_prg(&self, addr: usize) -> u8 {
        let offset_ = addr - 0x8000;
        let offset = if offset_ >= 16 * 0x400 && self.prg.len() == 16 * 0x400 {
            offset_ - 16 * 0x400
        } else {
            offset_
        };
        self.prg[offset]
    }
    fn read_prg_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8] {
        let offset_ = addr.start - 0x8000;
        let offset = if offset_ >= 16 * 0x400 && self.prg.len() == 16 * 0x400 {
            offset_ - 16 * 0x400
        } else {
            offset_
        };
        
        &self.prg[offset..offset + addr.len()]
    }
    fn write_prg(&mut self, addr: u16, v: u8) {
    }

    fn read_chr(&self, addr: usize) -> u8 {
        self.chr[addr as usize]
    }
    fn read_chr_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8] {
        &self.prg[addr]
    }
    fn write_chr(&mut self, addr: u16, v: u8) {
    }
}

//https://www.nesdev.org/wiki/INES_Mapper_003#Bank_select_($8000-$FFFF)
struct Mapper3 {
    prg : Vec::<u8>,
    chr : Vec::<u8>,
    bank : usize
}

impl Mapper3 {
    fn new(prg : Vec::<u8>, chr : Vec::<u8>) -> Self {
        Self {
           prg: prg,
           chr: chr,
           bank: 0,
        }
    }
}

impl Mapper for Mapper3 {
    fn read_prg(&self, addr: usize) -> u8 {
        let offset_ = addr - 0x8000;
        let offset = if offset_ >= 16 * 0x400 && self.prg.len() == 16 * 0x400 {
            offset_ - 16 * 0x400
        } else {
            offset_
        };
        self.prg[offset]
    }
    fn read_prg_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8] {
        let offset_ = addr.start - 0x8000;
        let offset = if offset_ >= 16 * 0x400 && self.prg.len() == 16 * 0x400 {
            offset_ - 16 * 0x400
        } else {
            offset_
        };
        &self.prg[offset..offset + addr.len()]
    }
    fn write_prg(&mut self, addr: u16, v: u8) {
        self.bank = (0x03 & (v as usize)) << 12;
    }

    fn read_chr(&self, addr: usize) -> u8{
        self.chr[addr | self.bank]
    }

    fn read_chr_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8] {
        let r = (addr.start | self.bank)..(addr.end | self.bank);
        &self.chr[r]
    }

    fn write_chr(&mut self, addr: u16, v: u8) {
    }
}