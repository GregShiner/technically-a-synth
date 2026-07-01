//! Grossly, incomprehensibly generic oscillators
use core::f32::consts::PI;

use dasp::Frame;
use libm::sinf;

use crate::{Module, generate_process_enum};

pub struct ConstHz {
    freq: f32,
}

impl ConstHz {
    pub fn new(freq: f32) -> Self {
        Self { freq }
    }
}

impl Module<(), f32> for ConstHz {
    fn process(&mut self, _input: ()) -> f32 {
        self.freq
    }
}

pub struct LinearPhase<F: Frame> {
    phase: F,
    sample_rate: f32,
}

impl LinearPhase<f32> {
    pub fn from_sample_rate(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            sample_rate,
        }
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

// TODO: Make this more generic maybe?
impl Module<f32, f32> for LinearPhase<f32> {
    fn process(&mut self, freq: f32) -> f32 {
        let phase = self.phase;
        self.phase = (self.phase + (freq / self.sample_rate)) % 1.0f32;
        phase
    }
}

pub struct Square;
impl Module<f32, f32> for Square {
    fn process(&mut self, phase: f32) -> f32 {
        if phase >= 0.5 { 1.0 } else { 0.0 }
    }
}

pub struct Saw;
impl Module<f32, f32> for Saw {
    fn process(&mut self, phase: f32) -> f32 {
        // Phase goes from 0->1, so double and shift down by 1
        (2.0 * phase) - 1.0
    }
}

// phase += (1/sample_rate) % 1
// sin(freq*2*pi*phase)
pub struct Sine;
impl Module<f32, f32> for Sine {
    fn process(&mut self, phase: f32) -> f32 {
        // TODO: Figure out which one's faster
        //(2.0 * PI * phase).sin()
        sinf(2.0 * PI * phase)
    }
}

pub struct Triangle;
impl Module<f32, f32> for Triangle {
    fn process(&mut self, phase: f32) -> f32 {
        // TODO: I think this math is right, but find out why
        (4.0 * (phase - 0.5).abs()) - 1.0
    }
}

generate_process_enum!(Oscillator, f32, f32, (Square, Sine, Saw, Triangle));
