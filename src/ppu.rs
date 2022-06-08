
pub const WIDTH: u32 = 256;
pub const HEIGHT: u32 = 240;

#[allow(dead_code)]
#[derive(Debug)]
pub struct PPU {
    // https://www.nesdev.org/wiki/PPU_registers
    pub ppuctrl : u8,
    pub ppumask	: u8,
    pub ppustatus : u8,
    pub oamaddr : u8,
    pub oamdata : u8,
    pub ppuscroll : u8,
    pub ppuaddr : u8,
    pub ppudata : u8,
    pub oamdma: u8,

    addr: u16,
    scroll_x : u8,
    scroll_y : u8,
    scroll_next_y : bool,
    palette_ram : [u8; 0x20],
    name_table : [u8; 0x400 * 4],
    pattern_table : Vec<u8>
}

impl PPU {
    pub fn new(chr: Vec<u8>) -> Self {
        PPU { 
            ppuctrl: 0,
            ppumask: 0,
            ppustatus: 1 << 7,
            oamaddr: 0,
            oamdata: 0,
            ppuscroll: 0,
            ppuaddr: 0,
            ppudata: 0,
            oamdma: 0,
            addr: 0,
            scroll_x: 0,
            scroll_y: 0,
            scroll_next_y: true,
            palette_ram: [0; 0x20],
            name_table: [0; 0x400 * 4],
            pattern_table: chr,
         }
    }

    pub fn read_status(&mut self) -> u8 {
        let status = self.ppustatus;

        self.ppustatus |= 1 << 7;
        status
    }

    pub fn write_ppuscroll(&mut self, v : u8) {
        println!(" write scroll: {:#02x}", v);
        match self.scroll_next_y {
            false => { self.scroll_x = v }
            true => { self.scroll_y = v }
        }
        self.scroll_next_y = self.scroll_next_y;
    }

    pub fn write_ppuaddr(&mut self, v : u8) {
        self.addr = self.addr << 8 | v as u16;
    }

    // https://www.nesdev.org/wiki/PPU_memory_map
    pub fn read_ppudata(&mut self) -> u8 {
        match self.addr {
            0x2000 ..= 0x2fff => {
                let v = self.name_table[self.addr as usize - 0x2000];
                println!(" read nametable: {:#04x} {:02X}", self.addr, v);
                v
            }
            0x3f00 ..= 0x3fff => {
                let a = (self.addr & 0x001f) as usize;
                let v = self.palette_ram[a];
                println!(" write palette_ram: {:04x} {:02X}", a, v);
                v
            }
            _ => {
                println!(" ppu cant read {:04X}", self.addr);
                panic!("not impl ppu read addr");
            }
        }
    }

    pub fn write_ppudata(&mut self, v : u8) {
        match self.addr {
            0x2000 ..= 0x2fff => {
                let a = self.addr as usize - 0x2000;
                let v0 = self.name_table[a];
                println!(" write nametable: {:04x} {:02X} {:02X}", self.addr, v0, v);
                self.name_table[a] = v;
                self.addr += 1;
            }
            0x3f00 ..= 0x3fff => {
                let a = (self.addr & 0x001f) as usize;
                let v0 = self.palette_ram[a];
                println!(" write palette_ram: {:04x} {:02X} {:02X}", a, v0, v);
                self.palette_ram[a] = v;
                self.addr += 1;
            }
            _ => {
                println!(" ppu cant write {:04x} {:02X}", self.addr, v);
                panic!("not impl ppu write addr");
            }
        }
    }

    pub fn draw(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % WIDTH as usize) as i16;
            let y = (i / WIDTH as usize) as i16;
            
            let attribute_table = &self.name_table[960..960 + 64];
            let attribute_table_index = x / 32 + y / 32 * 8;
            let bit = match (x%32 < 16, y%32 < 16) {
                (true,true) => 0,
                (false,true) => 2,
                (true,false) => 4,
                (false,false) => 6,
            };
            let palette_index = (attribute_table[attribute_table_index as usize] >> bit) as usize;

            let name_index = (x / 8 + y / 8 * 32) as usize;
            let pattern_index = self.name_table[name_index] as usize;

            let pattern_y = (y % 8) as usize;

            let pattern0 = self.pattern_table[pattern_index * 16 + pattern_y];
            let pattern1 = self.pattern_table[pattern_index * 16 + pattern_y + 8];
            let pattern_bit = (7 - (x % 8)) as usize;
            let palette_num = ((pattern0 >> pattern_bit) & 1 | ((pattern1 >> pattern_bit) & 1) << 1) as usize;

            let color = self.palette_ram[palette_index * 4 + palette_num];

            let rgb = COLORS[color as usize];

            pixel[0..3].copy_from_slice(&rgb);
            pixel[3] = 0xff;
        }
    }
}

static COLORS : [[u8;3];64]= [
    [0x80, 0x80, 0x80], [0x00, 0x3D, 0xA6], [0x00, 0x12, 0xB0], [0x44, 0x00, 0x96],
    [0xA1, 0x00, 0x5E], [0xC7, 0x00, 0x28], [0xBA, 0x06, 0x00], [0x8C, 0x17, 0x00],
    [0x5C, 0x2F, 0x00], [0x10, 0x45, 0x00], [0x05, 0x4A, 0x00], [0x00, 0x47, 0x2E],
    [0x00, 0x41, 0x66], [0x00, 0x00, 0x00], [0x05, 0x05, 0x05], [0x05, 0x05, 0x05],
    [0xC7, 0xC7, 0xC7], [0x00, 0x77, 0xFF], [0x21, 0x55, 0xFF], [0x82, 0x37, 0xFA],
    [0xEB, 0x2F, 0xB5], [0xFF, 0x29, 0x50], [0xFF, 0x22, 0x00], [0xD6, 0x32, 0x00],
    [0xC4, 0x62, 0x00], [0x35, 0x80, 0x00], [0x05, 0x8F, 0x00], [0x00, 0x8A, 0x55],
    [0x00, 0x99, 0xCC], [0x21, 0x21, 0x21], [0x09, 0x09, 0x09], [0x09, 0x09, 0x09],
    [0xFF, 0xFF, 0xFF], [0x0F, 0xD7, 0xFF], [0x69, 0xA2, 0xFF], [0xD4, 0x80, 0xFF],
    [0xFF, 0x45, 0xF3], [0xFF, 0x61, 0x8B], [0xFF, 0x88, 0x33], [0xFF, 0x9C, 0x12],
    [0xFA, 0xBC, 0x20], [0x9F, 0xE3, 0x0E], [0x2B, 0xF0, 0x35], [0x0C, 0xF0, 0xA4],
    [0x05, 0xFB, 0xFF], [0x5E, 0x5E, 0x5E], [0x0D, 0x0D, 0x0D], [0x0D, 0x0D, 0x0D],
    [0xFF, 0xFF, 0xFF], [0xA6, 0xFC, 0xFF], [0xB3, 0xEC, 0xFF], [0xDA, 0xAB, 0xEB],
    [0xFF, 0xA8, 0xF9], [0xFF, 0xAB, 0xB3], [0xFF, 0xD2, 0xB0], [0xFF, 0xEF, 0xA6],
    [0xFF, 0xF7, 0x9C], [0xD7, 0xE8, 0x95], [0xA6, 0xED, 0xAF], [0xA2, 0xF2, 0xDA],
    [0x99, 0xFF, 0xFC], [0xDD, 0xDD, 0xDD], [0x11, 0x11, 0x11], [0x11, 0x11, 0x11],
  ];