
use core::fmt;

use pa::{Stream, Blocking, Output, StreamAvailable, OutputStreamSettings};
use portaudio as pa;

const CHANNELS: i32 = 1;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = 64;

pub struct Apu {
    stream: Option<Stream<Blocking<OutputStreamSettings<f32>::Flow>, Output<f32>>>,

    pulse1_reg1 : u8, // $4000 DDLC VVVV  Duty (D), envelope loop / length counter halt (L), constant volume (C), volume/envelope (V)
    pulse1_reg2 : u8, // $4001 EPPP NSSS  Sweep unit: enabled (E), period (P), negate (N), shift (S)
    pulse1_reg3 : u8, // $4002 TTTT TTTT  Timer low (T)
    pulse1_reg4 : u8, // $4003 LLLL LTTT  Length counter load (L), timer high (T)

    pub status_reg : u8, // $4015 ---D NT21  Enable DMC (D), noise (N), triangle (T), and pulse channels (2/1)

    pulse1_start : bool,
    pulse1_timer_count : u64,
    pulse1_sequencer_counter : u64,
    pulse1_buffer : Vec<f32>,
    pulse1_sample_output_couter : f32,
    pulse1_state : f32,

    pulse1_step : usize, // 

}

impl fmt::Debug for Apu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Apu")
         .finish()
    }
}

impl Apu {
    pub fn new() -> Self {
        Self{
            stream: None,
            pulse1_reg1 : 0u8,
            pulse1_reg2 : 0u8,
            pulse1_reg3 : 0u8,
            pulse1_reg4 : 0u8,

            status_reg : 0u8,
            pulse1_start : false,
            pulse1_timer_count : 0u64,
            pulse1_sequencer_counter : 0,
            pulse1_buffer : vec![],
            pulse1_sample_output_couter : 0f32,
            pulse1_state : 0f32,

            pulse1_step : 0usize,
        }
    }

    pub fn start(&mut self) -> Result<(), pa::Error>{
        println!(
            "PortAudio Test: output sawtooth wave. SR = {}, BufSize = {}",
            SAMPLE_RATE, FRAMES_PER_BUFFER
        );
    
        let mut left_saw = 0.0;
    
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
        match addr {
            0x4000 => self.pulse1_reg1,
            0x4001 => self.pulse1_reg2,
            0x4002 => self.pulse1_reg3,
            0x4003 => self.pulse1_reg4,
            0x4015 => self.status_reg,
            _ => 0u8,
        }
    }

    pub fn write(&mut self, addr : u16, v : u8) {
        match addr {
            0x4000 => self.pulse1_reg1 = v,
            0x4001 => self.pulse1_reg2 = v,
            0x4002 => self.pulse1_reg3 = v,
            0x4003 => { 
                self.pulse1_reg4 = v;
                self.pulse1_timer_count = 0;
                self.pulse1_sequencer_counter = 0;
            }
            _ => {
            }
        };
    }

    // 1.789MHzのクロックで呼ばれる想定
    // 戻り値はIRQが発生したことを知らせる
    // 44.1kHzで音を出力する場合は40.58クロックにごとに1サンプルを出力 (1/44.1K)/(1/1789773)=1789773/44100=40.58
    pub fn step(&mut self) -> bool {
        let duty_array = [
            [0,1,0,0,0,0,0,0],
            [0,1,1,0,0,0,0,0],
            [0,1,1,1,1,0,0,0],
            [1,0,0,1,1,1,1,1],
        ];

        if self.status_reg & 1 != 0 {
            let reg = (self.pulse1_reg3 as u64) | ((self.pulse1_reg4 as u64 & 0x07) << 8);
            let duty_type = self.pulse1_reg1 >> 6;

            if reg < 8 {
                return false;
            }
            let reg_t = reg << 5 + 1;
            
            if self.pulse1_timer_count > 0 {
                self.pulse1_timer_count -= 0;

                if self.pulse1_timer_count == 0 {
                    self.pulse1_timer_count = reg_t / 2;

                    self.pulse1_step = (self.pulse1_step + 1) % 2;
                }
            }

            self.pulse1_sample_output_couter += 1f32;
            let sample_output_count = 1789773f32/44100.0;
            if self.pulse1_sample_output_couter > sample_output_count {
                self.pulse1_sample_output_couter - sample_output_count;

                if self.pulse1_timer_count > 0 {
                    let value = if self.pulse1_step == 0 {
                        1f32
                    } else {
                        0f32
                    };
                    self.pulse1_buffer.push(value);
                }

                let write_len = self.pulse1_buffer.len();
                if write_len > FRAMES_PER_BUFFER as usize {
                    if let Some(stream) = self.stream {
                        match stream.write_available() {
                            Some(StreamAvailable::Frames(l)) => {
                                let l = l as usize;
                                if write_len > l{
                                    let write_frame = &self.pulse1_buffer[0..l];
                                    stream.write(l, |output|{
                                        output.copy_from_slice(&write_frame);
                                    });
                                    let remain = write_len - l as usize;
                                    
                                }
                            },
                            Some(StreamAvailable::OutputUnderflowed) => { println!("OutputUnderflowed"); },
                            Some(StreamAvailable::InputOverflowed) => { println!("InputOverflowed");},
                            None => { println!("None"); }
                        }
                    }
                }
            }


            
        }

        false
    }

}