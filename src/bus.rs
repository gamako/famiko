use crate::{ppu::PPU};

#[derive(Debug)]
pub struct Bus {
    pub prg : Vec<u8>,
    pub ppu : PPU,
    ram : Vec<u8>,
}

impl Bus {

    pub fn new(prg: Vec<u8>, chr: Vec<u8>) -> Self {
        Bus { 
            prg: prg,
            ppu: PPU::new(chr),
            ram: [0].repeat(0x800),
        }
    }

    // https://www.nesdev.org/wiki/CPU_memory_map
    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000 ..= 0x1fff => {
                let addr = addr & 0x7fff;
                self.ram[addr as usize]
            }
            0x2000 => 0xff,
            0x2001 => 0xff,
            0x2002 => self.ppu.read_status(),
            0x2003 => 0xff,
            0x2004 => { println!("cant read {:#02x}", addr); panic!("not impl write addr"); },
            0x2005 => 0xff,
            0x2006 => 0xff,
            0x2007 => { self.ppu.read_ppudata() },
            0x4000 ..= 0x4017 => {
                0xff
            }
            0x4020 ..= 0xffff => {
                // mapper-0 prg
                if addr >= 0x8000 {
                    let offset_ = addr - 0x8000;
                    // mapper-0
                    let offset = if offset_ >= 16 * 0x400 && self.prg.len() == 16 * 0x400 {
                        offset_ - 16 * 0x400
                    } else {
                        offset_
                    };
                    self.prg[offset as usize]
                } else {
                    println!("cant read {:#02x}", addr);
                    panic!("not impl read addr");
                }
            }
            _ => {
                println!("cant read {:#02x}", addr);
                panic!("not impl read addr");
            }
        }
    }

    // https://www.nesdev.org/wiki/CPU_memory_map
    pub fn write(&mut self, addr: u16, value: u8) {
        // println!("write {:#04x}: {:#02x}", addr, value);

        match addr {
            0x0000 ..= 0x1fff => {
                let addr = addr & 0x7fff;
                self.ram[addr as usize] = value;
            }
            0x2000 => {
                self.ppu.ppuctrl = value;
            }
            0x2001 => {
                self.ppu.ppumask = value;
            }
            0x2003 => {
                self.ppu.write_ppu_sprite_addr(value);
            }
            0x2004 => {
                self.ppu.write_ppu_sprite_data(value);
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
            0x4000 ..= 0x4017 => {
                println!(" write apu register: {:#02x}", value);
            }
            _ => {
                println!("cant write {:#02x}", addr);
                panic!("not impl write addr");
            }
        }


    }

    pub fn read_nmi(&mut self) -> bool {
        let v = self.ppu.nmi;
        self.ppu.nmi = false;
        v
    }

    pub fn debug_prg_bytes(&mut self, addr: u16, l: usize) -> String {
        (addr .. (addr + (l as u16)))
            .map(|v|{ self.read(v) })
            .map(|x| { format!("{:02X}", x) })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
