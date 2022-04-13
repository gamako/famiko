use std::fs::File;
use std::io::Read;
use pretty_hex::*;

use log::error;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 240;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open("./rom/hw.nes")?;
    let mut rom = Vec::new();
    
    let _ = file.read_to_end(&mut rom)?;
    // println!("{:?}", buf);

    let h = parse_header(&rom).unwrap();

    let mut p : usize = 16;
    let prg_rom = Vec::from(&rom[p .. p + h.prg_size]);
    p += h.prg_size;
    let chr_rom = Vec::from(&rom[p .. p+h.chr_size]);

    println!("{:?}", h);
    println!("{:?}", prg_rom.hex_dump());
    println!("{:?}", chr_rom.hex_dump());

    let bus = Bus::new(prg_rom);
    let mut cpu = CPU::new(bus);

    // 電源ON
    cpu.int_reset();

    println!("pc: {:#04x}", cpu.pc);

    // 画面表示
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Famiko")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };
    let mut world = World::new();

    event_loop.run(move |event, _, control_flow| {
        // Draw the current frame
        if let Event::RedrawRequested(_) = event {
            world.draw(pixels.get_frame());
            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // Handle input events
        if input.update(&event) {
            
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
            }

            cpu.step_next();

            // Update internal state and request a redraw
            world.update();
            window.request_redraw();
        }
    });


}

#[derive(Debug)]
struct NesHeader {
    prg : u8,
    prg_size : usize,
    chr : u8,
    chr_size : usize,
    flag6 : u8,
    // flag6
    // 76543210
    // ||||||||
    // |||||||+- Mirroring: 0: horizontal (vertical arrangement) (CIRAM A10 = PPU A11)
    // |||||||              1: vertical (horizontal arrangement) (CIRAM A10 = PPU A10)
    // ||||||+-- 1: Cartridge contains battery-backed PRG RAM ($6000-7FFF) or other persistent memory
    // |||||+--- 1: 512-byte trainer at $7000-$71FF (stored before PRG data)
    // ||||+---- 1: Ignore mirroring control or above mirroring bit; instead provide four-screen VRAM
    // ++++----- Lower nybble of mapper number
    trainer_exist : bool,
}

fn parse_header(buf : &[u8]) -> Result<Box<NesHeader>, Box<dyn std::error::Error>> {
    
    if buf.len() < 4 {
        panic!("header size error");
    }

    if buf[0] != 'N' as u8 || buf[1] != 'E' as u8 || buf[2] != 'S' as u8 || buf[3] != 0x1A {
        panic!("constant bytes error");
    }

    let prg = buf[4];
    let chr = buf[5];
    let flag6 = buf[6];



    Ok(Box::new(NesHeader{
        prg : prg,
        prg_size: prg as usize * 16 * 1024,
        chr : chr,
        chr_size: chr as usize * 8 * 1024,
        flag6 : flag6,
        trainer_exist : flag6 & 0x40 != 0
    }))
}

struct World {
}

impl World {
    fn new() -> Self {
        Self {
        }
    }

    fn update(&mut self) {
        ()
    }

    fn draw(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % WIDTH as usize) as i16;
            let y = (i / WIDTH as usize) as i16;

            let rgba = [0x48, 0xb2, 0xe8, 0xff];

            pixel.copy_from_slice(&rgba);
        }
    }
}

#[derive(Debug)]
struct CPU {
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    s: u8,
    pc: u16,

    bus : Bus
}

static p_mask_carry : u8 = 1 << 0;
static p_mask_zero : u8 = 1 << 1;
static p_mask_int_disable : u8 = 1 << 2;
static p_mask_decimal_mode : u8 = 1 << 3;
static p_mask_break_command : u8 = 1 << 4;
static p_mask_overflow : u8 = 1 << 5;
static p_mask_negative : u8 = 1 << 6;



impl CPU {

    fn new(bus : Bus) -> Self {
        CPU { a: 0, x: 0, y: 0, p: 0, s: 0, pc: 0, bus: bus }
    }

    fn int_reset(&mut self) {
        let l = self.bus.read(0xFFFC);
        let h = self.bus.read(0xFFFD);
        let addr = (h as u16) << 8 | l as u16;

        self.pc = addr;
    }

    // 1命令の実行
    fn step_next(&mut self) {
        let op = self.bus.read(self.pc);
        self.pc += 1;

        match op {
            0x78 => {
                // SEI : set i flag
                self.p |= p_mask_int_disable;
            }
            0x8d => {
                // STA absolute
                let l = self.bus.read(self.pc);
                self.pc += 1;
                let h = self.bus.read(self.pc);
                self.pc += 1;
                let addr = (h as u16) << 8 | l as u16;
                self.bus.write(addr, self.a);

            }
            0x9a => {
                // TXS
                self.s = self.x;
            }
            0xa0 => {
                // LDY imm
                let v = self.bus.read(self.pc);
                self.pc += 1;
                self.y = v;
                self.update_status_zero(v);
            }
            0xa2 => {
                // LDX imm
                let v = self.bus.read(self.pc);
                self.pc += 1;
                self.x = v;
                self.update_status_zero(v);
            }
            0xa9 => {
                // LDA imm
                let v = self.bus.read(self.pc);
                self.pc += 1;
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            }
            0xbd => {
                // LDA Absolute,X
                let l = self.bus.read(self.pc);
                self.pc += 1;
                let h = self.bus.read(self.pc);
                self.pc += 1;
                let addr = (h as u16) << 8 | l as u16 + self.x as u16;
                let v = self.bus.read(addr);
                self.a = v;
                self.update_status_zero(v);
                self.update_status_negative(v);
            }
            0x88 => {
                // DEY
                self.y -= 1;
                self.update_status_zero(self.y);
                self.update_status_negative(self.y);
            }
            0xe8 => {
                // INX
                self.x += 1;
                self.update_status_zero(self.x);
                self.update_status_negative(self.x);
            }
            0xd0 => {
                // BNE Rel
                let rel = self.bus.read(self.pc) as i8 as u16;
                self.pc += 1;
                if self.p & p_mask_zero == 0 {
                    println!("branch {}", rel);
                    println!("branch {:#04x} {:#04x}", self.pc, self.pc.wrapping_add(rel));
                    
                    self.pc = self.pc.wrapping_add(rel);
                }
            }
            0x4c => {
                // JMP Absolute
                let l = self.bus.read(self.pc);
                self.pc += 1;
                let h = self.bus.read(self.pc);
                self.pc += 1;
                let addr = (h as u16) << 8 | l as u16;
                self.pc = addr;
            }
            0x18 => {
                // CLC
                self.p &= !p_mask_carry
            }
            0x28 => {
                // PLP

            }
            _ => {
                println!("not impl {:#02x}", op);
                panic!("not impl error");
            }
        }
    }

    fn update_status_zero(&mut self, v : u8) {
        if v == 0 {
            self.p |= p_mask_zero
        } else {
            self.p &= !p_mask_zero
        }
    }
    fn update_status_negative(&mut self, v : u8) {
        if v & 0x80 != 0 {
            self.p |= p_mask_negative
        } else {
            self.p &= !p_mask_negative
        }
    }


}

#[derive(Debug)]
struct Bus {
    prg : Vec<u8>,
    ppu : PPU,
}

impl Bus {

    fn new(prg: Vec<u8>) -> Self {
        Bus { 
            prg: prg,
            ppu: PPU::new(),
        }
    }

    // https://www.nesdev.org/wiki/CPU_memory_map
    fn read(&self, addr: u16) -> u8 {
        if addr >= 0x8000 {
            let offset = addr - 0x8000;
            return self.prg[offset as usize];
        }
        println!("cant read {:#02x}", addr);
        panic!("not impl read addr");
    }
    // https://www.nesdev.org/wiki/CPU_memory_map
    fn write(&mut self, addr: u16, value: u8) {
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

#[derive(Debug)]
struct PPU {
    // https://www.nesdev.org/wiki/PPU_registers
    ppuctrl : u8,
    ppumask	: u8,
    ppustatus : u8,
    oamaddr : u8,
    oamdata : u8,
    ppuscroll : u8,
    ppuaddr : u8,
    ppudata : u8,
    oamdma: u8,

    addr: u16,
    scroll_x : u8,
    scroll_y : u8,
    scroll_next_y : bool,
    palette_ram : [u8; 0x20],
    name_table : [u8; 0x400 * 4],
}

impl PPU {
    fn new() -> Self {
        PPU { 
            ppuctrl: 0,
            ppumask: 0,
            ppustatus: 0,
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
         }
    }
    fn write_ppuscroll(&mut self, v : u8) {
        println!(" write scroll: {:#02x}", v);
        match self.scroll_next_y {
            false => { self.scroll_x = v }
            true => { self.scroll_y = v }
        }
        self.scroll_next_y = self.scroll_next_y;
    }
    fn write_ppuaddr(&mut self, v : u8) {
        self.addr = self.addr << 8 | v as u16;
    }
    // https://www.nesdev.org/wiki/PPU_memory_map
    fn write_ppudata(&mut self, v : u8) {
        println!(" ppu write {:04x} {:02x}", self.addr, v);
        match self.addr {
            0x2000 ..= 0x2fff => {
                println!(" write nametable: {:#04x} {:#02x}", self.addr, v);
                self.name_table[self.addr as usize - 0x2000] = v;
                self.addr += 1;
            }
            0x3f00 ..= 0x3f1f => {
                println!(" write palette_ram: {:#04x} {:#02x}", self.addr, v);
                self.palette_ram[self.addr as usize - 0x3f00] = v;
                self.addr += 1;
            }
            _ => {
                println!(" ppu cant write {:#02x}", self.addr);
                panic!("not impl ppu write addr");
            }
        }
    }
}