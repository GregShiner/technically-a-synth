//! Grossly, incomprehensibly generic oscillators
use core::f32::consts::PI;

use dasp::{Frame, Signal};
use libm::sinf;

pub struct ConstHz {
    freq: f32,
}

impl ConstHz {
    pub fn new(freq: f32) -> Self {
        Self { freq }
    }
}

impl Signal for ConstHz {
    type Frame = f32;

    fn next(&mut self) -> Self::Frame {
        self.freq
    }
}

pub struct LinearPhase<F: Frame, Hz: Signal<Frame = F>> {
    phase: F,
    sample_rate: f32,
    freq: Hz,
}

impl<Hz: Signal<Frame = f32>> LinearPhase<f32, Hz> {
    pub fn from_freq(freq: Hz, sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            sample_rate,
            freq,
        }
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

// TODO: Make this more generic maybe?
impl<Hz: Signal<Frame = f32>> Signal for LinearPhase<f32, Hz> {
    type Frame = f32;

    fn next(&mut self) -> Self::Frame {
        let phase = self.phase;
        let freq = self.freq.next();
        self.phase = (self.phase + (freq / self.sample_rate)) % 1.0f32;
        phase
    }
}

pub struct Square<F: Frame, P: Signal<Frame = F>> {
    phase: P,
}

impl<F: Frame, P: Signal<Frame = F>> Square<F, P> {
    pub fn from_phase(phase: P) -> Self {
        Self { phase }
    }
}

impl<P: Signal<Frame = f32>> Signal for Square<f32, P> {
    type Frame = f32;

    fn next(&mut self) -> f32 {
        let phase = self.phase.next();
        if phase >= 0.5 { 1.0 } else { 0.0 }
    }
}

pub struct Saw<F: Frame, P: Signal<Frame = F>> {
    phase: P,
}

impl<F: Frame, P: Signal<Frame = F>> Saw<F, P> {
    pub fn from_phase(phase: P) -> Self {
        Self { phase }
    }
}

impl<P: Signal<Frame = f32>> Signal for Saw<f32, P> {
    type Frame = f32;

    fn next(&mut self) -> f32 {
        let phase = self.phase.next();
        (2.0 * phase) - 1.0
    }
}

// phase += (1/sample_rate) % 1
// sin(freq*2*pi*phase)
pub struct Sine<F: Frame, P: Signal<Frame = F>> {
    phase: P,
}

impl<F: Frame, P: Signal<Frame = F>> Sine<F, P> {
    pub fn from_phase(phase: P) -> Self {
        Self { phase }
    }
}

impl<P: Signal<Frame = f32>> Signal for Sine<f32, P> {
    type Frame = f32;

    fn next(&mut self) -> f32 {
        let phase = self.phase.next();
        // TODO: Figure out which one's faster
        //(2.0 * PI * phase).sin()
        sinf(2.0 * PI * phase)
    }
}

pub struct Triangle<F: Frame, P: Signal<Frame = F>> {
    phase: P,
}

impl<F: Frame, P: Signal<Frame = F>> Triangle<F, P> {
    pub fn from_phase(phase: P) -> Self {
        Self { phase }
    }
}

impl<P: Signal<Frame = f32>> Signal for Triangle<f32, P> {
    type Frame = f32;

    fn next(&mut self) -> f32 {
        let phase = self.phase.next();
        (4.0 * (phase - 0.5).abs()) - 1.0
    }
}
