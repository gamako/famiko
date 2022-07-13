use std::cell::RefCell;


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

    is_mirror_horizontal: bool,
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
    pub fn new(chr: Vec<u8>, is_mirror_horizontal: bool) -> Self {
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
            is_mirror_horizontal,
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
            frame_sprite_bg: [0].repeat(WIDTH*HEIGHT),
            frame_sprite_fg: [0].repeat(WIDTH*HEIGHT),
            frame_bg: [0].repeat(WIDTH*HEIGHT),
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
                let a = match self.is_mirror_horizontal {
                    true => a & !0x400,
                    false => a & !0x800,
                };
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
                let a = match self.is_mirror_horizontal {
                    true => a & !0x400,
                    false => a & !0x800,
                };
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

    pub fn step(&mut self, cycle : usize) -> Option<Box<Vec<u8>>> {
        let mut ret : Option<Box<Vec<u8>>> =  None;
        for _ in 0..cycle {
            if self.x == 340 {
                // lineの最後で描画する
                if self.y == 0 {
                    // 最初にそれぞれのフレームを描く
                    self.init_frame();
                    self.write_sprite();
                    self.write_frame_bg();
                }

                if self.y < HEIGHT {
                    // 1ラインずつコピーしていく
                    self.write_line(self.y);
                }
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

    pub fn init_frame(&mut self) {
        _ = self.frame_bg.iter_mut().map(|v|*v=0xffu8).count();
        _ = self.frame_sprite_bg.iter_mut().map(|v|*v=0xffu8).count();
        _ = self.frame_sprite_fg.iter_mut().map(|v|*v=0xffu8).count();
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
                
                                if x < WIDTH && y < HEIGHT {
                                    let i = x + y * WIDTH;

                                    self.frame_bg[i] = color;
                                }
                            }
                        }

                        shift_bit += 2;
                    }
                }

            }
        }
    }

    pub fn write_sprite(&mut self) {
        for i in 0..64 {
            let sprite = &self.sprite_ram[i*4..i*4+4];
            let sprite_y = sprite[0] as usize;
            let tile = sprite[1] as usize;
            let attr = sprite[2] as usize;
            let sprite_x = sprite[3] as usize;

            // https://www.nesdev.org/wiki/PPU_OAM
            let is_fg = attr & (1 << 5) != 0;

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

                    let color = self.palette_to_color(palette_base + palette_num);
                    
                    let x_ = sprite_x + x;
                    let y_ = sprite_y + y;
                    if x_ < WIDTH && y_ < HEIGHT {
                        let i = (y_ * WIDTH + x_) as usize;
                        if is_fg {
                            self.frame_sprite_fg[i] = color;
                        } else {
                            self.frame_sprite_bg[i] = color;
                        }
                    }
                }
            }
        }
    }

    pub fn write_line(&mut self, line: usize) {
        for x in 0..WIDTH {
            let p = x + line * WIDTH;
            let mut c = self.frame_sprite_fg[p];
            if c == 0xff {
                c = self.frame_bg[p];
            }
            if c == 0xff {
                c = self.frame_sprite_bg[p];
            }
            let out_pixel = &mut self.frame[p*4..p*4+4];
            if c == 0xff {
                out_pixel.clone_from_slice(&[0x0, 0x0, 0x0, 0xff]);
            } else {
                out_pixel[0..3].clone_from_slice(&COLORS[c as usize]);
                out_pixel[3] = 0xff;
            }
        }
    }

    fn palette_to_color(&self, i: usize) -> u8 {
        if i % 4 == 0 {
            return 0xff
        }
        self.palette_ram[i]
    }

    // debug
    pub fn draw_chr(&self, frame: &mut [u8]) {
        for i in 0..256 {
            let base = i * 16;
            let pattern0 = &self.pattern_table[base .. (base + 8)];
            let pattern1 = &self.pattern_table[(base + 8).. (base + 16)];

            for y_pattern in 0..8 {
                let line0 = pattern0[y_pattern];
                let line1 = pattern1[y_pattern];

                for x_pattern in 0..8 {
                    let pattern_bit = (7 - (x_pattern % 8)) as usize;
                    let palette_num = ((line0 >> pattern_bit) & 1 | ((line1 >> pattern_bit) & 1) << 1) as usize;

                    let x_base = i % 16;
                    let y_base = i / 16;
                    
                    let base = ((y_base * 8 + y_pattern) * 128 + x_base * 8 + x_pattern) * 4;
                    let c = match palette_num {
                        1 => &COLORS[1],
                        2 => &COLORS[3],
                        3 => &COLORS[6],
                        _ => &COLORS[0],
                    };
                    frame[base..base+3].clone_from_slice(c);
                    frame[base+3] = 0xff;
                }
            }
        }
    }
    // debug
    pub fn draw_name_table(&self, frame_: &RefCell<Vec<u8>>) {
        let mut frame = frame_.borrow_mut();

        for i in 0..4 {

            let base_addr = i * 0x400;
            let base_addr = match self.is_mirror_horizontal {
                true => base_addr & !0x400,
                false => base_addr & !0x800,
            };
            let attribute_table = &self.name_table[base_addr + 0x3c0..base_addr + 0x3c0 + 64];

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
                            let shift_bit_ = (shift_bit >> 1) << 1;
                            let palette_index = (attribute as usize >> shift_bit_) & 3;
    
                            let name_index = base_addr + (name_x + name_y * 32) as usize;
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
    
                                    let color = self.palette_ram[palette_index * 4 + palette_num] as usize;
                    
                                    let c = &COLORS[color];

                                    let x_ = if i % 2 == 1 { x + WIDTH } else { x };
                                    let y_ = if i / 2 == 1 { y + HEIGHT } else { y };

                                    if x_ < WIDTH*2 && y_ < HEIGHT*2 {
                                        let j = (x_ + y_ * WIDTH * 2) * 4;
    
                                        frame[j..j+3].clone_from_slice(c);
                                        frame[j+3] = 0xff;
                                    }

                                }
                            }
    
                            shift_bit += 1;
                        }
                    }
    
                }
            }
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