pub mod cpu;
pub mod bus;
pub mod ppu;
pub mod joypad;
pub mod hex;
pub mod apu_impl;
pub mod mapper;
pub mod rom;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::window::{WindowBuilder, Window};
use winit_input_helper::WinitInputHelper;

use mapper::new_mapper;

use joypad::PadKey;
use cpu::{CPU, CpuDebugLog, CPU_CLOCK_UNIT_NSEC};
use bus::Bus;
use rom::parse_header;
use ppu::{WIDTH, HEIGHT, CHR_DEBUG_FRAME_SIZE, CHR_DEBUG_WIDTH, CHR_DEBUG_HEIGT, SPRITE_DEBUG_WIDTH, SPRITE_DEBUG_HEIGT};

#[derive(Debug)]
struct FpsCounter {
    start_time : Instant,
    pub frame_count : usize,
    pub fps : f32,
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            start_time : Instant::now(),
            frame_count : 0,
            fps : 0f32,
        }
    }

    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.frame_count = 0;
        self.fps = 0f32;
    }

    pub fn add_frame(&mut self) {
        let time = Instant::now().duration_since(self.start_time).as_secs_f32();
        self.frame_count += 1;
        self.fps = (self.frame_count as f32) / time;
    }
}

pub struct FamikoOption {
    pub start_addr : Option<u16>,
    pub debug : bool,
    pub sound_debug : bool,
    pub no_sound : bool,
    pub show_chr_table : bool,
    pub show_name_table : bool,
    pub show_sprite : bool,
    pub is_show_fps : bool,
    pub rom_bytes : Vec<u8>,
}

struct FamikoRunner {
    cpu : CPU,
    debug : bool,
    time_base : Instant,
    elapsed_time : u128,
    fps_counter : Option<FpsCounter>,
}

impl FamikoRunner {
    fn new(
        start_addr : Option<u16>,
        debug : bool,
        sound_debug : bool,
        no_sound : bool,
        show_chr_table : bool,
        show_name_table : bool,
        show_sprite : bool,
        is_show_fps : bool,
        rom_bytes : Vec<u8>,
    ) -> Self {
        let h = parse_header(&rom_bytes).unwrap();

        let mut p : usize = 16;
        let prg_rom = Vec::from(&rom_bytes[p .. p + h.prg_size]);
        p += h.prg_size;
        let chr_rom = Vec::from(&rom_bytes[p .. p+h.chr_size]);

        let mapper = Rc::new(RefCell::new(new_mapper(h.mapper, prg_rom, chr_rom)));

        let bus = Bus::new(mapper, h.flag6 & 1 == 0, sound_debug,no_sound);
        let mut cpu = CPU::new(bus);

        // apu開始
        _ = cpu.bus.apu.start();
    
        // 電源ON
        if let Some(start_addr) = start_addr {
            println!("start addr {start_addr:04x}");
            cpu.jmp_int_handler(start_addr);
        } else {
            cpu.int_reset();
        }
        cpu.bus.ppu.step(7*3);

        Self {
            cpu: cpu,
            debug: debug,
            time_base : Instant::now(),
            elapsed_time : 0u128,
            fps_counter : if is_show_fps { Some(FpsCounter::new()) } else { None }
        }
    }

    fn update_keys(&mut self,  keys: Vec<(usize, bool)>) {
        for (k, b) in keys {
            self.cpu.bus.joy_pad.update_key(k as usize, b);
        }
    }

    // 消費したclockぶんだけ内部時刻を進める。
    // 内部時刻を実時刻が追い越したら、次の命令サイクルを消費する。
    fn run(&mut self) -> Option<Vec<u8>> {

        let actual = Instant::now().duration_since(self.time_base).as_nanos();
        
        let mut ret : Option<Vec<u8>> = None;

        while actual > self.elapsed_time {
            let mut log = CpuDebugLog::new();
            log.ppu_line = self.cpu.bus.ppu.y_();
            log.ppu_x = self.cpu.bus.ppu.x_();
            let cycle = self.cpu.step_next(&mut log);
            if self.debug {
                log.log();
            }
            let frame = self.cpu.bus.ppu.step(cycle*3);

            self.cpu.bus.apu.step(cycle);
            self.elapsed_time += (cycle as u128) * CPU_CLOCK_UNIT_NSEC;

            
            if let Some(b) = frame {
                if let Some(fps) = &mut self.fps_counter {
                    fps.add_frame();
                    println!("fps:{}", fps.fps);
        
                    if fps.frame_count > 60 {
                        fps.reset();
                    }
                }
                ret = Some(*b)
            }
        }
        
        ret

    }
}

pub fn main(option : &FamikoOption) -> Result<(), Box<dyn std::error::Error>> {

    let h = parse_header(&option.rom_bytes).unwrap();

    let mut p : usize = 16;
    let prg_rom = Vec::from(&option.rom_bytes[p .. p + h.prg_size]);
    p += h.prg_size;
    let chr_rom = Vec::from(&option.rom_bytes[p .. p+h.chr_size]);

    // println!("{:?}", h);
    // println!("{:?}", prg_rom.hex_dump());
    // println!("{:?}", chr_rom.hex_dump());

    let mut famiko_runner = FamikoRunner::new(
        option.start_addr,
        option.debug,
        option.sound_debug,
        option.no_sound,
        option.show_chr_table,
        option.show_name_table,
        option.show_sprite,
        option.is_show_fps,
        option.rom_bytes.clone(),
    );

    // 画面表示
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let (window, mut pixels) = create_window("Famiko".into(), WIDTH as u32, HEIGHT as u32, &event_loop).unwrap();


    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event:  WindowEvent::Resized(size), window_id: win_id } if win_id == window.id() => {
                pixels.resize_surface(size.width, size.height);
            }
            Event::RedrawRequested(win_id) if win_id == window.id() => {
                pixels.render().unwrap();
            }
            Event::MainEventsCleared => {

                if let Some(buffer) = famiko_runner.run() {
                    pixels.get_frame().copy_from_slice(buffer.as_slice());

                    window.request_redraw();
                }

            },
            _ => {
            }
        }

        // Handle input events
        if input.update(&event) {
    
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
            }
            
            let joy_and_code = [
                (joypad::A, VirtualKeyCode::Z),
                (joypad::B, VirtualKeyCode::X),
                (joypad::SELECT, VirtualKeyCode::C),
                (joypad::START, VirtualKeyCode::V),
                (joypad::UP, VirtualKeyCode::Up),
                (joypad::DOWN, VirtualKeyCode::Down),
                (joypad::RIGHT, VirtualKeyCode::Right),
                (joypad::LEFT, VirtualKeyCode::Left),
                ];
            let mut ks : Vec<(usize, bool)> = vec!();
            for (key,code) in joy_and_code {
                if input.key_pressed(code) {
                    ks.push((key, true));
                } else if input.key_released(code) {
                    ks.push((key, false));
                }
            }

            if !ks.is_empty() {
                famiko_runner.update_keys(ks);
            }
            
        };


    });
}

fn create_window<T>(
    title: String,
     w: u32,
     h: u32,
     target: &EventLoopWindowTarget<T>) -> Result<(Window, Pixels), pixels::Error> where T: 'static, {

    let size = LogicalSize::new(w, h);
    let win = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(size)
        .with_min_inner_size(size)
        .build(&target).unwrap();
    
    let window_size = win.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &win);
    let p = Pixels::new(w, h, surface_texture)?;

    Ok((win, p))
}
