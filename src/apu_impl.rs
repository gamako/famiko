
use core::fmt;
use std::{usize::MIN, collections::VecDeque, time::{self, Instant}};

use pa::{Stream, Blocking, Output, StreamAvailable, OutputStreamSettings};
use portaudio as pa;
use apu::{self, Apu};

const CHANNELS: i32 = 1;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = 512;

pub struct ApuImpl {
    stream: Option<Stream<Blocking<pa::stream::Buffer>, Output<f32>>>,
    apu : Apu,
    pub irq : bool
}

impl fmt::Debug for ApuImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Apu")
         .finish()
    }
}

impl ApuImpl {
    pub fn new() -> Self {
        Self{
            stream: None,
            apu : Apu::new(),
            irq : false,
        }
    }

    pub fn start(&mut self) -> Result<(), pa::Error>{
    
        let pa = pa::PortAudio::new()?;
    
        let mut settings =
            pa.default_output_stream_settings(CHANNELS, SAMPLE_RATE, FRAMES_PER_BUFFER)?;
        // we won't output out of range samples so don't bother clipping them.
        settings.flags = pa::stream_flags::CLIP_OFF;
        
        let mut stream = pa.open_blocking_stream(settings)?;
        stream.start()?;
        self.stream = Some(stream);

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), pa::Error>{
        if let Some(st) = &mut self.stream {
            (*st).stop()?
        }
        self.stream = None;
        Ok(())
    }

    pub fn read(&mut self, addr : u16, is_debug : bool) -> u8 {
        self.apu.read(addr, is_debug)
    }

    pub fn write(&mut self, addr : u16, v : u8) {
        self.apu.write(addr, v);
    }

    pub fn step(&mut self, cycle: usize) {
        let is_irq = self.apu.step(cycle);
        self.flush_buffer_if_need();

        if is_irq {
            self.irq = true;
        }
    }

    fn flush_buffer_if_need(&mut self) {
        let buffer_len = self.apu.frames.len();
        
        if buffer_len >= FRAMES_PER_BUFFER as usize {

            if let Some(stream) = &mut self.stream {
                match stream.write_available() {
                    Ok(StreamAvailable::Frames(l)) => {
                        if l > (FRAMES_PER_BUFFER as i64) {
                            
                            let write_len = std::cmp::min(l as usize, buffer_len as usize);

                            // print!("{:?}/{:?}/{:?} ", write_len, buffer_len, l);

                            let r = stream.write(l as u32, |output|{

                                let mut i = 0;
                                let buffer_len = self.apu.frames.len();

                                while i < l as usize {
                                    let cp_len = std::cmp::min((l as usize -i), buffer_len);

                                    for j in i..i+cp_len {
                                        output[j] = self.apu.frames[j % buffer_len];
                                    }
                                    i += cp_len;
                                }

                                for i in 0..l as usize {
                                    output[i] = 0.0;
                                }
                            });
                            if let Err(e) = r {
                                println!("{:?}", e);
                            }
                            self.apu.frames.clear();
                        }
                    },
                    Ok(StreamAvailable::OutputUnderflowed) => { println!("OutputUnderflowed"); },
                    Ok(StreamAvailable::InputOverflowed) => { println!("InputOverflowed");},
                    Err(err) => { println!("err {:?}", err); }
                }
            }
        }
        
    }

}