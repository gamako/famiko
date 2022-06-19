use std::fs::File;
use std::io::Read;
use std::sync::mpsc;
use std::thread::{self, sleep};
use std::time::{Instant, Duration};

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use famiko::cpu::{CPU, CpuDebugLog, CPU_CLOCK_UNIT_NSEC};
use famiko::bus::Bus;
use famiko::ppu::{WIDTH, HEIGHT, FRAME_SIZE};
use clap::{arg, command, Command};
use hex;

#[derive(Debug)]
enum RenderEvent {
    Render(Vec<u8>),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("famiko")
        .arg(arg!(--start_addr <addr> "開始アドレス"))
        .arg(
            arg!(
                -d --debug "Turn debugging information on"
            ),
        )
        .arg(arg!([rom] "rom"))
        .get_matches();
    
    // You can check the value provided by positional arguments, or option arguments
    let start_addr = if let Some(data) = matches.get_one::<String>("start_addr") {
        let v = hex::decode(data).unwrap();
        let addr = ((v[0] as u16) << 8) | (v[1] as u16);
        Some(addr)
    } else {
        None
    };
    let file = matches.get_one::<String>("rom").unwrap();

    let mut file = File::open(file)?;
    let mut rom = Vec::new();
    
    let _ = file.read_to_end(&mut rom)?;
    // println!("{:?}", buf);

    let h = parse_header(&rom).unwrap();

    let mut p : usize = 16;
    let prg_rom = Vec::from(&rom[p .. p + h.prg_size]);
    p += h.prg_size;
    let chr_rom = Vec::from(&rom[p .. p+h.chr_size]);

    // println!("{:?}", h);
    // println!("{:?}", prg_rom.hex_dump());
    // println!("{:?}", chr_rom.hex_dump());

    let bus = Bus::new(prg_rom, chr_rom);
    let mut cpu = CPU::new(bus);

    // 画面情報をUIスレッドに転送するチャネル
    let (render_sender, render_receiver) = mpsc::channel::<RenderEvent>();

    thread::spawn(move ||{

        // 電源ON
        if let Some(start_addr) = start_addr {
            // nestest用にc000から始める
            println!("start addr {start_addr:04x}");
            cpu.init_pc(start_addr, 7);
            cpu.bus.ppu.step(7*3);
        } else {
            cpu.int_reset();
        }

        let mut time = Instant::now();

        loop {
            let mut log = CpuDebugLog::new();
            log.ppu_line = cpu.bus.ppu.y_();
            log.ppu_x = cpu.bus.ppu.x_();
            let cycle = cpu.step_next(&mut log);
            log.log();
            let frame_ = cpu.bus.ppu.step(cycle*3);

            if let Some(f) = frame_ {
                time = Instant::now();
                render_sender.send(RenderEvent::Render(*f)).unwrap();
            }
            let t = (cycle * (CPU_CLOCK_UNIT_NSEC as usize)) as u64;
            sleep(Duration::from_nanos(t));
        };
    });


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
        Pixels::new(WIDTH as u32, HEIGHT as u32, surface_texture)?
    };

    event_loop.run(move |event, _, control_flow| {

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::RedrawRequested(_) => {
                pixels.render().unwrap();
            }
            Event::MainEventsCleared => match render_receiver.try_recv() {
                Ok(event) => match event {
                    RenderEvent::Render(buffer) => {
                        pixels.get_frame().copy_from_slice(buffer.as_slice());
                    }
                },
                _ => {}
            },
            _ => {}
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
