use once_cell::sync::Lazy;

static DUTY_TABLE : [[u8;8];4] = [
    [0,1,0,0,0,0,0,0],
    [0,1,1,0,0,0,0,0],
    [0,1,1,1,1,0,0,0],
    [1,0,0,1,1,1,1,1],
];

static LENGTH_TABLE : [u8; 32] = [
    10,254, 20,  2, 40,  4, 80,  6, 160,  8, 60, 10, 14, 12, 26, 14,
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];

static FLAME_SEQ_4 : [(bool, bool, bool); 4] = [ // IRQ, LENGTH, ENVELOPEの順で、ステップごとの処理のありなし
    (false, false, true),
    (false, true, true),
    (false, false, true),
    (true, true, true),
];
static FLAME_SEQ_5 : [(bool, bool, bool); 5] = [
    (false, true, true),
    (false, false, true),
    (false, true, true),
    (false, false, true),
    (false, false, false),
];

static PULSE_TABLE : Lazy<[f32;31]> = Lazy::new(||{
    let mut t : [f32;31] = Default::default();
    for i in 0..31 {
        t[i] = 95.52 / (8128.0 / (i as f32) + 100.0)
    }
    t
});

#[derive(Debug)]
pub struct Pulse {
    pub reg_duty_type : usize,
    pub reg_envelope_loop_enable_and_length_is_disable : bool,
    pub reg_envelope_is_disabled : bool,
    pub reg_envelope_value : u8,
    pub reg_sweep_is_enabled : bool,
    pub reg_sweep_period : u8,
    pub reg_sweep_is_negate : bool,
    pub reg_sweep_shift : u8,
    pub reg_timer_low : u16,
    pub reg_timer_high : u16,
    pub reg_length_counter_type : u8,
    pub reg_is_reset : bool,
    pub reg_is_enable : bool,
    pub reg_5step_mode : bool,
    pub reg_enable_IRQ : bool,

    timer_divider : u16,
    timer_step : u8,
    sweep_divider : u8,
    envelope_divider : u8,
    envelope_counter : u8,
    length_counter : u8,
    sequencer_diveder : u16,
    sequencer_step : u8,
}

impl Pulse {
    pub fn new() -> Self {
        Self { 
            // $4000 DDLC VVVV  Duty (D), envelope loop / length counter halt (L), constant volume (C), volume/envelope (V)
            // $4001 EPPP NSSS  Sweep unit: enabled (E), period (P), negate (N), shift (S)
            // $4002 TTTT TTTT  Timer low (T)
            // $4003 LLLL LTTT  Length counter load (L), timer high (T)
            reg_duty_type: 0,
            reg_envelope_loop_enable_and_length_is_disable: false,
            reg_envelope_is_disabled : false,
            reg_envelope_value: 0,
            reg_sweep_is_enabled: false,
            reg_sweep_period: 0,
            reg_sweep_is_negate: false,
            reg_sweep_shift: 0,
            reg_timer_low: 0,
            reg_timer_high: 0,
            reg_length_counter_type: 0,
            reg_is_reset : true,
            reg_is_enable : true,
            reg_5step_mode : false,
            reg_enable_IRQ : true,

            timer_divider : 0,
            timer_step : 0,
            sweep_divider : 0, 
            envelope_divider : 0,
            envelope_counter : 0,
            length_counter : 0,
            sequencer_diveder : 0,
            sequencer_step : 0,

        }
    }

    pub fn write_reg1(&mut self, v : u8) {
        // $4000 DDLC VVVV  Duty (D), envelope loop / length counter halt (L), constant volume (C), volume/envelope (V)
        self.reg_duty_type = (v >> 6) as usize;
        self.reg_envelope_loop_enable_and_length_is_disable = v & (1 << 5) != 0;
        self.reg_envelope_is_disabled = v & (1 << 4) != 0;
        self.reg_envelope_value = v & 0b1111;
    }

    pub fn write_reg2(&mut self, v : u8) {
        // $4001 EPPP NSSS  Sweep unit: enabled (E), period (P), negate (N), shift (S)
        self.reg_sweep_is_enabled = (v & (1 << 7)) != 0;
        self.reg_sweep_period = (v >> 4) & 0b111;
        self.reg_sweep_is_negate = v & (1 << 3) != 0;
        self.reg_sweep_shift = v & 0b111;
    }

    pub fn write_reg3(&mut self, v : u8) {
        // $4002 TTTT TTTT  Timer low (T)
        self.reg_timer_low = v as u16;
    }

    pub fn write_reg4(&mut self, v : u8) {
        // $4003 LLLL LTTT  Length counter load (L), timer high (T)
        self.reg_length_counter_type = (v >> 3) & 0b11111;
        self.length_counter = LENGTH_TABLE[self.reg_length_counter_type as usize];
        self.reg_timer_high = (v & 0b111) as u16;
        self.reg_is_reset = true;
        self.reg_is_enable = true;
    }

    pub fn step(&mut self, cycle : usize) {
        for _ in 0..cycle {
            self.step_cycle();
        }
    }

    fn step_cycle(&mut self) {
        // タイマー
        if self.timer_divider != 0 {
            self.timer_divider -= 1;
        } else {
            self.timer_divider = (self.reg_timer_high << 8 | self.reg_timer_low + 1) << 1;
            self.timer_step = (self.timer_step + 1) % 8;
        }

        // フレームシーケンサー
        if self.sequencer_diveder != 0 {
            self.sequencer_diveder -= 1;
        } else {
            self.sequencer_diveder = 7467;

            let (step_max, (is_IRQ, is_length, is_envelope)) = if self.reg_5step_mode {
                (5, FLAME_SEQ_5[self.sequencer_step as usize])
            } else {
                (4, FLAME_SEQ_4[self.sequencer_step as usize])
            };
            self.sequencer_step = (self.sequencer_step + 1) % step_max;


            if is_length {
                self.step_sweep();
                self.step_length();
            }

            if is_envelope {
                self.step_envelope();
            }
        }

        self.reg_is_reset = false;
    }

    fn step_envelope(&mut self) {
        if !self.reg_envelope_is_disabled {
            if self.reg_is_reset {
                self.envelope_divider = self.reg_envelope_value;
                self.envelope_counter = 15;
    
            } else if self.envelope_divider == 0 {
                if self.envelope_counter == 0 {
                    if self.reg_envelope_loop_enable_and_length_is_disable {
                        // loop
                        self.envelope_counter = 15;
                    }
                } else {
                    self.envelope_counter -= 1;
                }
            } else {
                self.envelope_divider -= 1;
            }
        }
    }

    fn envelope_value(&self) -> u8 {
        if self.reg_envelope_is_disabled {
            self.reg_envelope_value
        } else {
            self.envelope_counter
        }
    }

    fn step_length(&mut self) {
        if self.reg_is_enable && !self.reg_envelope_loop_enable_and_length_is_disable {
            if self.length_counter == 0 {
                // pulse1をoff
                self.reg_is_enable = false;
            } else {
                self.length_counter -= 1;
            }
        }
    }

    fn step_sweep(&mut self) {
        if self.reg_sweep_is_enabled {
            if self.sweep_divider == 0 {
                self.sweep_divider = self.reg_sweep_period;
                let timer = self.reg_timer_high << 8 | self.reg_timer_low;
                let v = timer >> self.reg_sweep_shift;

                let timer = if self.reg_sweep_is_negate {
                    timer - v
                } else {
                    timer + v
                };
                self.reg_timer_high = timer >> 8;
                self.reg_timer_low = timer & 0xff;
            } else {
                self.sweep_divider -= 1;
            }
        }
    }

    fn timer_value(&self) -> u8 {
        match self.reg_duty_type {
            0..=3 => { DUTY_TABLE[self.reg_duty_type as usize][self.timer_step as usize] }
            _ => panic!("duty_type error {:?}", self.reg_duty_type)
        }
    }

    pub fn value(&self) -> u8 {
        if self.reg_is_enable {
            let v1 = self.timer_value();
            let v2 = self.envelope_value();
    
            v1 * v2
        } else {
            0
        }
    }
}

#[derive(Debug)]
pub struct Mixer {
    pub pulse1 : Pulse,
    pub frames : Vec<f32>,

    time : f32,
    frame_cycle : f32, 
    time_per_cycle : f32,
}

impl Mixer {
    pub fn new() -> Self {
        Self { 
            pulse1: Pulse::new(), 
            frames: vec![],
            time : 0.0,
            frame_cycle : 1.0 / 44_100.0,
            time_per_cycle : 1.0 / 1_789_773.0,
         }
    }

    pub fn step(&mut self, count : usize) {
        for _ in 0..count {
            self.step_cycle()
        }
    }

    pub fn step_cycle(&mut self) {
        self.pulse1.step_cycle();

        self.time += self.time_per_cycle;
        if self.time >= self.frame_cycle {
            self.time -= self.frame_cycle;

            let v = self.value();
            self.frames.push(v);
        }
    }

    // https://www.nesdev.org/wiki/APU_Mixer
    pub fn value(&self) -> f32 {
        let pulse1 = self.pulse1.value();
        let pulse2 = 0; // TODO

        let pulse1_out = PULSE_TABLE[(pulse1 + pulse2) as usize];

        let tnd_out = 0.0; // TODO

        pulse1_out + tnd_out
    }

}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::Mixer;
    use hound;

    static TEST_OUTPUT : &str = "test_result/";

    fn prepare_dir() {
        _ = fs::create_dir(TEST_OUTPUT);
    }

    // テスト用にファイルの書き出し
    impl Mixer {
        fn write_wav_file(&self, output: &str) {
            prepare_dir();

            let wav_spec = hound::WavSpec {
                channels : 1,
                sample_rate : 44100,
                bits_per_sample : 32,
                sample_format : hound::SampleFormat::Float,
            };
    
            let mut writer = hound::WavWriter::create(output, wav_spec).unwrap();
            
            for i in 0..self.frames.len() {
                writer.write_sample(self.frames[i]).unwrap();
            }
            writer.finalize().unwrap();
        }
    }

    #[test]
    #[ignore]
    fn pulse1_440hzの音をファイル出力() {
        // 440hz => 1789773 / 440 / 32 - 1 = 126.11 ≒ 126 = 0x7e
        let mut m = Mixer::new();
        m.pulse1.write_reg1(0x34);
        m.pulse1.write_reg2(0x00);
        m.pulse1.write_reg3(0x7e);
        m.pulse1.write_reg4(0x00);
        m.pulse1.reg_is_enable = true;

        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "1_pusle_440_loop.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn pulse1_880hzの音をファイル出力() {
        // 880hz => 1789773 / 880 / 32 - 1 = 62.56 ≒ 63 = 0x3e
        let mut m = Mixer::new();
        m.pulse1.write_reg1(0x34);
        m.pulse1.write_reg2(0x00);
        m.pulse1.write_reg3(0x3e);
        m.pulse1.write_reg4(0x00);
        m.pulse1.reg_is_enable = true;

        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "2_pusle_880_loop.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn pulse1_duty0_1_3_2の音をファイル出力() {
        let mut m = Mixer::new();
        m.pulse1.write_reg1(0x34);
        m.pulse1.write_reg2(0x00);
        m.pulse1.write_reg3(0x7e);
        m.pulse1.write_reg4(0x00);
        m.pulse1.reg_is_enable = true;

        m.step(40*44100/2);
        m.pulse1.write_reg1(0x74);
        m.step(40*44100/2);
        m.pulse1.write_reg1(0xf4);
        m.step(40*44100/2);
        m.pulse1.write_reg1(0xb4);
        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "3_pusle_duty.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn pulse1_長さ160msecの音をファイル出力() {
        let mut m = Mixer::new();
        m.pulse1.write_reg1(0x14);
        m.pulse1.write_reg2(0x00);
        m.pulse1.write_reg3(0x7e);
        m.pulse1.write_reg4(0x10);
        m.pulse1.reg_is_enable = true;

        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "4_pusle_length.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn pulse1_sweep音をファイル出力() {
        let mut m = Mixer::new();
        m.pulse1.write_reg1(0x34);
        m.pulse1.write_reg2(0b10010100);
        m.pulse1.write_reg3(0x7e);
        m.pulse1.write_reg4(0x30);
        m.pulse1.reg_is_enable = true;

        m.step(40*44100*4/5);

        let file = TEST_OUTPUT.to_string() + "5_pusle_sweep.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn pulse1_sweep音をファイル出力2() {
        let mut m = Mixer::new();
        m.pulse1.write_reg1(0x34);
        m.pulse1.write_reg2(0b11001100);
        m.pulse1.write_reg3(0x7e);
        m.pulse1.write_reg4(0x00);
        m.pulse1.reg_is_enable = true;

        m.step(40*44100*14/10);

        let file = TEST_OUTPUT.to_string() + "6_pusle_sweep2.wav";
        m.write_wav_file(&file); 
    }

}
