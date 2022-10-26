use std::cell::RefCell;

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 240;
pub const FRAME_SIZE : usize = WIDTH * HEIGHT * 4;

pub const CHR_DEBUG_WIDTH : usize = 16 * 8 * 2;
pub const CHR_DEBUG_HEIGT : usize = 16 * 8;
pub const CHR_DEBUG_FRAME_SIZE : usize = CHR_DEBUG_HEIGT * CHR_DEBUG_WIDTH;

pub const SPRITE_DEBUG_WIDTH : usize = 8 * 8;
pub const SPRITE_DEBUG_HEIGT : usize = 8 * 8;
pub const SPRITE_DEBUG_FRAME_SIZE : usize = SPRITE_DEBUG_HEIGT * SPRITE_DEBUG_WIDTH;

const CLEAR_COLOR : u8 = 0x40;

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

    togle : bool,
    
    is_mirror_horizontal: bool,
    vram_addr: u16,
    temp_vram_addr: u16,
    x_value : u8,
    sprite_addr : u8,
    scroll_x : u8,
    scroll_y : u8,
    palette_ram : [u8; 0x20],
    name_table : [u8; 0x400 * 4],
    pattern_table : Vec<u8>,
    sprite_ram : [u8; 0x100],

    read_buffer : u8,
    
    x : usize,
    y : usize,

    pub nmi : bool,

    frame: Vec<u8>,

    frame_sprite_fg: Vec<u8>,
    frame_sprite_bg: Vec<u8>,
    frame_bg: RefCell<Vec<u8>>,
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
            togle: false,
            is_mirror_horizontal,
            vram_addr: 0,
            temp_vram_addr: 0,
            x_value : 0,
            sprite_addr: 0,
            scroll_x: 0,
            scroll_y: 0,
            palette_ram: [0; 0x20],
            name_table: [0; 0x400 * 4],
            pattern_table: chr,
            sprite_ram: [0; 0x100],
            read_buffer : 0,
            nmi : false,
            x: 0,
            y: 0,
            frame: [0].repeat(FRAME_SIZE),
            frame_sprite_bg: [0].repeat(WIDTH*HEIGHT),
            frame_sprite_fg: [0].repeat(WIDTH*HEIGHT),
            frame_bg: RefCell::new([0].repeat(WIDTH*HEIGHT*4)),
         }
    }

    pub fn x_(&self) -> usize {
        self.x
    }

    pub fn y_(&self) -> usize {
        self.y
    }

    fn update_vblank(&mut self, b: bool) {
        self.ppustatus = if b {
            self.ppustatus | (1u8 << 7)
        } else {
            self.ppustatus & !(1u8 << 7)
        }
    }
    fn update_sprite_0_hit(&mut self, b: bool) {
        self.ppustatus = if b {
            self.ppustatus | (1u8 << 6)
        } else {
            self.ppustatus & !(1u8 << 6)
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let status = self.ppustatus;
        self.update_vblank(false);
        self.togle = false;
        status
    }

    pub fn write_ppuctrl(&mut self, v : u8) {
        self.ppuctrl = v;

        // t: ...GH.. ........ <- d: ......GH
        //    <used elsewhere> <- d: ABCDEF..
        self.temp_vram_addr = self.temp_vram_addr & !0xc00 | (v as u16) & 0x3 << 10;
    }

    pub fn write_ppuscroll(&mut self, v : u8) {
        match self.togle {
            false => { 
                self.scroll_x = v;
                // t: ....... ...ABCDE <- d: ABCDE...
                // x:              FGH <- d: .....FGH
                // w:                  <- 1
                self.temp_vram_addr = self.temp_vram_addr & !0x1f | (v as u16) >> 3;
                self.x_value = v & 0x07;
            }
            true => { 
                self.scroll_y = v;
                // t: FGH..AB CDE..... <- d: ABCDEFGH
                // w:                  <- 0
                self.temp_vram_addr = self.temp_vram_addr & !0x0c1f | (v as u16) & 0x07 << 12 | (v as u16) & 0xf8 << 2;
            }
        }
        self.togle = !self.togle;
    }

    pub fn write_ppuaddr(&mut self, v : u8) {
        self.vram_addr = self.vram_addr << 8 | v as u16;
        if !self.togle {
            self.ppuctrl = self.ppuctrl & !0x3u8 | v & 0x3u8;
        }
        match self.togle {
            false => {
                self.temp_vram_addr = self.temp_vram_addr & 0x00ff | (v as u16) & 0x3f << 8;
            }
            true => {
                self.temp_vram_addr = self.temp_vram_addr & 0xff00 | (v as u16);
                //self.vram_addr = self.temp_vram_addr;
            }
        }
        self.togle = !self.togle;
    }

    // https://www.nesdev.org/wiki/PPU_memory_map
    pub fn read_ppudata(&mut self, is_increment : bool) -> u8 {
        let read_for_buffer = match self.vram_addr {
            0x0000 ..= 0x1fff => {
                self.pattern_table[self.vram_addr as usize]
            }
            0x2000 ..= 0x3fff => {
                let a = (self.vram_addr as usize - 0x2000) % 0x1000;
                let a = match self.is_mirror_horizontal {
                    true => a & !0x400,
                    false => a & !0x800,
                };
                self.name_table[a]
            }
            _ => {
                println!(" ppu cant read {:04X}", self.vram_addr);
                panic!("not impl ppu read addr");
            }
        };
        if is_increment {
            if self.ppuctrl & 4 != 0 {
                self.vram_addr += 32;
            } else {
                self.vram_addr += 1;
            }
        };
        // パレットのみ値がすぐに読める
        let ret = match self.vram_addr {
            0x3f00 ..= 0x3fff => {
                let a = (self.vram_addr & 0x001f) as usize;
                self.palette_ram[a]
            }
            _ => self.read_buffer
        };
        self.read_buffer = read_for_buffer;
        ret
    }

    pub fn write_ppu_sprite_addr(&mut self, v: u8) {
        self.sprite_addr = v;
    }
    pub fn write_ppu_sprite_data(&mut self, v: u8) {
        self.sprite_ram[self.sprite_addr as usize] = v;
    }

    pub fn write_ppudata(&mut self, v : u8) {
        match self.vram_addr {
            0x2000 ..= 0x3eff | 0x3f20 ..= 0x3fff => {
                let a = (self.vram_addr as usize - 0x2000) % 0x1000;
                let a = match self.is_mirror_horizontal {
                    true => a & !0x400,
                    false => a & !0x800,
                };
                self.name_table[a] = v;
            }
            0x3f00 ..= 0x3f1f => {
                let a = (self.vram_addr & 0x001f) as usize;
                if a % 4 == 0 {
                    let a = a & 0x0f;
                    self.palette_ram[a] = v;
                    self.palette_ram[a | 0x10 ] = v;
                } else {
                    self.palette_ram[a] = v;
                }
            }
            _ => {
                println!(" ppu cant write {:04x} {:02X}", self.vram_addr, v);
                panic!("not impl ppu write addr");
            }
        }
        if self.ppuctrl & 4 != 0 {
            self.vram_addr += 32;
        } else {
            self.vram_addr += 1;
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
                    self.write_sprite(&mut None::<Vec<u8>>);
                    self.write_frame_bg();
                }

                if self.y < HEIGHT {
                    // 1ラインずつコピーしていく
                    let sprite_0_hit = self.write_line(self.y);
                    if sprite_0_hit {
                        self.update_sprite_0_hit(true);
                    }
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
                    self.update_sprite_0_hit(false);
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
        _ = self.frame_bg.borrow_mut().iter_mut().map(|v|*v=CLEAR_COLOR).count();
        _ = self.frame_sprite_bg.iter_mut().map(|v|*v=CLEAR_COLOR).count();
        _ = self.frame_sprite_fg.iter_mut().map(|v|*v=CLEAR_COLOR).count();
    }

    pub fn write_frame_bg(&mut self) {
        let mut frame = self.frame_bg.borrow_mut();
        self.draw_name_table_(|x, y, c|{
            let i = x + y * WIDTH * 2;
            frame[i] = c as u8;
        });
    }

    pub fn write_sprite(&mut self, frame: &mut Option<Vec<u8>>) {
        for sprite_i in 0..64 {
            let sprite = &self.sprite_ram[sprite_i*4..sprite_i*4+4];
            let tile = sprite[1] as usize;
            let attr = sprite[2] as usize;
            let is_h_reverse = attr & (1 << 6) != 0;
            let is_v_reverse = attr & (1 << 7) != 0;

            let is_debug = frame.is_some();
            let sprite_x = if !is_debug {sprite[3] as usize} else { sprite_i % 8 * 8};
            let sprite_y = if !is_debug {sprite[0] as usize} else { sprite_i / 8 * 8};
            let width = if !is_debug { WIDTH } else { 64 };

            // https://www.nesdev.org/wiki/PPU_OAM
            let is_fg = attr & (1 << 5) == 0;

            // size : 8x8
            let pattern_table_base = if self.ppuctrl & 0x08 != 0 { 0x1000usize } else { 0x0000usize };
            let pattern_base = pattern_table_base + tile * 16;
            let pattern_table = &self.pattern_table[pattern_base..pattern_base+16];
            let palette_type = attr & 3;
            let palette_base = palette_type * 4 + 0x10;

            for y in 0..8usize {
                let y_ = if is_v_reverse { 7 - y } else { y };
                let pattern0 = pattern_table[y_];
                let pattern1 = pattern_table[y_ + 8];

                for x in 0..8usize {

                    let pattern_bit = if is_h_reverse { x } else { 7 - x };
                    let palette_num = ((pattern0 >> pattern_bit) & 1 | ((pattern1 >> pattern_bit) & 1) << 1) as usize;

                    let color = self.palette_to_color(palette_base + palette_num);
                    
                    let x_ = sprite_x + x;
                    let y_ = sprite_y + y;
                    if x_ < width && y_ < HEIGHT {
                        let i = (y_ * width + x_) as usize;
                        if let Some(frame_) = frame {
                            let i = i*4;
                            if color == CLEAR_COLOR {
                                frame_[i+0] = 0;
                                frame_[i+1] = 0;
                                frame_[i+2] = 0;
                            } else {
                                let c = &COLORS[color as usize];
                                frame_[i..i+3].clone_from_slice(c);
                            }
                            frame_[i+3] = 0xff;
                        } else {
                            // Sprite 0 Hit判定のためにフラグを一緒にセットする
                            if color != CLEAR_COLOR {
                                let color_ = color | if sprite_i == 0 && palette_num % 4 != 0 { 0x80 } else { 0x00 };
                                if is_fg {
                                    self.frame_sprite_fg[i] = self.frame_sprite_fg[i] & 0x80 | color_;
                                } else {
                                    self.frame_sprite_bg[i] = self.frame_sprite_bg[i] & 0x80 | color_;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 戻り値はsprite 0 hitが起きた時にonとなる
    pub fn write_line(&mut self, line: usize) -> bool {
        let mut sprite_0_hit = false;
        for x in 0..WIDTH {
            let p = x + line * WIDTH;
            let scroll_x = self.scroll_x as usize + ((self.ppuctrl & 1) as usize) * 256;
            let scroll_y = self.scroll_y as usize + (((self.ppuctrl & 2) >> 1) as usize) * 240;
            let p2 = ((x + scroll_x) % (WIDTH*2)) + ((line + scroll_y) % (HEIGHT * 2)) * WIDTH * 2;
            let mut c = self.frame_sprite_fg[p];
            if c == CLEAR_COLOR {
                c = self.frame_bg.borrow()[p2];
            }
            if c == CLEAR_COLOR {
                c = self.frame_sprite_bg[p];
            }
            let out_pixel = &mut self.frame[p*4..p*4+4];
            if c == CLEAR_COLOR {
                out_pixel[0..3].clone_from_slice(&COLORS[self.palette_ram[0] as usize]);
                out_pixel[3] = 0xff;
            } else {
                out_pixel[0..3].clone_from_slice(&COLORS[(c & 0x3f) as usize]);
                out_pixel[3] = 0xff;
            }

            if (self.frame_sprite_fg[p] & 0x80 != 0) || ( self.frame_sprite_bg[p] & 0x80 != 0) {
                if self.frame_bg.borrow()[p2] != CLEAR_COLOR {
                    sprite_0_hit |= true;
                }
            }
        }
        sprite_0_hit
    }

    fn palette_to_color(&self, i: usize) -> u8 {
        if i % 4 == 0 {
            return CLEAR_COLOR
        }
        self.palette_ram[i]
    }

    // debug
    pub fn draw_chr(&self, frame: &mut [u8]) {

        for j in 0..2usize {
            let chr_base = j * 0x1000;
            for i in 0..256 {
                let base = i * 16 + chr_base;
                let pattern0 = &self.pattern_table[base .. (base + 8)];
                let pattern1 = &self.pattern_table[(base + 8).. (base + 16)];
    
                for y_pattern in 0..8 {
                    let line0 = pattern0[y_pattern];
                    let line1 = pattern1[y_pattern];
    
                    for x_pattern in 0..8 {
                        let pattern_bit = (7 - (x_pattern % 8)) as usize;
                        let palette_num = ((line0 >> pattern_bit) & 1 | ((line1 >> pattern_bit) & 1) << 1) as usize;
    
                        let x_base = i % 16 + j * 16;
                        let y_base = i / 16;
                        
                        let base = ((y_base * 8 + y_pattern) * CHR_DEBUG_WIDTH + x_base * 8 + x_pattern) * 4;
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

    }

    pub fn draw_name_table(&self, frame_: &RefCell<Vec<u8>>) {
        let mut frame = frame_.borrow_mut();
        self.draw_name_table_(|x,y,c| {
            let i = (x + y * WIDTH * 2) * 4;
            let color = if c == CLEAR_COLOR as usize {
                &COLORS[self.palette_ram[0] as usize]
            } else {
                &COLORS[c]
    
            };
            frame[i..i+3].clone_from_slice(color);
            frame[i+3] = 0xff;

        });
    }

    // debug
    pub fn draw_name_table_<F>(&self, mut f: F) where F : FnMut(usize, usize, usize) {
        let chr_base = if self.ppuctrl & (1 << 4) != 0 { 0x1000 } else { 0x0000 };

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
    
                    // nameテーブルは8x8px単位
                    // https://www.nesdev.org/wiki/PPU_nametables
                    // https://taotao54321.hatenablog.com/entry/2017/04/11/115205
                    let name_y_base = attr_y * 4;
                    let name_x_base = attr_x * 4;
                    for name_y in name_y_base..name_y_base + 4 {
                        for name_x in name_x_base..name_x_base + 4 {
                            let shift_bit = (((name_x%4)/2) + ((name_y%4)/2)*2)*2;
                            let palette_index = (attribute as usize >> shift_bit) & 3;
    
                            let name_index = base_addr + (name_x + name_y * 32) as usize;
                            let pattern_index = self.name_table[name_index] as usize;
    
                            let y_base = name_y * 8;
                            let x_base = name_x * 8;
                            for y in y_base..y_base+8 {
                                let pattern_y = (y % 8) as usize;
    
                                let pattern0 = self.pattern_table[chr_base + pattern_index * 16 + pattern_y];
                                let pattern1 = self.pattern_table[chr_base + pattern_index * 16 + pattern_y + 8];

                                for x in x_base..x_base+8 {

                                    let pattern_bit = (7 - (x % 8)) as usize;
                                    let palette_num = ((pattern0 >> pattern_bit) & 1 | ((pattern1 >> pattern_bit) & 1) << 1) as usize;
    
                                    let color = if palette_num % 4 != 0 {
                                        self.palette_ram[palette_index * 4 + palette_num] as usize 
                                    } else {
                                        CLEAR_COLOR as usize
                                    };

                                    let x_ = if i % 2 == 1 { x + WIDTH } else { x };
                                    let y_ = if i / 2 == 1 { y + HEIGHT } else { y };

                                    if x_ < WIDTH*2 && y_ < HEIGHT*2 {
                                        f(x_, y_, color);
                                    }

                                }
                            }
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

#[cfg(test)]
mod tests {

    fn a(a: u16) -> u16 {
        a & !0x400        
    }
    fn v(a: u16) -> u16 {
        a & !0x800        
    }
    #[test]
    #[ignore]
    fn test() {
        assert_eq!(a(1), 1);
        assert_eq!(a(0x401), 0x1);
        assert_eq!(a(0x801), 0x801);
        assert_eq!(a(0xc01), 0x801);

        assert_eq!(v(1), 0x001);
        assert_eq!(v(0x401), 0x401);
        assert_eq!(v(0x801), 0x001);
        assert_eq!(v(0xc01), 0x401);
        
    }
}