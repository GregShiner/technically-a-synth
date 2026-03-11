#![no_std]

use dasp::signal::{self, ConstHz, Saw, Sine, Square};

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
