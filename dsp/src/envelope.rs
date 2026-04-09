use dasp::Signal;

pub struct ADSR {
    attack: u32,       // Number of samples at sample_rate
    decay: u32,        // Number of samples at sample_rate
    sustain: f64,      // Amplitude 0 to 1
    release: u32,      // Number of samples at sample_rate
    sample_rate: f64,  // Samples / sec
    sample_step: u32,  // Current step of envelope (gets reset when entering release phase)
    key_pressed: bool, // When true, goes through ADS phase. Once false, goes through R phase
    active: bool,      // True once the key is pressed, never set back to false
    released: bool,    // True once the key is released after previously being pressed
}

impl ADSR {
    pub fn new(
        attack: u32,
        decay: u32,
        sustain: f64,
        release: u32,
        sample_rate: f64,
        key_pressed: bool,
    ) -> Self {
        Self {
            attack,
            decay,
            sustain,
            release,
            sample_rate,
            sample_step: 0,
            key_pressed,
            // state: if key_pressed {
            //     ADSRState::ADS
            // } else {
            //     ADSRState::NotStarted
            // },
            active: key_pressed,
            released: false,
        }
    }

    pub fn from_seconds(
        attack: f64,
        decay: f64,
        sustain: f64,
        release: f64,
        sample_rate: f64,
        key_pressed: bool,
    ) -> Self {
        let attack_samples = (attack * sample_rate) as u32;
        let decay_samples = (decay * sample_rate) as u32;
        let release_samples = (release * sample_rate) as u32;
        Self::new(
            attack_samples,
            decay_samples,
            sustain,
            release_samples,
            sample_rate,
            key_pressed,
        )
    }

    fn note_on(&mut self) {
        self.sample_step = 0;
        self.key_pressed = true;
    }

    fn note_off(&mut self) {
        self.key_pressed = false;
    }

    /// Updates the ADSR parameters
    /// May be unstable if changing parameters while a note is active
    pub fn update_unchecked(&mut self, a: f64, d: f64, s: f64, r: f64) {
        self.attack = (a * self.sample_rate) as u32;
        self.decay = (d * self.sample_rate) as u32;
        self.sustain = s;
        self.release = (r * self.sample_rate) as u32;
    }

    /// Updates ADSR parameters but only if the ADSR is inactive
    /// Returns true if parameters were updated, false if not
    /// You probably want to continue doing this between samples until its true
    pub fn update(&mut self, a: f64, d: f64, s: f64, r: f64) -> bool {
        if self.active {
            return false;
        };
        self.update_unchecked(a, d, s, r);
        true
    }
}

fn lerp(start: f64, end: f64, t: f64) -> f64 {
    // Some performance notes here: https://en.wikipedia.org/wiki/Linear_interpolation#Programming_language_support
    start + t * (end - start)
}

fn clamped_lerp(start: f64, end: f64, t: f64) -> f64 {
    if t <= 0.0 {
        start
    } else if t >= 1.0 {
        end
    } else {
        lerp(start, end, t)
    }
}

// t = sample_step / attack,decay,release

impl Signal for ADSR {
    type Frame = f64;

    fn next(&mut self) -> Self::Frame {
        let Self {
            attack,
            decay,
            sustain,
            release,
            sample_rate: _,
            sample_step,
            key_pressed,
            active,
            released,
        } = *self;
        // If key pressed (ADS phase)
        //      if sample <= attack:
        //          ret lerp(0, 1, sample / attack)
        //      elif sample <= decay:
        //          ret lerp(1, sustain, (sample - attack) / decay)
        //      else // in hold phase
        //          ret sustain
        // Else (in R phase)
        //      ret lerp(sustain, 0, (sample but only in R phase) / decay)

        // Key Pressed
        let sample = if key_pressed {
            self.active = true;
            self.released = false;
            // Attack Phase
            if sample_step <= attack {
                clamped_lerp(0.0, 1.0, sample_step as f64 / attack as f64)
            // Decay Phase
            } else if sample_step <= attack + decay {
                clamped_lerp(1.0, sustain, (sample_step - attack) as f64 / decay as f64)
            // Sustain Phase
            } else {
                sustain
            }
        // Key released but note already started; Release Phase
        } else if active {
            self.released = true;
            // If it was not ended on the previous step, reset the counter
            if !released {
                self.sample_step = 0;
            }
            if self.sample_step > release {
                self.active = false;
            }
            clamped_lerp(sustain, 0.0, self.sample_step as f64 / release as f64)
        } else {
            0.0
        };
        self.sample_step += 1;

        sample
    }
}

impl Default for ADSR {
    fn default() -> Self {
        Self::new(0, 0, 1.0, 0, 44100.0, false)
    }
}
