#![no_std]

pub mod biquad;
use dasp::{
    Signal,
    signal::{
        self, ConstHz, Saw, Sine, Square,
        bus::{Bus, Output, SignalBus},
    },
};
use libm::sqrtf;
use microfft::{Complex32, real::rfft_1024};

const FFT_BUFFER_SIZE: usize = 1024;

pub struct Oscillator<S: Signal<Frame = f64>> {
    freq: Option<f64>,
    sample_rate: Option<f64>,
    pub bus: Bus<S>,
    pub main_send: Output<S>,
}

impl<S: Signal<Frame = f64>> Signal for Oscillator<S> {
    type Frame = f64;

    fn next(&mut self) -> Self::Frame {
        self.main_send.next()
    }
}

impl<S: Signal<Frame = f64>> Oscillator<S> {
    pub fn new(freq: Option<f64>, sample_rate: Option<f64>, signal: S) -> Self {
        let bus = signal.bus();
        let main_send = bus.send();
        Self {
            freq,
            sample_rate,
            bus,
            main_send,
        }
    }
}

pub struct FFTAnalyzer<S: Signal<Frame = f64>> {
    inner: S,
    fft_buffer: [f32; FFT_BUFFER_SIZE],
    fft_cursor: usize,
}

impl<S: Signal<Frame = f64>> FFTAnalyzer<S> {
    pub fn new(signal: S) -> Self {
        let fft_buffer = [0.0f32; FFT_BUFFER_SIZE];
        let fft_cursor = 0usize;
        Self {
            inner: signal,
            fft_buffer,
            fft_cursor,
        }
    }
    pub fn fft_1024_magnitudes(&mut self) -> [f32; FFT_BUFFER_SIZE / 2] {
        let spectrum = self.fft_1024();
        complex_magnitudes(spectrum)
    }

    pub fn fft_1024(&mut self) -> [Complex32; FFT_BUFFER_SIZE / 2] {
        // It might make sense to make this function return an option that is only Some when the
        // fft_cursor is 0. This may also avoid the need for a copy of the buffer, but only if the
        // buffer is ever consumed once every time since it may be modified.
        // Reorder ring buffer so oldest sample is first
        let mut ordered = [0.0f32; FFT_BUFFER_SIZE];
        let (a, b) = self.fft_buffer.split_at(self.fft_cursor);
        ordered[..b.len()].copy_from_slice(b);
        ordered[b.len()..].copy_from_slice(a);
        // dasp comes with a hann window function that gets applied to a signal but for whatever
        // reason that did not work and broke the FFT. Doing it manually seems to be fine.
        // Maybe switch this out in the future for something a little faster.
        (0..FFT_BUFFER_SIZE).for_each(|i| {
            let hann = 0.5
                * (1.0 - (2.0 * core::f32::consts::PI * i as f32 / FFT_BUFFER_SIZE as f32).cos());
            ordered[i] *= hann;
        });
        *rfft_1024(&mut ordered)
    }
}

impl<S: Signal<Frame = f64>> Signal for FFTAnalyzer<S> {
    type Frame = f64;
    fn next(&mut self) -> f64 {
        let fft_sample = self.inner.next();

        self.fft_buffer[self.fft_cursor] = fft_sample as f32;
        self.fft_cursor = (self.fft_cursor + 1) % FFT_BUFFER_SIZE;
        fft_sample
    }
}
pub fn complex_magnitudes<const N: usize>(complex: [Complex32; N]) -> [f32; N] {
    complex.map(|c| sqrtf(c.re * c.re + c.im * c.im))
}

impl Oscillator<Square<ConstHz>> {
    pub fn new_square(freq: f64, sample_rate: f64) -> Self {
        let bus = signal::rate(sample_rate).const_hz(freq).square().bus();
        let main_send = bus.send();
        Self {
            freq: Some(freq),
            sample_rate: Some(sample_rate),
            bus,
            main_send,
        }
    }
}

impl Oscillator<Sine<ConstHz>> {
    pub fn new_sine(freq: f64, sample_rate: f64) -> Self {
        let bus = signal::rate(sample_rate).const_hz(freq).sine().bus();
        let main_send = bus.send();
        Self {
            freq: Some(freq),
            sample_rate: Some(sample_rate),
            bus,
            main_send,
        }
    }
}

impl Oscillator<Saw<ConstHz>> {
    pub fn new_saw(freq: f64, sample_rate: f64) -> Self {
        let bus = signal::rate(sample_rate).const_hz(freq).saw().bus();
        let main_send = bus.send();
        Self {
            freq: Some(freq),
            sample_rate: Some(sample_rate),
            bus,
            main_send,
        }
    }
}

pub fn square_oscillator(freq: f64, sample_rate: f64) -> Square<ConstHz> {
    signal::rate(sample_rate).const_hz(freq).square()
}

pub fn sine_oscillator(freq: f64, sample_rate: f64) -> Sine<ConstHz> {
    signal::rate(sample_rate).const_hz(freq).sine()
}

pub fn saw_oscillator(freq: f64, sample_rate: f64) -> Saw<ConstHz> {
    signal::rate(sample_rate).const_hz(freq).saw()
}

// TODO: Custom triangle wave oscilator
