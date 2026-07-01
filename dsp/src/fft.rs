use dasp::Signal;
use libm::{cosf, sqrtf};
use microfft::{Complex32, real::rfft_1024};

const FFT_BUFFER_SIZE: usize = 1024;

pub struct FFTAnalyzer<S: Signal<Frame = f32>> {
    inner: S,
    fft_buffer: [f32; FFT_BUFFER_SIZE],
    fft_cursor: usize,
}

impl<S: Signal<Frame = f32>> FFTAnalyzer<S> {
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
            let hann =
                0.5 * (1.0 - cosf(2.0 * core::f32::consts::PI * i as f32 / FFT_BUFFER_SIZE as f32));
            ordered[i] *= hann;
        });
        *rfft_1024(&mut ordered)
    }
}

impl<S: Signal<Frame = f32>> Signal for FFTAnalyzer<S> {
    type Frame = f32;
    fn next(&mut self) -> f32 {
        let fft_sample = self.inner.next();

        self.fft_buffer[self.fft_cursor] = fft_sample as f32;
        self.fft_cursor = (self.fft_cursor + 1) % FFT_BUFFER_SIZE;
        fft_sample
    }
}
pub fn complex_magnitudes<const N: usize>(complex: [Complex32; N]) -> [f32; N] {
    complex.map(|c| sqrtf(c.re * c.re + c.im * c.im))
}
