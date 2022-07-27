
use core::fmt;

use pa::{Stream, NonBlocking, Output};
use portaudio as pa;

const CHANNELS: i32 = 1;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = 64;

pub struct Apu {
    stream: Option<Stream<NonBlocking, Output<f32>>>
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
            stream: None
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
    
        // This routine will be called by the PortAudio engine when audio is needed. It may called at
        // interrupt level on some machines so don't do anything that could mess up the system like
        // dynamic resource allocation or IO.
        let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
            let mut idx = 0;
            for _ in 0..frames {
                buffer[idx] = left_saw;
                left_saw += 0.01;
                if left_saw >= 1.0 {
                    left_saw -= 2.0;
                }
                idx += 1;
            }
            pa::Continue
        };
    
        let mut stream = pa.open_non_blocking_stream(settings, callback)?;
    
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

}