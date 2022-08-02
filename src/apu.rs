
use core::fmt;
use std::usize::MIN;

use pa::{Stream, Blocking, Output, StreamAvailable, OutputStreamSettings};
use portaudio as pa;

const CHANNELS: i32 = 1;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = 512;

static DUTY_TABLE : [[u8;8];4] = [
    [0,1,0,0,0,0,0,0],
    [0,1,1,0,0,0,0,0],
    [0,1,1,1,1,0,0,0],
    [1,0,0,1,1,1,1,1],
];

pub struct Apu {
    stream: Option<Stream<Blocking<pa::stream::Buffer>, Output<f32>>>,

    pulse1_reg1 : u8, // $4000 DDLC VVVV  Duty (D), envelope loop / length counter halt (L), constant volume (C), volume/envelope (V)
    pulse1_reg2 : u8, // $4001 EPPP NSSS  Sweep unit: enabled (E), period (P), negate (N), shift (S)
    pulse1_reg3 : u8, // $4002 TTTT TTTT  Timer low (T)
    pulse1_reg4 : u8, // $4003 LLLL LTTT  Length counter load (L), timer high (T)

    status_reg : u8, // $4015 ---D NT21  Enable DMC (D), noise (N), triangle (T), and pulse channels (2/1)
    frame_counter_reg : u8, // $4017 MI-- ----  Mode (M, 0 = 4-step, 1 = 5-step), IRQ inhibit flag (I)

    pulse1_timer_divider : u64, // reg3,reg4の (T << 5)を初期値として毎クロック(-1)する。0になったら`pulse1_timer2`を+1
    pulse1_timer_step : u8, // 0-8の値。この値で`DUTY_TABLE`から取り出した値を出力値とする
    pulse1_seq_diveder : u16, // 0-7456。毎クロック(-1)し、7457クロックごとにpulse1_seq_stepを(+1)する
    pulse1_seq_step : u8, // 7457クロックごとに(+1)。Mによって、0-3または0-4の値をとる。
    pulse1_sweep : u8,
    
    pulse1_envelope_reset : bool, // pulse1_reg4に書き込みがあったらにtrueにする。trueであればenvelopeのクロック（シーケンサの各クロック）でdivider, counterをリセット
    pulse1_envelope_divider : u8, // envelopeの分周の実装。初期値はreg1のVVVVを与える。クロックごとに-1して、0のときにcouterを処理。
    pulse1_envelope_counter : u8, // 0-15の値。envelopeのクロックごとに(-1)する。0のときにloopが有効であれば15にする。

    pulse1_sample_output_couter : f32,
    pulse1_value : u8,
    pulse1_buffer : Vec<f32>,

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
            frame_counter_reg: 0u8,

            pulse1_timer_divider : 0,
            pulse1_timer_step : 0,
            pulse1_seq_diveder : 0,
            pulse1_seq_step : 0,
            pulse1_sweep : 0,
            
            pulse1_envelope_reset : false, // pulse1_reg4に書き込みがあったらにtrueにする。trueであればenvelopeのクロック（シーケンサの各クロック）でdivider, counterをリセット
            pulse1_envelope_divider : 0, // envelopeの分周の実装。初期値はreg1のVVVVを与える。クロックごとに-1して、0のときにcouterを処理。
            pulse1_envelope_counter : 0, // 0-15の値。envelopeのクロックごとに(-1)する。0のときにloopが有効であれば15にする。
        
            pulse1_sample_output_couter : 0f32,
            pulse1_value : 0u8,
            pulse1_buffer : Vec::<f32>::new(),
        
        }
    }

    pub fn start(&mut self) -> Result<(), pa::Error>{
    
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

                let t = (self.pulse1_reg3 as u64) | ((self.pulse1_reg4 as u64 & 0x07) << 8);

                self.pulse1_timer_divider = t << 5;
                self.pulse1_seq_step = 0;
                self.pulse1_envelope_reset = true;
            }
            _ => {
            }
        };
    }

    // 1.789MHzのクロックで呼ばれる想定
    // 戻り値はIRQが発生したことを知らせる
    // 44.1kHzで音を出力する場合は40.58クロックにごとに1サンプルを出力 (1/44.1K)/(1/1789773)=1789773/44100=40.58
    pub fn step(&mut self) {

        if true {
            let t = (self.pulse1_reg3 as u64) | ((self.pulse1_reg4 as u64 & 0x07) << 8);
            let duty_type = self.pulse1_reg1 >> 6;

            // タイマー
            if self.pulse1_timer_divider != 0 {
                self.pulse1_timer_divider -= 1;
            } else {
                self.pulse1_timer_divider = t << 5;
                self.pulse1_timer_step += 1;
            }
            // シーケンサ
            let value = match duty_type {
                0..=3 => { DUTY_TABLE[duty_type as usize][self.pulse1_timer_step as usize] }
                _ => panic!("duty_type error {:?}", duty_type)
            };

            // スイープ
            // TODO

            // フレームシーケンサー
            if self.pulse1_seq_diveder != 0 {
                self.pulse1_seq_diveder -= 1;
            } else {
                self.pulse1_seq_diveder = 7467;

                if self.frame_counter_reg & (1u8 << 7) == 0 {
                    // 4step mode
                    match self.pulse1_seq_step {
                        0 => {}
                        1 => {}
                        2 => {}
                        3 => {}
                    }
                } else {
                    // 5step mode
                    match self.pulse1_seq_step {
                        0 => {}
                        1 => {}
                        2 => {}
                        3 => {}
                        4 => {}
                    }
                }

            }

            // エンベロープ

            // レングス




            self.pulse1_sample_output_couter += 1f32;
            let sample_output_count = 1789773f32/44100.0;
            if self.pulse1_sample_output_couter > sample_output_count {
                self.pulse1_sample_output_couter =- sample_output_count;

                let v = (self.pulse1_value as f32) * 1.5 / 15.0 - 1.0;
                self.pulse1_buffer.push(v);

                let buffer_len = self.pulse1_buffer.len();

                if buffer_len >= FRAMES_PER_BUFFER as usize {

                    if let Some(stream) = &mut self.stream {
                        match stream.write_available() {
                            Ok(StreamAvailable::Frames(l)) => {
                                let write_len = if buffer_len > l as usize {
                                    l as usize
                                } else {
                                    buffer_len
                                };
                                

                                let write_frame = &self.pulse1_buffer[0..write_len];
                                if l > (FRAMES_PER_BUFFER as i64) {
                                    let r = stream.write((FRAMES_PER_BUFFER) as u32, |output|{
                                        // output.copy_from_slice(&write_frame);

                                        for i in 0 ..(FRAMES_PER_BUFFER) as usize{
                                            // output[i] = write_frame[i];
                                            output[i] = self.saw;
                                        }
    
                                        //print!("");
                                    });
                                    if let Err(e) = r {
                                        println!("{:?}", e);
                                    }
                                }

                                let remain = buffer_len - write_len;
                                self.pulse1_buffer.clear();
                            
                            },
                            Ok(StreamAvailable::OutputUnderflowed) => { println!("OutputUnderflowed"); },
                            Ok(StreamAvailable::InputOverflowed) => { println!("InputOverflowed");},
                            Err(err) => { println!("err {:?}", err); }
                        }
                    }
                }
            }


            
        }

    }

}