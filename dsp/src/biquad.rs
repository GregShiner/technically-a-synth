use core::f64::consts::PI;

#[derive(Default)]
pub struct BiquadFilter {
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl BiquadFilter {
    pub fn new(a1: f64, a2: f64, b0: f64, b1: f64, b2: f64) -> Self {
        Self {
            a1,
            a2,
            b0,
            b1,
            b2,
            ..Self::default()
        }
    }

    pub fn low_pass(cutoff: f64, sample_rate: f64, q: f64) -> Self {
        // https://en.wikipedia.org/wiki/Digital_biquad_filter#Bilinear_transform_examples
        let w0 = 2.0 * PI * cutoff / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        // Calculate a0 first because everything is normalized by a0
        let a0 = 1.0 + alpha;

        let b0 = ((1.0 - cos_w0) / 2.0) / a0;
        let b1 = (1.0 - cos_w0) / a0;
        let b2 = ((1.0 - cos_w0) / 2.0) / a0;

        let a1 = (-2.0 * cos_w0) / a0;
        let a2 = (1.0 - alpha) / a0;

        Self {
            a1,
            a2,
            b0,
            b1,
            b2,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }


    pub fn process(&mut self, x: f64) -> f64 {
        // https://en.wikipedia.org/wiki/Digital_biquad_filter#Direct_form_1
        let y = (self.b0 * x) + (self.b1 * self.x1) + (self.b2 * self.x2)
            - (self.a1 * self.y1)
            - (self.a2 * self.y2);
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}
