pub mod cpu;
pub mod bus;
pub mod ppu;
pub mod joypad;
pub mod hex;
pub mod apu_impl;
pub mod mapper;
pub mod rom;

use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

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
enum RenderEvent {
    Render(Vec<u8>),
    ChrTableRender(Vec<u8>),
    NameTableRender(RefCell<Vec<u8>>),
    SpriteRender(Vec<u8>),
}

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

pub fn main(option : &FamikoOption) -> Result<(), Box<dyn std::error::Error>> {

    let h = parse_header(&option.rom_bytes).unwrap();

    let mut p : usize = 16;
    let prg_rom = Vec::from(&option.rom_bytes[p .. p + h.prg_size]);
    p += h.prg_size;
    let chr_rom = Vec::from(&option.rom_bytes[p .. p+h.chr_size]);

    // println!("{:?}", h);
    // println!("{:?}", prg_rom.hex_dump());
    // println!("{:?}", chr_rom.hex_dump());


    // 画面情報をUIスレッドに転送するチャネル
    let (render_sender, render_receiver) = mpsc::channel::<RenderEvent>();

    // キー情報をUIスレッドから転送するチャネル
    let (key_sender, key_receiver) = mpsc::channel::<(PadKey, bool)>();

    let start_addr = option.start_addr;
    let sound_debug = option.sound_debug;
    let no_sound = option.no_sound;
    let debug = option.debug;
    let show_chr_table = option.show_chr_table;
    let show_name_table = option.show_name_table;
    let show_sprite = option.show_sprite;
    let is_show_fps = option.is_show_fps;

    thread::spawn(move ||{
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


        let mut fps = FpsCounter::new();

        let mut elapsed_time = 0u128;
        let time_base = Instant::now();
        
        loop {
            let mut log = CpuDebugLog::new();
            log.ppu_line = cpu.bus.ppu.y_();
            log.ppu_x = cpu.bus.ppu.x_();
            let cycle = cpu.step_next(&mut log);
            if debug {
                log.log();
            }
            let frame_ = cpu.bus.ppu.step(cycle*3);

            cpu.bus.apu.step(cycle);

            elapsed_time += (cycle as u128) * CPU_CLOCK_UNIT_NSEC;
            let actual = Instant::now().duration_since(time_base).as_nanos();
            if elapsed_time > actual && elapsed_time - actual > 1_000_000 { // 1.2msec
                let t = elapsed_time - actual;
                sleep(Duration::from_nanos(t as u64));
            }

            if let Some(f) = frame_ {

                render_sender.send(RenderEvent::Render(*f)).unwrap();

                if show_chr_table {
                    let mut draw_chr_frame = [0u8].repeat(CHR_DEBUG_FRAME_SIZE*4);
                    cpu.bus.ppu.draw_chr(draw_chr_frame.as_mut_slice());
                    render_sender.send(RenderEvent::ChrTableRender(draw_chr_frame)).unwrap();
                }
                if show_name_table {
                    let draw_name_frame = RefCell::new(vec![0u8;256*240*4*4]);
                    cpu.bus.ppu.draw_name_table(&draw_name_frame);
                    render_sender.send(RenderEvent::NameTableRender(draw_name_frame)).unwrap();
                }
                if show_sprite {
                    let mut frame = Some([0u8].repeat(64*64*4));
                    cpu.bus.ppu.write_sprite(&mut frame);
                    render_sender.send(RenderEvent::SpriteRender(frame.unwrap())).unwrap();
                }

                if is_show_fps {
                    fps.add_frame();
                    println!("fps:{}", fps.fps);

                    if fps.frame_count > 60 {
                        fps = FpsCounter::new();
                    }
                }
                if let Ok((k, b)) = key_receiver.try_recv() {
                    cpu.bus.joy_pad.update_key(k, b);
                }
            }
        };
    });


    // 画面表示
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let (window, mut pixels) = create_window("Famiko".into(), WIDTH as u32, HEIGHT as u32, &event_loop).unwrap();

    // デバッグ用表示
    let mut chr_table_window = if option.show_chr_table {
        Some(create_window("chr_table".into(), CHR_DEBUG_WIDTH as u32, CHR_DEBUG_HEIGT as u32, &event_loop)?)
    } else {
        None
    };
    let mut sprite_table_window = if option.show_sprite {
        Some(create_window("sprite_table".into(), SPRITE_DEBUG_WIDTH as u32, SPRITE_DEBUG_HEIGT as u32, &event_loop)?)
    } else {
        None
    };
    let mut name_table_window = if option.show_name_table {
        Some(create_window("name_table".into(), WIDTH as u32 *2, HEIGHT as u32 *2, &event_loop)?)
    } else {
        None
    };

    event_loop.run(move |event, _, control_flow| {
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
            Event::WindowEvent { event:  WindowEvent::Resized(size), window_id: win_id } => {
                if let Some((w, p)) = chr_table_window.borrow_mut() {
                    if w.id() == win_id {
                        p.resize_surface(size.width, size.height);
                    }
                }
                if let Some((w, p)) = sprite_table_window.borrow_mut() {
                    if w.id() == win_id {
                        p.resize_surface(size.width, size.height);
                    }
                }
                if let Some((w, p)) = name_table_window.borrow_mut() {
                    if w.id() == win_id {
                        p.resize_surface(size.width, size.height);
                    }
                }
            }
            Event::RedrawRequested(win_id) if win_id == window.id() => {
                pixels.render().unwrap();
            }
            Event::RedrawRequested(win_id) => {
                if let Some((w, p)) = chr_table_window.borrow_mut() {
                    if w.id() == win_id {
                        p.render().unwrap();
                    }
                }
                if let Some((w, p)) = sprite_table_window.borrow_mut() {
                    if w.id() == win_id {
                        p.render().unwrap();
                    }
                }
                if let Some((w, p)) = name_table_window.borrow_mut() {
                    if w.id() == win_id {
                        p.render().unwrap();
                    }
                }
            }
            Event::MainEventsCleared => {
                match render_receiver.try_recv() {
                    Ok(RenderEvent::Render(buffer)) => pixels.get_frame().copy_from_slice(buffer.as_slice()),
                    Ok(RenderEvent::ChrTableRender(buffer)) => {
                        if let Some((_, p)) = chr_table_window.borrow_mut() {
                            p.get_frame().copy_from_slice(buffer.as_slice());
                        }
                    }
                    Ok(RenderEvent::SpriteRender(buffer)) => {
                        if let Some((_, p)) = sprite_table_window.borrow_mut() {
                            p.get_frame().copy_from_slice(buffer.as_slice());
                        }
                    }
                    Ok(RenderEvent::NameTableRender(buffer)) => {
                        if let Some((_, p)) = name_table_window.borrow_mut() {
                            p.get_frame().copy_from_slice(buffer.borrow().as_slice());
                        }
                    }
                    _ => {}
                }
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
            for (key,code) in joy_and_code {
                if input.key_pressed(code) {
                    key_sender.send((key, true)).unwrap();
                }
                if input.key_released(code) {
                    key_sender.send((key, false)).unwrap();
                }
            }

            // Update internal state and request a redraw
            window.request_redraw();
            name_table_window.as_ref().map(|(x, _)| { x.request_redraw() });
            chr_table_window.as_ref().map(|(x, _)| { x.request_redraw() });
            sprite_table_window.as_ref().map(|(x, _)| { x.request_redraw() });
        }
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
