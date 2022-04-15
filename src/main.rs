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

use famiko::cpu::CPU;
use famiko::bus::Bus;
use famiko::ppu::{WIDTH, HEIGHT};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = std::env::args().nth(1).expect("famiko <NES file path>");
    let mut file = File::open(file)?;
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

    let bus = Bus::new(prg_rom, chr_rom);
    let mut cpu = CPU::new(bus);

    // 電源ON
    cpu.int_reset();

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

    event_loop.run(move |event, _, control_flow| {
        // Draw the current frame
        if let Event::RedrawRequested(_) = event {
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
            cpu.bus.ppu.draw(pixels.get_frame());
            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Update internal state and request a redraw
            window.request_redraw();
        }
    });


}

#[allow(dead_code)]
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
