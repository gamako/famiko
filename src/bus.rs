use crate::ppu::PPU;

#[derive(Debug)]
pub struct Bus {
    pub prg : Vec<u8>,
    pub ppu : PPU,
}

impl Bus {

    pub fn new(prg: Vec<u8>, chr: Vec<u8>) -> Self {
        Bus { 
            prg: prg,
            ppu: PPU::new(chr),
        }
    }

    // https://www.nesdev.org/wiki/CPU_memory_map
    pub fn read(&self, addr: u16) -> u8 {
        if addr >= 0x8000 {
            let offset_ = addr - 0x8000;
            // mapper-0
            let offset = if offset_ >= 16 * 0x400 && self.prg.len() == 16 * 0x400 {
                offset_ - 16 * 0x400
            } else {
                offset_
            };
            return self.prg[offset as usize];
        }
        println!("cant read {:#02x}", addr);
        panic!("not impl read addr");
    }

    // https://www.nesdev.org/wiki/CPU_memory_map
    pub fn write(&mut self, addr: u16, value: u8) {
        println!("write {:#04x}: {:#02x}", addr, value);

        match addr {
            0x0000 ..= 0x07ff => {
                // ram
                println!(" write ram");
            }
            0x2000 => {
                self.ppu.ppuctrl = value;
            }
            0x2001 => {
                self.ppu.ppumask = value;
            }
            0x2005 => {
                self.ppu.write_ppuscroll(value);
            }
            0x2006 => {
                println!(" write ppuaddr: {:#02x}", value);
                self.ppu.write_ppuaddr(value);
            }
            0x2007 => {
                println!(" write ppudata: {:#02x}", value);
                self.ppu.write_ppudata(value);
            }
            _ => {
                println!("cant write {:#02x}", addr);
                panic!("not impl write addr");
            }
        }


    }
}
