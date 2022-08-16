
use core::fmt;
use std::{usize::MIN, collections::VecDeque, time::{self, Instant}, fs::{self, File}, io::BufWriter};

use hound::WavWriter;
use pa::{Stream, Blocking, Output, StreamAvailable, OutputStreamSettings};
use portaudio as pa;
use apu::{self, Apu};

const CHANNELS: i32 = 1;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = 512;

pub struct ApuImpl {
    stream: Option<Stream<Blocking<pa::stream::Buffer>, Output<f32>>>,
    apu : Apu,
    pub irq : bool,

    is_debug : bool,
    debug_writer : Option<WavWriter<BufWriter<File>>>,
    debug_writer_num : usize,
    debug_writer_length : usize,
}

impl fmt::Debug for ApuImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Apu")
         .finish()
    }
}

impl ApuImpl {
    pub fn new(is_debug : bool) -> Self {
        Self{
            stream: None,
            apu : Apu::new(),
            irq : false,
            is_debug : is_debug,
            debug_writer : None,
            debug_writer_num : 0,
            debug_writer_length : 0,
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
        println!("apu stop");
        if let Some(st) = &mut self.stream {
            (*st).stop()?
        }
        self.stream = None;
        Ok(())
    }

    fn debug_start(&mut self) {
        self.debug_stop();

        let test_output = "test_output";
        _ = fs::create_dir(test_output);
        let file_name = format!("test_output/debug{}.wav", self.debug_writer_num);
        self.debug_writer_num += 1;

        let wav_spec = hound::WavSpec {
            channels : 1,
            sample_rate : 44100,
            bits_per_sample : 32,
            sample_format : hound::SampleFormat::Float,
        };

        self.debug_writer = Some(hound::WavWriter::create(file_name, wav_spec).unwrap());

        self.debug_writer_length  = 0;
    }

    fn debug_stop(&mut self) {
        let d = std::mem::take(&mut self.debug_writer);
        if let Some(w) = d {
            _ = w.finalize();
            self.debug_writer = None;
        }
    }

    fn debug_write(&mut self) {
        if let None = self.debug_writer {
            self.debug_start();
        }
        let len = self.apu.frames.len();
        if let Some(w) = self.debug_writer.as_mut() {
            for i in 0..len {
                _ = w.write_sample(self.apu.frames[i]);
            }
        }
        self.debug_writer_length += len;
        println!("{:}", self.debug_writer_length);
        if self.debug_writer_length > 44_100 * 10 {
            println!("flush");
            self.debug_stop();
        }
    }

    pub fn read(&mut self, addr : u16, is_debug : bool) -> u8 {
        if !is_debug {
            // println!("apu read {:04x} {:}", addr, is_debug);
        }
        self.apu.read(addr, is_debug)
    }

    pub fn write(&mut self, addr : u16, v : u8) {
        // println!("apu write {:04x} {:02x}", addr, v);
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

                            if self.is_debug {
                                self.debug_write();
                            }

                            self.apu.frames.clear();
                        }
                    },
                    Ok(StreamAvailable::OutputUnderflowed) => { println!("OutputUnderflowed"); },
                    Ok(StreamAvailable::InputOverflowed) => { println!("InputOverflowed");},
                    Err(err) => { println!("err {:?}", err); }
                }
            } else {
                println!("if else ..");
            }
        }
        
    }

}