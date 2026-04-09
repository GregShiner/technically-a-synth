use dasp_signal::Signal;
use dsp::biquad::BiquadFilter;
use dsp::envelope::ADSR;
use dsp::{saw_oscillator, sine_oscillator, square_oscillator};
use nih_plug::prelude::*;
use std::f32::consts::FRAC_1_SQRT_2;
use std::sync::Arc;

/// A test tone generator that can either generate a sine wave based on the plugin's parameters or
/// based on the current MIDI input.
pub struct TASVst {
    params: Arc<TASParams>,
    sample_rate: f32,
    midi_note_id: u8,
    midi_note_gain: Smoother<f32>,
    oscillator: Option<Box<dyn Signal<Frame = f64> + Send>>,
    filter: BiquadFilter, // TODO: Make generic filter trait
    envelope: ADSR,       // TODO: Make generic envelope trait
}

#[derive(Enum, PartialEq)]
enum OscType {
    Sine,
    Saw,
    Square,
}

#[derive(Params)]
struct TASParams {
    #[id = "osc_type"]
    osc_type: EnumParam<OscType>,
    #[id = "cutoff"]
    cutoff: FloatParam,
    #[id = "q"]
    q: FloatParam,
    #[id = "attack"]
    attack: FloatParam,
    #[id = "decay"]
    decay: FloatParam,
    #[id = "sustain"]
    sustain: FloatParam,
    #[id = "release"]
    release: FloatParam,
}

impl Default for TASVst {
    fn default() -> Self {
        Self {
            params: Arc::new(TASParams::default()),
            sample_rate: 1.0,
            midi_note_id: 0,
            midi_note_gain: Smoother::new(SmoothingStyle::Linear(5.0)),
            oscillator: None,
            filter: BiquadFilter::default(),
            envelope: ADSR::default(),
        }
    }
}

impl Default for TASParams {
    fn default() -> Self {
        Self {
            osc_type: EnumParam::new("Source Oscillator Type", OscType::Sine),
            cutoff: FloatParam::new(
                "Cutoff",
                10000.0,
                FloatRange::Linear {
                    min: 10.0,
                    max: 24000.0,
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(3.0))
            .with_step_size(10.0)
            // We purposely don't specify a step size here, but the parameter should still be
            // displayed as if it were rounded. This formatter also includes the unit.
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            q: FloatParam::new(
                "Quality",
                FRAC_1_SQRT_2,
                FloatRange::Linear {
                    min: 0.5,
                    max: 10.0,
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(3.0))
            .with_step_size(0.01),
            attack: FloatParam::new(
                "Attack",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_step_size(0.01),
            decay: FloatParam::new(
                "Decay",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_step_size(0.01),
            sustain: FloatParam::new("Sustain", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.001),
            release: FloatParam::new(
                "Release",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_step_size(0.01),
        }
    }
}

impl Plugin for TASVst {
    const NAME: &'static str = "Technically A Synth Vst";
    const VENDOR: &'static str = "Greg Shiner";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "greg@gregshiner.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            // This is also the default and can be omitted here
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;

        true
    }

    fn reset(&mut self) {
        self.midi_note_id = 0;
        self.midi_note_gain.reset(0.0);
        self.oscillator = None;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();
        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            let cutoff = self.params.cutoff.smoothed.next() as f64;
            let sample_rate = self.sample_rate as f64;
            let q = self.params.q.smoothed.next() as f64;
            self.filter.update_low_pass(cutoff, sample_rate, q);

            // Act on the next MIDI event
            while let Some(event) = next_event {
                if event.timing() > sample_id as u32 {
                    break;
                }

                match event {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        self.midi_note_id = note;
                        let freq = util::midi_note_to_freq(note) as f64;
                        self.midi_note_gain.set_target(self.sample_rate, velocity);
                        self.oscillator = Some(match self.params.osc_type.value() {
                            OscType::Sine => Box::new(sine_oscillator(freq, sample_rate)),
                            OscType::Saw => Box::new(saw_oscillator(freq, sample_rate)),
                            OscType::Square => Box::new(square_oscillator(freq, sample_rate)),
                        });
                    }
                    NoteEvent::NoteOff { note, .. } if note == self.midi_note_id => {
                        self.midi_note_gain.set_target(self.sample_rate, 0.0);
                    }
                    // NoteEvent::PolyPressure { note, pressure, .. } if note == self.midi_note_id => {
                    //     self.midi_note_gain.set_target(self.sample_rate, pressure);
                    // }
                    _ => (),
                }

                next_event = context.next_event();
            }

            // This gain envelope prevents clicks with new notes and with released notes
            let next_sample = if let Some(osc) = &mut self.oscillator {
                self.filter.process(osc.next()) as f32 * self.midi_note_gain.next()
            } else {
                0.0
            };

            for sample in channel_samples {
                *sample = next_sample;
            }
        }

        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for TASVst {
    const CLAP_ID: &'static str = "com.gregshiner.technically-a-synth-vst";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A VST/CLAP plugin implementation for testing my dsp library");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for TASVst {
    const VST3_CLASS_ID: [u8; 16] = *b"TechnicallySynth";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Tools,
    ];
}

nih_export_clap!(TASVst);
nih_export_vst3!(TASVst);
