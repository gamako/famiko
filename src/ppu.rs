
pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 240;
pub const FRAME_SIZE : usize = WIDTH * HEIGHT * 4;

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
    sprite_addr : u8,
    scroll_x : u8,
    scroll_y : u8,
    scroll_next_y : bool,
    palette_ram : [u8; 0x20],
    name_table : [u8; 0x400 * 4],
    pattern_table : Vec<u8>,
    sprite_ram : [u8; 0x100],
    
    x : usize,
    y : usize,

    pub nmi : bool,

    frame: Vec<u8>,

    frame_sprite_fg: Vec<u8>,
    frame_sprite_bg: Vec<u8>,
    frame_bg: Vec<u8>,
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
            sprite_addr: 0,
            scroll_x: 0,
            scroll_y: 0,
            scroll_next_y: true,
            palette_ram: [0; 0x20],
            name_table: [0; 0x400 * 4],
            pattern_table: chr,
            sprite_ram: [0; 0x100],
            nmi : false,
            x: 0,
            y: 0,
            frame: [0].repeat(FRAME_SIZE),
            frame_sprite_bg: [0].repeat(FRAME_SIZE),
            frame_sprite_fg: [0].repeat(FRAME_SIZE),
            frame_bg: [0].repeat(FRAME_SIZE),
         }
    }

    pub fn x_(&self) -> usize {
        self.x
    }

    pub fn y_(&self) -> usize {
        self.y
    }

    fn read_vblank(&self) -> bool {
        self.ppustatus & 0x80 != 0
    }

    fn update_vblank(&mut self, b: bool) {
        self.ppustatus = if b {
            self.ppustatus | (1u8 << 7)
        } else {
            self.ppustatus & !(1u8 << 7)
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let status = self.ppustatus;
        self.update_vblank(false);
        status
    }

    pub fn write_ppuscroll(&mut self, v : u8) {
        println!(" write scroll: {:02x}", v);
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
                let a = self.addr as usize - 0x2000;
                let v = self.name_table[a];
                println!(" read nametable: {:04x} {:02X}", a, v);
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

    pub fn write_ppu_sprite_addr(&mut self, v: u8) {
        self.sprite_addr = v;
    }
    pub fn write_ppu_sprite_data(&mut self, v: u8) {
        println!(" write sprite data: {:02x} {:02X}", self.sprite_addr, v);
        self.sprite_ram[self.sprite_addr as usize] = v;
    }

    pub fn write_ppudata(&mut self, v : u8) {
        match self.addr {
            0x2000 ..= 0x2fff => {
                let a = self.addr as usize - 0x2000;
                let v0 = self.name_table[a];
                println!(" write nametable: {:04x} {:02X} {:02X}", a, v0, v);
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
    
    pub fn write_dma(&mut self, data : &[u8]) {
        self.sprite_ram.clone_from_slice(data)
    }

    // 戻り値は描画フレーム
    pub fn step(&mut self, cycle : usize) -> Option<Box<Vec<u8>>> {
        let mut ret : Option<Box<Vec<u8>>> =  None;
        for _ in 0..cycle {
            let x = self.x;
            let y = self.y;
            if x < WIDTH && y < HEIGHT {
                let attribute_table = &self.name_table[960..960 + 64];
                let attribute_table_index = x / 32 + y / 32 * 8;
                let bit = match (x%32 < 16, y%32 < 16) {
                    (true,true) => 0,
                    (false,true) => 2,
                    (true,false) => 4,
                    (false,false) => 6,
                };
                let palette_index = ((attribute_table[attribute_table_index as usize] >> bit) & 3) as usize;
    
                let name_index = (x / 8 + y / 8 * 32) as usize;
                let pattern_index = self.name_table[name_index] as usize;
    
                let pattern_y = (y % 8) as usize;
    
                let pattern0 = self.pattern_table[pattern_index * 16 + pattern_y];
                let pattern1 = self.pattern_table[pattern_index * 16 + pattern_y + 8];
                let pattern_bit = (7 - (x % 8)) as usize;
                let palette_num = ((pattern0 >> pattern_bit) & 1 | ((pattern1 >> pattern_bit) & 1) << 1) as usize;
    
                let color = self.palette_ram[palette_index * 4 + palette_num];
    
                let rgb = COLORS[color as usize];

                let i = (x + y * WIDTH) * 4;
                let pixel = & mut self.frame[i..i+4];

                pixel[0..3].copy_from_slice(&rgb);
                pixel[3] = 0xff;
            }

            self.x += 1;
            if self.x >= 341 {
                self.x = 0;
                self.y += 1;
                if self.y == 241 {
                    self.update_vblank(true);
                    if self.ppuctrl & (1 << 7) != 0 {
                        self.nmi = true;
                    }
                } else if self.y == 261 {
                    self.update_vblank(false);
                }
                if self.y > 262 {
                    self.y = 0;
                    ret = Some(Box::new(self.frame.clone()));
                    self.frame.iter_mut().for_each(|v| *v = 0);
                }
            }
        }
        ret
    }

    //　スプライトのパレット番号を返す
    fn sprite_pixel(&self, x: usize, y: usize) -> Option<usize> {
        for i in 0..64 {
            let sprite = &self.sprite_ram[i*4..i*4+4];
            let sprite_y = sprite[0] as usize ;
            let sprite_tile = sprite[1];
            let sprite_attr = sprite[2];
            let sprite_x = sprite[3] as usize ;

            if x >= sprite_x && x < sprite_x + 8 && 
            y >= sprite_y && y < sprite_y + 8 {
                let x_ = x - sprite_x;
                let y_ = y - sprite_y;

                
            }
        }   
        None
    }

    pub fn init_frame(&mut self) {
        _ = self.frame_bg.iter_mut().map(|v|*v=0u8);
        _ = self.frame_sprite_bg.iter_mut().map(|v|*v=0u8);
        _ = self.frame_sprite_fg.iter_mut().map(|v|*v=0u8);
    }


    pub fn write_frame_bg(&mut self) {

        let attribute_table = &self.name_table[0x3c0..0x3c0 + 64];

        // 属性テーブルは32x32px単位で
        // https://www.nesdev.org/wiki/PPU_attribute_tables
        // https://taotao54321.hatenablog.com/entry/2017/04/11/115205
        for attr_y in 0..8 {
            for attr_x in 0..8 {
                let attribute = attribute_table[attr_y * 8 + attr_x];
                let mut shift_bit = 0;

                // nameテーブルは8x8px単位
                // https://www.nesdev.org/wiki/PPU_nametables
                // https://taotao54321.hatenablog.com/entry/2017/04/11/115205
                let name_y_base = attr_y * 4;
                let name_x_base = attr_x * 4;
                for name_y in name_y_base..name_y_base + 4 {
                    for name_x in name_x_base..name_x_base + 4 {
                        let palette_index = (attribute as usize >> shift_bit) & 3;

                        let name_index = (name_x + name_y * 32) as usize;
                        let pattern_index = self.name_table[name_index] as usize;

                        let y_base = name_y * 8;
                        let x_base = name_x * 8;
                        for y in y_base..y_base+8 {
                            for x in x_base..x_base+8 {
                                let pattern_y = (y % 8) as usize;

                                let pattern0 = self.pattern_table[pattern_index * 16 + pattern_y];
                                let pattern1 = self.pattern_table[pattern_index * 16 + pattern_y + 8];

                                let pattern_bit = (7 - (x % 8)) as usize;
                                let palette_num = ((pattern0 >> pattern_bit) & 1 | ((pattern1 >> pattern_bit) & 1) << 1) as usize;

                                let color = self.palette_ram[palette_index * 4 + palette_num];
                                let rgb = COLORS[color as usize];
                
                                let i = (x + y * WIDTH) * 4;
                                let pixel = & mut self.frame[i..i+4];

                                pixel[0..3].copy_from_slice(&rgb);
                                pixel[3] = 0xff;

                            }
                        }

                        shift_bit += 2;
                    }
                }

            }
        }
    }

    

    pub fn write_sprite(&mut self, out_fg: &mut [u8], out_bg: &mut [u8]) {
        for i in 0..64 {
            let sprite = &self.sprite_ram[i*4..i*4+4];
            let sprite_y = sprite[0] as usize;
            let tile = sprite[1] as usize;
            let attr = sprite[2] as usize;
            let sprite_x = sprite[3] as usize;

            // size : 8x8
            let pattern_table_base = if self.ppuctrl & 0x08 != 0 { 0x0000usize } else { 0x1000usize };
            let pattern_base = pattern_table_base + tile * 16;
            let pattern_table = &self.pattern_table[pattern_base..pattern_base+16];
            let palette_type = attr & 3;
            let palette_base = palette_type * 4 + 0x10;

            for y in 0..8usize {
                for x in 0..8usize {
                    let pattern0 = pattern_table[y];
                    let pattern1 = pattern_table[y + 8];

                    let pattern_bit = 7 - x;
                    let palette_num = ((pattern0 >> pattern_bit) & 1 | ((pattern1 >> pattern_bit) & 1) << 1) as usize;

                    let color = self.palette_to_color(&self.palette_ram, palette_base + palette_num);
                    
                    out[((sprite_y + y) * WIDTH + sprite_x + x) as usize] = color;
                }
            }
        }
    }

    fn palette_to_color(&self, palette_ram: &[u8;32], i: usize) -> u8 {
        if i % 4 == 0 {
            return 0xff
        }
        palette_ram[i]
    }

    pub fn draw(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % WIDTH) as i16;
            let y = (i / WIDTH) as i16;
            
            let attribute_table = &self.name_table[960..960 + 64];
            let attribute_table_index = x / 32 + y / 32 * 8;
            let bit = match (x%32 < 16, y%32 < 16) {
                (true,true) => 0,
                (false,true) => 2,
                (true,false) => 4,
                (false,false) => 6,
            };
            let palette_index = ((attribute_table[attribute_table_index as usize] >> bit) & 3) as usize;

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