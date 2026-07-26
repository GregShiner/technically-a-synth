//! Grossly, incomprehensibly generic oscillators
use core::f32::consts::PI;

use dasp::Frame;
use libm::sinf;

use crate::{Module, generate_process_enum};

#[derive(Clone)]
pub struct ConstHz {
    freq: f32,
}

impl ConstHz {
    pub fn new(freq: f32) -> Self {
        Self { freq }
    }
}

impl Module for ConstHz {
    const INPUTS: usize = 0;
    const OUTPUTS: usize = 1;
    const SCRATCH_BUFFERS: usize = 0;

    fn process(
        &mut self,
        _input_buffers: &[&[f32]],
        output_buffers: &mut [&mut [f32]],
        _scratch_buffers: &mut [&mut [f32]],
    ) -> () {
        output_buffers[0].fill(self.freq);
    }
}

#[derive(Clone)]
pub struct LinearPhase {
    phase: f32,
    sample_rate: f32,
}

impl LinearPhase {
    const INPUTS: usize = 1;
    const OUTPUTS: usize = 1;
    const SCRATCH_BUFFERS: usize = 0;

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

impl Module for LinearPhase {
    const INPUTS: usize = 1;
    const OUTPUTS: usize = 1;
    const SCRATCH_BUFFERS: usize = 0;

    fn process(
        &mut self,
        input_buffers: &[&[f32]],
        output_buffers: &mut [&mut [f32]],
        _scratch_buffers: &mut [&mut [f32]],
    ) -> () {
        input_buffers[0]
            .iter()
            .zip(output_buffers[0].iter_mut())
            .map(|(freq, phase)| {
                let old_phase = self.phase;
                self.phase = (self.phase + (freq / self.sample_rate)) % 1.0f32;
                *phase = old_phase;
            });
    }
}

#[derive(Clone)]
pub struct Square;
impl Module for Square {
    const INPUTS: usize = 1;
    const OUTPUTS: usize = 1;
    const SCRATCH_BUFFERS: usize = 0;

    fn process(
        &mut self,
        input_buffers: &[&[f32]],
        output_buffers: &mut [&mut [f32]],
        _scratch_buffers: &mut [&mut [f32]],
    ) -> () {
        input_buffers[0]
            .iter()
            .zip(output_buffers[0].iter_mut())
            .map(|(phase, output)| *output = if *phase >= 0.5 { 1.0 } else { 0.0 });
    }
}

#[derive(Clone)]
pub struct Saw;
impl Module for Saw {
    const INPUTS: usize = 1;
    const OUTPUTS: usize = 1;
    const SCRATCH_BUFFERS: usize = 0;

    fn process(
        &mut self,
        input_buffers: &[&[f32]],
        output_buffers: &mut [&mut [f32]],
        _scratch_buffers: &mut [&mut [f32]],
    ) -> () {
        // Phase goes from 0->1, so double and shift down by 1
        input_buffers[0]
            .iter()
            .zip(output_buffers[0].iter_mut())
            .map(|(phase, output)| *output = (2.0 * phase) - 1.0);
    }
}

// phase += (1/sample_rate) % 1
// sin(freq*2*pi*phase)
#[derive(Clone)]
pub struct Sine;
impl Module for Sine {
    const INPUTS: usize = 1;
    const OUTPUTS: usize = 1;
    const SCRATCH_BUFFERS: usize = 0;

    fn process(
        &mut self,
        input_buffers: &[&[f32]],
        output_buffers: &mut [&mut [f32]],
        _scratch_buffers: &mut [&mut [f32]],
    ) -> () {
        // TODO: Figure out which one's faster
        //(2.0 * PI * phase).sin()
        input_buffers[0]
            .iter()
            .zip(output_buffers[0].iter_mut())
            .map(|(phase, output)| *output = sinf(2.0 * PI * phase));
    }
}

#[derive(Clone)]
pub struct Triangle;
impl Module for Triangle {
    const INPUTS: usize = 1;
    const OUTPUTS: usize = 1;
    const SCRATCH_BUFFERS: usize = 0;

    fn process(
        &mut self,
        input_buffers: &[&[f32]],
        output_buffers: &mut [&mut [f32]],
        _scratch_buffers: &mut [&mut [f32]],
    ) -> () {
        // TODO: I think this math is right, but find out why
        input_buffers[0]
            .iter()
            .zip(output_buffers[0].iter_mut())
            .map(|(phase, output)| *output = (4.0 * (phase - 0.5).abs()) - 1.0);
    }
}

generate_process_enum!(Oscillator, (Square, Sine, Saw, Triangle));
