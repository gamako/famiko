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

static TND_TABLE : Lazy<[f32;256]> = Lazy::new(||{
    let mut t : [f32;256] = [0.0;256];
    for i in 0..256 {
        t[i] = 163.67 / (24329.0 / (i as f32) + 100.0)
    }
    t
});

#[derive(Debug)]
pub struct Pulse {
    num : u8,
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
    pub fn new(num :u8) -> Self {
        Self { 
            num : num,
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
            reg_is_enable : false,
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

static TRIANGLE_ENVELOPE_TABLE : [u8; 32] = [
    0xF, 0xE, 0xD, 0xC, 0xB, 0xA, 0x9, 0x8, 0x7, 0x6, 0x5, 0x4, 0x3, 0x2, 0x1, 0x0,
    0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF,
];

#[derive(Debug)]
pub struct Triangle {
    pub disable_length : bool,
    pub linear_counter : u8,
    pub timer : u16,
    pub length_counter : u8,
    pub envelope : u8,
    pub is_enable : bool,
    pub is_reset : bool,

    pub timer_divider : u16,
    pub value : u16,
}

impl Triangle {
    pub fn new() -> Self {
        Self { 
            disable_length: true,
            linear_counter: 0,
            timer: 0,
            length_counter: 0,
            envelope: 0,
            is_enable: false,
            is_reset: false,
            timer_divider: 0,
            value: 0,
        }
    }

    pub fn write_reg1(&mut self, v : u8) {
        // CRRR RRRR	Length counter halt / linear counter control (C), linear counter load (R)
        self.disable_length = v & (1 << 7) != 0;
        self.linear_counter = v & 0x7f;
    }

    pub fn write_reg2(&mut self, v : u8) {
        // TTTT TTTT	Timer low (T)
        self.timer = (self.timer & 0x700) | (v as u16);
        self.timer_divider = self.timer;
    }

    pub fn write_reg3(&mut self, v : u8) {
        // LLLL LTTT	Length counter load (L), timer high (T)
        self.timer = (self.timer & 0xff) | ((v as u16 & 0x07) << 8);
        self.timer_divider = self.timer;
        let length_type = v >> 3;
        self.length_counter = LENGTH_TABLE[length_type as usize];
        self.is_reset = true;
    }

    fn step_cycle(&mut self, is_step_length : bool) {
        if self.timer_divider != 0 {
            self.timer_divider -= 1;
        } else {
            self.timer_divider = self.timer;
            self.envelope = (self.envelope + 1) % 32;
        }

        if is_step_length {
            if self.linear_counter != 0 {
                self.linear_counter -= 1;
            }

            if !self.disable_length {
                if self.length_counter != 0 {
                    self.length_counter -= 1;
                }
            }
        }
    }

    fn value(&self) -> u8 {
        if self.is_enable() {
            TRIANGLE_ENVELOPE_TABLE[self.envelope as usize]
        } else {
            0
        }
    }

    fn is_enable(&self) -> bool {
        self.is_enable && self.length_counter != 0 && self.linear_counter != 0
    }
}

static NOISE_PERIOD_TABLE : [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068
];

#[derive(Debug)]
pub struct Noise {
    
    pub is_length_disable : bool,
    pub is_constant_volume : bool,
    pub volume : u8,
    pub mode : bool,
    pub period_type : u8,
    pub length : u8,
    pub is_reset : bool,
    pub is_enable : bool,

    pub timer_divider : u16,

    pub length_counter : u8,

    pub shift_register : u16,
}

impl Noise {
    pub fn new() -> Self {
        Self {
            is_length_disable: false,
            is_constant_volume: true,
            volume: 0,
            mode: false,
            period_type: 0,
            length: 0,
            is_reset : false,
            is_enable : true,

            timer_divider : 0,

            length_counter : 0,

            shift_register : 1,
        }
    }

    pub fn write_reg1(&mut self, v : u8) {
        // --lc.vvvv	Length counter halt, constant volume/envelope flag, and volume/envelope divider period (write)
        self.is_length_disable = v & (1 << 5) != 0;
        self.is_constant_volume = v & (1 << 4) != 0;
        self.volume = v & 0x0f;
    }

    pub fn write_reg2(&mut self, v : u8) {
        // M---.PPPP	Mode and period (write)
        self.mode = v & (1 << 7) != 0;
        self.period_type = v & 0x0f;
    }

    pub fn write_reg3(&mut self, v : u8) {
        // llll.l---	Length counter load and envelope restart (write)
        let length_type = v >> 3;
        self.length_counter = LENGTH_TABLE[length_type as usize];
        self.is_reset = true;
    }

    pub fn step_cycle(&mut self, is_step_length : bool) {
        if self.timer_divider != 0 {
            self.timer_divider -= 1;
        } else {
            self.timer_divider = NOISE_PERIOD_TABLE[self.period_type as usize];

            let bit = if self.mode == false { 1 } else { 6 };
            let b = (self.shift_register & 1) ^ ((self.shift_register >> bit) & 1);
            
            self.shift_register = (self.shift_register >> 1) & 0x3fff | (b << 14);

            if !self.is_constant_volume {
                if self.volume != 0 {
                    self.volume -= 0;
                }
            }
        }

        if is_step_length {
            if !self.is_length_disable {
                if self.length_counter != 0 {
                    self.length_counter -= 1;
                } else {
                    self.is_enable = false;
                }
            }
        }
    }

    pub fn value(&self) -> u8 {
        if self.is_enable {
            ((self.shift_register & 1) as u8) * self.volume
        } else {
            0
        }
    }

}

#[derive(Debug)]
struct FrameSequencer {
    is_5step_mode : bool,
    diveder : u16,
    step : u8,
}

impl FrameSequencer {
    pub fn new() -> Self {
        Self {
            is_5step_mode : false,
            diveder : 0,
            step : 0,
        }
    }

    // (IRQ, LENGTH, ENVELOPE)のフラグを返す
    fn step(&mut self) -> (bool, bool, bool) {
        // フレームシーケンサー
        if self.diveder != 0 {
            self.diveder -= 1;
            (false, false, false)
        } else {
            self.diveder = 7467;

            let (step_max, r) = if self.is_5step_mode {
                (5, FLAME_SEQ_5[self.step as usize])
            } else {
                (4, FLAME_SEQ_4[self.step as usize])
            };
            self.step = (self.step + 1) % step_max;
            r
        }
    }
}

fn mixer(pulse1 : u8, pulse2 : u8, triangle : u8, noise : u8, dmc: u8) -> f32 {
    let pulse_out = PULSE_TABLE[(pulse1 + pulse2) as usize];

    let tnd_out = TND_TABLE[(3 * triangle + noise + dmc) as usize];

    pulse_out + tnd_out
}

#[derive(Debug)]
pub struct Apu {
    frame_sequencer : FrameSequencer,
    pub pulse1 : Pulse,
    pub pulse2 : Pulse,
    pub triangle : Triangle,
    pub noise : Noise,
    pub frames : Vec<f32>,

    time : f32,
    frame_cycle : f32, 
    time_per_cycle : f32,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            frame_sequencer : FrameSequencer::new(),
            pulse1: Pulse::new(1),
            pulse2: Pulse::new(2),
            triangle : Triangle::new(),
            noise : Noise::new(),
            frames: vec![],
            time : 0.0,
            frame_cycle : 1.0 / 44_100.0,
            time_per_cycle : 1.0 / 1_789_773.0,
         }
    }

    pub fn read(&mut self, addr : u16, is_debug : bool) -> u8 {
        match addr {
            0x4015 => {
                ( (self.pulse1.reg_is_enable as u8) << 0) | 
                ( (self.pulse2.reg_is_enable as u8) << 1) | 
                ( (self.triangle.is_enable() as u8) << 2) | 
                ( (self.noise.is_enable as u8) << 3)
            }
            _ => 0u8,
        }
    }

    pub fn write(&mut self, addr : u16, v : u8) {
        match addr {
            0x4000 => self.pulse1.write_reg1(v),
            0x4001 => self.pulse1.write_reg2(v),
            0x4002 => self.pulse1.write_reg3(v),
            0x4003 => self.pulse1.write_reg4(v),
            0x4004 => self.pulse2.write_reg1(v),
            0x4005 => self.pulse2.write_reg2(v),
            0x4006 => self.pulse2.write_reg3(v),
            0x4007 => self.pulse2.write_reg4(v),
 
            0x4008 => self.triangle.write_reg1(v),
            0x4009 => {},
            0x400a => self.triangle.write_reg2(v),
            0x400b => self.triangle.write_reg3(v),
            0x400c => self.noise.write_reg1(v),
            0x400d => {},
            0x400e => self.noise.write_reg2(v),
            0x400f => self.noise.write_reg3(v),
            0x4010 => {},
            0x4011 => {},
            0x4012 => {},
            0x4013 => {},
            0x4015 => { 
                self.pulse1.reg_is_enable = v & (1 << 0) != 0;
                self.pulse2.reg_is_enable = v & (1 << 1) != 0;
                self.triangle.is_enable = v & (1 << 2) != 0;
                self.noise.is_enable = v & (1 << 3) != 0;
            },
            _ => {
                panic!("unimplemented apu {:?}", addr);
            }
        };
    }

    pub fn step(&mut self, count : usize) -> bool {
        let mut is_irq = false;
        for _ in 0..count {
            is_irq |= self.step_cycle()
        }
        is_irq
    }

    pub fn step_cycle(&mut self) -> bool {
        let (is_irq, is_length, _) = self.frame_sequencer.step();
        self.pulse1.step_cycle();
        self.pulse2.step_cycle();
        self.triangle.step_cycle(is_length);
        self.noise.step_cycle(is_length);

        self.time += self.time_per_cycle;
        if self.time >= self.frame_cycle {
            self.time -= self.frame_cycle;

            let v = self.value();
            self.frames.push(v);
        }
        is_irq
    }

    // https://www.nesdev.org/wiki/APU_Mixer
    pub fn value(&self) -> f32 {
        let pulse1 = self.pulse1.value();
        let pulse2 = self.pulse2.value();
        let triangle = self.triangle.value();
        let noise = self.noise.value();
        let dmc = 0;

        mixer(pulse1, pulse2, triangle, noise, dmc)
    }

}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::Apu;
    use hound;

    static TEST_OUTPUT : &str = "test_result/";

    fn prepare_dir() {
        _ = fs::create_dir(TEST_OUTPUT);
    }

    // テスト用にファイルの書き出し
    impl Apu {
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
        let mut m = Apu::new();
        m.pulse1.write_reg1(0x34);
        m.pulse1.write_reg2(0x00);
        m.pulse1.write_reg3(0x7e);
        m.pulse1.write_reg4(0x00);
        m.pulse1.reg_is_enable = true;

        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "pusle_1_440_loop.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn pulse1_880hzの音をファイル出力() {
        // 880hz => 1789773 / 880 / 32 - 1 = 62.56 ≒ 63 = 0x3e
        let mut m = Apu::new();
        m.pulse1.write_reg1(0x34);
        m.pulse1.write_reg2(0x00);
        m.pulse1.write_reg3(0x3e);
        m.pulse1.write_reg4(0x00);
        m.pulse1.reg_is_enable = true;

        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "pusle_2_880_loop.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn pulse1_duty0_1_3_2の音をファイル出力() {
        let mut m = Apu::new();
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

        let file = TEST_OUTPUT.to_string() + "pusle_3_duty.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn pulse1_長さ160msecの音をファイル出力() {
        let mut m = Apu::new();
        m.pulse1.write_reg1(0x14);
        m.pulse1.write_reg2(0x00);
        m.pulse1.write_reg3(0x7e);
        m.pulse1.write_reg4(0x10);
        m.pulse1.reg_is_enable = true;

        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "pusle_4_length.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn pulse1_sweep音をファイル出力() {
        let mut m = Apu::new();
        m.pulse1.write_reg1(0x34);
        m.pulse1.write_reg2(0b10010100);
        m.pulse1.write_reg3(0x7e);
        m.pulse1.write_reg4(0x30);
        m.pulse1.reg_is_enable = true;

        m.step(40*44100*4/5);

        let file = TEST_OUTPUT.to_string() + "pusle_5_sweep.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn pulse1_sweep音をファイル出力2() {
        let mut m = Apu::new();
        m.pulse1.write_reg1(0x34);
        m.pulse1.write_reg2(0b11001100);
        m.pulse1.write_reg3(0x7e);
        m.pulse1.write_reg4(0x00);
        m.pulse1.reg_is_enable = true;

        m.step(40*44100*14/10);

        let file = TEST_OUTPUT.to_string() + "pusle_6_sweep2.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn triangle_440hz音をファイル出力() {
        let mut m = Apu::new();
        m.triangle.write_reg1(0b11000000);
        m.triangle.write_reg2(0b10000001);
        m.triangle.write_reg3(0b01111000);

        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "triangle_1_440hz.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn triangle_880hz音をファイル出力() {
        let mut m = Apu::new();
        m.triangle.write_reg1(0b11000000);
        m.triangle.write_reg2(0b00001110);
        m.triangle.write_reg3(0b01111000);

        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "triangle_2_880hz.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn triangle_160secの音をファイル出力() {
        let mut m = Apu::new();
        m.triangle.write_reg1(0b01111111);
        m.triangle.write_reg2(0b10000001);
        m.triangle.write_reg3(0b01111000);

        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "triangle_3_length.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn noise_25hzファイル出力() {
        let mut m = Apu::new();
        m.noise.write_reg1(0b00111100);
        m.noise.write_reg2(0b00001100);
        m.noise.write_reg3(0b00000000);

        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "noise_1_25hz.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn noise_200hzファイル出力() {
        let mut m = Apu::new();
        m.noise.write_reg1(0b00111100);
        m.noise.write_reg2(0b00000101);
        m.noise.write_reg3(0b00000000);

        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "noise_2_200hz.wav";
        m.write_wav_file(&file); 
    }

    #[test]
    #[ignore]
    fn noise_mode1ファイル出力() {
        let mut m = Apu::new();
        m.noise.write_reg1(0b00111100);
        m.noise.write_reg2(0b10001100);
        m.noise.write_reg3(0b00000000);

        m.step(40*44100/2);

        let file = TEST_OUTPUT.to_string() + "noise_3_mode1.wav";
        m.write_wav_file(&file); 
    }

}
