
use std::time::Instant;
pub struct FpsTimer {
    fps_period_start_time: Instant,
    fps: u32,
    elapsed_frames: u32,
}

impl FpsTimer {
    pub fn new() -> Self {
        FpsTimer {
            fps_period_start_time: Instant::now(),
            fps: 0,
            elapsed_frames: 0,
        }
    }

    pub fn calculate(&mut self) -> u32 {
        self.elapsed_frames += 1;
        let elapsed = self.fps_period_start_time.elapsed().as_millis() as u32;
        if elapsed > 1000 {
            self.fps = self.elapsed_frames;
            self.fps_period_start_time = Instant::now();
            self.elapsed_frames = 0;
        }

        self.fps
    }
}