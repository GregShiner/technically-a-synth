use dasp::Signal;

enum ADSRState {
    NotStarted, // Key not pressed
    ADS,        // Key pressed
    R,          // Key released
}
pub struct ADSR {
    attack: u32,       // Number of samples at sample_rate
    decay: u32,        // Number of samples at sample_rate
    sustain: f64,      // Amplitude 0 to 1
    release: u32,      // Number of samples at sample_rate
    sample_rate: f64,  // Samples / sec
    sample_step: u32,  // Current step of envelope
    key_pressed: bool, // When true, goes through ADS phase. Once false, goes through R phase
    started: bool,     // True once the key is pressed, never set back to false
    ended: bool,       // True once the key is released after previously being pressed
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
            started: key_pressed,
            ended: false,
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
}

fn lerp(start: f64, end: f64, t: f64) -> f64 {
    // Some performance notes here: https://en.wikipedia.org/wiki/Linear_interpolation#Programming_language_support
    start + t * (end - start)
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
            started,
            ended,
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

        // let prev_state = self.state;
        // self.state = match key_pressed {
        //     true => ADSRState::ADS;
        //     false => if prev_state == ADSRState::ADS {
        //         ADSRState::R
        //     }
        // }
        // TODO: figure out if this logic works for multiple presses
        let sample = if key_pressed {
            self.started = true;
            self.ended = false;
            if sample_step <= attack {
                lerp(0.0, 1.0, sample_step as f64 / attack as f64)
            } else if sample_step <= decay {
                lerp(1.0, sustain, (sample_step - attack) as f64 / decay as f64)
            } else {
                sustain
            }
        } else if started {
            self.ended = true;
            if !ended {
                self.sample_step = 0;
            }
            lerp(sustain, 0.0, sample_step as f64 / release as f64)
        } else {
            0.0
        };

        self.sample_step += 1;
        sample
    }
}
