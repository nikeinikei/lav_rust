use std::time::Instant;

pub struct Timer {
    start_instant: Instant,
    last_frame: Instant,
    dt: f64,
}

const AMOUNT_NANOS: f64 = 1_000_000_000_f64;

impl Timer {
    pub fn new() -> Timer {
        let now = Instant::now();

        Timer {
            start_instant: now.clone(),
            last_frame: now,
            dt: 0_f64,
        }
    }

    pub fn step(&mut self) {
        let now = Instant::now();

        let ns = self.last_frame.elapsed().as_nanos() as f64;
        let dt = ns / AMOUNT_NANOS;
        self.dt = dt;

        self.last_frame = now;
    }

    pub fn get_time(&self) -> f64 {
        let ns = self.start_instant.elapsed().as_nanos() as f64;
        ns / AMOUNT_NANOS
    }

    pub fn get_fps(&self) -> f64 {
        if self.dt == 0_f64 {
            0_f64
        } else {
            1_f64 / self.dt
        }
    }

    pub fn get_delta(&self) -> f64 {
        self.dt
    }

    pub fn sleep(&self, duration: f64) {
        std::thread::sleep(std::time::Duration::from_secs_f64(duration));
    }
}
