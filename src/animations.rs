use std::collections::HashMap;
use std::time::{Duration, Instant};
use smithay::desktop::Window;

pub struct Animation {
    pub start_time: Instant,
    pub duration: Duration,
    pub start_val: f64,
    pub end_val: f64,
}

impl Animation {
    pub fn new(start: f64, end: f64, duration_ms: u64) -> Self {
        Self {
            start_time: Instant::now(),
            duration: Duration::from_millis(duration_ms),
            start_val: start,
            end_val: end,
        }
    }

    pub fn value(&self, now: Instant) -> f64 {
        let elapsed = now.duration_since(self.start_time);
        if elapsed >= self.duration {
            return self.end_val;
        }
        let progress = elapsed.as_secs_f64() / self.duration.as_secs_f64();
        // Linear interpolation for now
        self.start_val + (self.end_val - self.start_val) * progress
    }
    
    pub fn is_done(&self, now: Instant) -> bool {
        now >= self.start_time + self.duration
    }
}

pub struct WindowAnimationState {
    pub alpha: f64,
    pub scale: f64,
    pub animations: HashMap<String, Animation>, // e.g. "fade", "scale"
}

impl Default for WindowAnimationState {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            scale: 1.0,
            animations: HashMap::new(),
        }
    }
}

pub struct AnimationManager {
    pub states: HashMap<Window, WindowAnimationState>,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self { states: HashMap::new() }
    }
    
    pub fn start_fade_in(&mut self, window: &Window) {
        let state = self.states.entry(window.clone()).or_default();
        state.animations.insert("fade".to_string(), Animation::new(0.0, 1.0, 250));
    }
    
    pub fn start_fade_out(&mut self, window: &Window) {
        let state = self.states.entry(window.clone()).or_default();
        state.animations.insert("fade".to_string(), Animation::new(1.0, 0.0, 250));
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        
        for state in self.states.values_mut() {
            // Processing "fade"
            if let Some(anim) = state.animations.get("fade") {
                state.alpha = anim.value(now);
            }
            // Cleanup done animations
            state.animations.retain(|_, anim| !anim.is_done(now));
        }
    }
    
    pub fn get_alpha(&self, window: &Window) -> f32 {
        self.states.get(window).map(|s| s.alpha as f32).unwrap_or(1.0)
    }
}
