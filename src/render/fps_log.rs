use std::time::Instant;

const TIME_AVG: f64 = 0.5;

pub struct FpsLog {
    frame_times: Vec<f64>,
    prev_frame: Instant,
    last_log: Instant,
}

impl FpsLog {
    pub fn new() -> Self {
        Self {
            frame_times: Vec::with_capacity(30),
            prev_frame: Instant::now(),
            last_log: Instant::now(),
        }
    }

    pub fn update(&mut self) -> f32 {
        let frame_time = Instant::now() - self.prev_frame;
        self.frame_times.push(frame_time.as_secs_f64());
        self.prev_frame = Instant::now();

        if (Instant::now() - self.last_log).as_secs_f64() >= TIME_AVG {
            let mut sum = 0.0;
            self.frame_times.iter().for_each(|t| { sum += t });
            println!("Avg. FPS: {:.2}", self.frame_times.len() as f64 / sum);
            self.last_log = Instant::now();
            self.frame_times.clear();
        }

        frame_time.as_secs_f32()
    }
}