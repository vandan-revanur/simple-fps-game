use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

const MOUSE_SCALE: f32 = 1000.0;

/// Lock-free accumulator for mouse deltas.  Scaled to preserve sub-pixel fractions.
pub struct MouseAccum {
    dx: AtomicI32,
    dy: AtomicI32,
    active: std::sync::atomic::AtomicBool,
}

impl MouseAccum {
    pub fn new() -> Self {
        Self {
            dx: AtomicI32::new(0),
            dy: AtomicI32::new(0),
            active: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Accumulate raw PS/2 integer deltas (from `/dev/input/mice`).
    pub fn add_raw(&self, dx: i32, dy: i32) {
        self.dx.fetch_add((dx as f32 * MOUSE_SCALE) as i32, Ordering::Relaxed);
        self.dy.fetch_add((dy as f32 * MOUSE_SCALE) as i32, Ordering::Relaxed);
    }

    /// Accumulate high-precision f64 deltas (from `DeviceEvent::MouseMotion`).
    pub fn add_f64(&self, dx: f64, dy: f64) {
        self.dx.fetch_add((dx * MOUSE_SCALE as f64) as i32, Ordering::Relaxed);
        self.dy.fetch_add((dy * MOUSE_SCALE as f64) as i32, Ordering::Relaxed);
    }

    /// Atomically take (swap to zero) accumulated deltas and return them as `f32`.
    pub fn take(&self) -> (f32, f32) {
        let dx = self.dx.swap(0, Ordering::Relaxed);
        let dy = self.dy.swap(0, Ordering::Relaxed);
        (dx as f32 / MOUSE_SCALE, dy as f32 / MOUSE_SCALE)
    }

    pub fn set_active(&self) {
        self.active.store(true, Ordering::Relaxed);
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

/// Spawn a background thread that reads raw PS/2 packets from `/dev/input/mice`
/// and accumulates them into `accum`.
pub fn spawn_mouse_thread(accum: Arc<MouseAccum>) {
    std::thread::spawn(move || {
        use std::io::Read;
        let _ = std::fs::write("/tmp/simple_quake_mouse.log", "mouse thread starting\n");
        match std::fs::File::open("/dev/input/mice") {
            Ok(mut f) => {
                let _ = std::fs::write("/tmp/simple_quake_mouse.log", "opened /dev/input/mice\n");
                accum.set_active();
                let mut buf = [0u8; 3];
                let mut count = 0u64;
                loop {
                    if f.read_exact(&mut buf).is_err() {
                        break;
                    }
                    // PS/2 3-byte packet: buf[1]=dx, buf[2]=dy (signed)
                    let dx = buf[1] as i8 as i32;
                    let dy = -(buf[2] as i8 as i32); // invert Y
                    accum.add_raw(dx, dy);
                    count += 1;
                    if count % 200 == 0 {
                        let _ = std::fs::write(
                            "/tmp/simple_quake_mouse.log",
                            format!("mouse events: {} last_dx={} last_dy={}\n", count, dx, dy),
                        );
                    }
                }
                let _ = std::fs::write("/tmp/simple_quake_mouse.log", "mouse thread exiting\n");
            }
            Err(e) => {
                let _ = std::fs::write(
                    "/tmp/simple_quake_mouse.log",
                    format!("failed to open /dev/input/mice: {}\n", e),
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_accum_new_defaults() {
        let accum = MouseAccum::new();
        assert!(!accum.is_active(), "new accumulator should not be active");
        let (dx, dy) = accum.take();
        assert_eq!(dx, 0.0, "initial dx should be 0");
        assert_eq!(dy, 0.0, "initial dy should be 0");
    }

    #[test]
    fn test_add_raw_accumulates() {
        let accum = MouseAccum::new();
        accum.add_raw(10, 20);
        let (dx, dy) = accum.take();
        assert!((dx - 10.0).abs() < 0.01, "dx should be ~10.0, got {}", dx);
        assert!((dy - 20.0).abs() < 0.01, "dy should be ~20.0, got {}", dy);
    }

    #[test]
    fn test_take_clears_accumulator() {
        let accum = MouseAccum::new();
        accum.add_f64(5.0, 3.0);
        let _ = accum.take();
        let (dx, dy) = accum.take();
        assert!(dx.abs() < 0.01, "dx should be ~0 after take, got {}", dx);
        assert!(dy.abs() < 0.01, "dy should be ~0 after take, got {}", dy);
    }

    #[test]
    fn test_set_active_flag() {
        let accum = MouseAccum::new();
        assert!(!accum.is_active());
        accum.set_active();
        assert!(accum.is_active());
    }

    #[test]
    fn test_add_f64_fractional() {
        let accum = MouseAccum::new();
        accum.add_f64(1.5, -2.5);
        let (dx, dy) = accum.take();
        assert!((dx - 1.5).abs() < 0.01, "dx should be ~1.5, got {}", dx);
        assert!((dy - (-2.5)).abs() < 0.01, "dy should be ~-2.5, got {}", dy);
    }

    #[test]
    fn test_add_raw_negative() {
        let accum = MouseAccum::new();
        accum.add_raw(-5, -10);
        let (dx, dy) = accum.take();
        assert!((dx - (-5.0)).abs() < 0.01, "dx should be ~-5.0, got {}", dx);
        assert!((dy - (-10.0)).abs() < 0.01, "dy should be ~-10.0, got {}", dy);
    }

    #[test]
    fn test_accumulation_is_additive() {
        let accum = MouseAccum::new();
        accum.add_raw(3, 4);
        accum.add_raw(7, 6);
        let (dx, dy) = accum.take();
        assert!((dx - 10.0).abs() < 0.01, "dx should sum to ~10.0, got {}", dx);
        assert!((dy - 10.0).abs() < 0.01, "dy should sum to ~10.0, got {}", dy);
    }
}
