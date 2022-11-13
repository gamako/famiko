use std::{fmt::Debug, ops::Range};


pub trait Mapper : Debug {
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

struct Mapper0 {
    prg : Vec::<u8>,
    chr : Vec::<u8>,
}

impl Debug for Mapper0 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Mapper0")
    }
}

impl Mapper0 {
    fn new(prg : Vec::<u8>, chr : Vec::<u8>) -> Self {
        Self {
            prg: prg,
            chr: chr
        }
    }

    fn offset_from(&self, addr: usize) -> usize {
        let offset_ = addr - 0x8000;
        if offset_ >= 16 * 0x400 && self.prg.len() == 16 * 0x400 {
            offset_ - 16 * 0x400
        } else {
            offset_
        }
    }
}

impl Mapper for Mapper0 {

    fn read_prg(&self, addr: usize) -> u8 {
        self.prg[self.offset_from(addr)]
    }
    fn read_prg_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8] {
        let offset = self.offset_from(addr.start);
        &self.prg[offset..offset + addr.len()]
    }

    fn write_prg(&mut self, _addr: u16, _v: u8) {
    }

    fn read_chr(&self, addr: usize) -> u8 {
        self.chr[addr as usize]
    }
    fn read_chr_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8] {
        &self.prg[addr]
    }
    fn write_chr(&mut self, _addr: u16, _v: u8) {
    }
}

//https://www.nesdev.org/wiki/INES_Mapper_003
struct Mapper2 {
    prg : Vec::<u8>,
    chr : Vec::<u8>,
    bank : usize
}

impl Debug for Mapper2 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Mapper2")
    }
}

impl Mapper2 {
    fn new(prg : Vec::<u8>, chr : Vec::<u8>) -> Self {
        print!("prg: {:} chr {:}", prg.len(), chr.len());
        Self {
           prg: prg,
           chr: chr,
           bank: 0,
        }
    }
    fn offset_from(&self, addr: usize) -> usize {
        let offset_ = addr - 0x8000;
        if offset_ >= 16 * 0x400 && self.prg.len() == 16 * 0x400 {
            offset_ - 16 * 0x400
        } else {
            offset_
        }
    }
}

impl Mapper for Mapper2 {
    fn read_prg(&self, addr: usize) -> u8 {
        self.prg[self.offset_from(addr)]
    }
    fn read_prg_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8] {
        let offset = self.offset_from(addr.start);
        &self.prg[offset..offset + addr.len()]
    }
    fn write_prg(&mut self, _addr: u16, v: u8) {
        print!("write {:04x} {:02x}", _addr, v);
        self.bank = (0x03 & (v as usize)) * 0x2000;
    }

    fn read_chr(&self, addr: usize) -> u8 {
        self.chr[addr as usize]
    }
    fn read_chr_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8] {
        &self.prg[addr]
    }
    fn write_chr(&mut self, _addr: u16, _v: u8) {
    }
}

//https://www.nesdev.org/wiki/INES_Mapper_003#Bank_select_($8000-$FFFF)
struct Mapper3 {
    prg : Vec::<u8>,
    chr : Vec::<u8>,
    bank : usize
}

impl Debug for Mapper3 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Mapper3")
    }
}


impl Mapper3 {
    fn new(prg : Vec::<u8>, chr : Vec::<u8>) -> Self {
        Self {
           prg: prg,
           chr: chr,
           bank: 0,
        }
    }
    fn offset_from(&self, addr: usize) -> usize {
        let offset_ = addr - 0x8000;
        if offset_ >= 16 * 0x400 && self.prg.len() == 16 * 0x400 {
            offset_ - 16 * 0x400
        } else {
            offset_
        }
    }
}

impl Mapper for Mapper3 {
    fn read_prg(&self, addr: usize) -> u8 {
        self.prg[self.offset_from(addr)]
    }
    fn read_prg_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8] {
        let offset = self.offset_from(addr.start);
        &self.prg[offset..offset + addr.len()]
    }
    fn write_prg(&mut self, _addr: u16, v: u8) {
        self.bank = (0x03 & (v as usize)) * 0x2000;
    }

    fn read_chr(&self, addr: usize) -> u8{
        self.chr[addr | self.bank]
    }

    fn read_chr_range<'a>(&'a self, addr: Range<usize>) -> &'a [u8] {
        let r = (addr.start | self.bank)..(addr.end | self.bank);
        &self.chr[r]
    }

    fn write_chr(&mut self, _addr: u16, _v: u8) {
    }
}