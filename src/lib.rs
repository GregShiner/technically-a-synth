#![no_std]

use dasp::{
    Signal,
    signal::{
        self, ConstHz, Saw, Sine, Square,
        bus::{Bus, Output, SignalBus},
    },
};
use microfft::{Complex32, real::rfft_1024};

pub struct Oscillator<S: Signal<Frame = f64>> {
    freq: Option<f64>,
    sample_rate: Option<f64>,
    pub bus: Bus<S>,
    fft_send: Output<S>,
    fft_buffer: [f32; 1024],
}

impl<S: Signal<Frame = f64>> Oscillator<S> {
    pub fn new(freq: Option<f64>, sample_rate: Option<f64>, signal: S) -> Self {
        let bus = signal.bus();
        let fft_send = bus.send();
        Self {
            freq,
            sample_rate,
            bus,
            fft_send,
            fft_buffer: [0.0; 1024],
        }
    }

    pub fn fft_1024(&mut self) -> &mut [Complex32; 512] {
        self.fft_buffer = core::array::from_fn(|_| self.fft_send.next() as f32);
        rfft_1024(&mut self.fft_buffer)
    }

    pub fn real_fft_1024(&mut self) -> [f32; 512] {
        let complex_result = self.fft_1024();
        complex_result.map(|c| (c.re * c.re + c.im * c.im).sqrt())
    }
}

impl Oscillator<Square<ConstHz>> {
    pub fn new_square(freq: f64, sample_rate: f64) -> Self {
        let bus = signal::rate(sample_rate).const_hz(freq).square().bus();
        let fft_send = bus.send();
        Self {
            freq: Some(freq),
            sample_rate: Some(sample_rate),
            bus,
            fft_send,
            fft_buffer: [0.0; 1024],
        }
    }
}

impl Oscillator<Sine<ConstHz>> {
    pub fn new_sine(freq: f64, sample_rate: f64) -> Self {
        let bus = signal::rate(sample_rate).const_hz(freq).sine().bus();
        let fft_send = bus.send();
        Self {
            freq: Some(freq),
            sample_rate: Some(sample_rate),
            bus,
            fft_send,
            fft_buffer: [0.0; 1024],
        }
    }
}

impl Oscillator<Saw<ConstHz>> {
    pub fn new_saw(freq: f64, sample_rate: f64) -> Self {
        let bus = signal::rate(sample_rate).const_hz(freq).saw().bus();
        let fft_send = bus.send();
        Self {
            freq: Some(freq),
            sample_rate: Some(sample_rate),
            bus,
            fft_send,
            fft_buffer: [0.0; 1024],
        }
    }
}

pub fn square_oscillator(sample_rate: f64, freq: f64) -> Square<ConstHz> {
    signal::rate(sample_rate).const_hz(freq).square()
}

pub fn sine_oscillator(sample_rate: f64, freq: f64) -> Sine<ConstHz> {
    signal::rate(sample_rate).const_hz(freq).sine()
}

pub fn saw_oscillator(sample_rate: f64, freq: f64) -> Saw<ConstHz> {
    signal::rate(sample_rate).const_hz(freq).saw()
}

// TODO: Custom triangle wave oscilator
