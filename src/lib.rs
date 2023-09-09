#![allow(unused_variables)]
use nih_plug::prelude::*;
use std::sync::Arc;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct FloatCrush {
    params: Arc<FloatCrushParams>,
}

#[derive(Params)]
struct FloatCrushParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "input gain"]
    pub input_gain: FloatParam,

    #[id = "round"]
    pub round: IntParam,

    #[id = "exponent"]
    pub exponent: FloatParam,

    #[id = "exponent bias"]
    pub exponent_bias: FloatParam,

    #[id = "mantissa"]
    pub mantissa: FloatParam,

    #[id = "mantissa bias"]
    pub mantissa_bias: FloatParam,

    #[id = "dry"]
    pub dry: FloatParam,

    #[id = "wet"]
    pub wet: FloatParam,
}

impl Default for FloatCrush {
    fn default() -> Self {
        Self {
            params: Arc::new(FloatCrushParams::default()),
        }
    }
}

impl Default for FloatCrushParams {
    fn default() -> Self {
        Self {
            input_gain: FloatParam::new(
                "drive",
                1.,
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.),
                    max: util::db_to_gain(30.),
                    factor: FloatRange::gain_skew_factor(-30., 30.)
                }
            )
            .with_unit(" db")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            round: IntParam::new("round", 0, IntRange::Linear { min: -1, max: 1 }),
            
            exponent: FloatParam::new(
                "exponent",
                8.,
                FloatRange::Skewed { min: 0., max: 8., factor: 1.5 }
            )
            .with_unit(" bits")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            
            exponent_bias: FloatParam::new(
                "exponent_bias",
                1.,
                FloatRange::Skewed {
                    min: 0.500000059604644775390625,
                    max: 8.,
                    factor: 0.25
                }
            ),

            mantissa: FloatParam::new(
                "mantissa",
                12.,
                FloatRange::Skewed { min: 0., max: 12., factor: 1.25 }
            )
            .with_unit(" bits")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            mantissa_bias: FloatParam::new(
                "mantissa_bias",
                0.,
                FloatRange::Linear { min: -1., max: 1. }
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            dry: FloatParam::new("dry", 0., FloatRange::Skewed {
                min: 0.,
                max: 1.,
                factor: FloatRange::gain_skew_factor(1e-5, 0.)
            })
            .with_unit(" db")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            wet: FloatParam::new("wet", 1., FloatRange::Skewed {
                min: 0.,
                max: 1., 
                factor: FloatRange::gain_skew_factor(1e-5, 0.)
            })
            .with_unit(" db")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

        }
    }
}

impl Plugin for FloatCrush {
    const NAME: &'static str = "Float Crush";
    const VENDOR: &'static str = "Katlyn Thomas";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "katlyn.c.thomas@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];


    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            let exponent = 2_f32.powf(self.params.exponent.value()).round() as u32;
            let e_bias = self.params.exponent_bias.value();

            let mantissa = 2_f32.powf(self.params.mantissa.value()).round() as u32;
            let m_bias = 50000_f32.powf(self.params.mantissa_bias.value());

            let input_gain = self.params.input_gain.value();
            let dry_gain = self.params.dry.value();
            let wet_gain = self.params.wet.value();

            let quantizer = Quantizator::from_i32(self.params.round.value());
            
            for sample in channel_samples {
                let sample_dry = sample.clone();
                let sample_wet = sample.clone();

                // apply input drive
                let sample_wet = sample_wet * input_gain;

                if sample_wet.abs() >= 1. {
                    // clip sample, mix w/ dry according to dry/wet settings
                    *sample = mix_dry_wet(
                        sample_dry,
                        dry_gain,
                        sample_wet / sample_wet.abs(),
                        wet_gain
                    );
                    continue;
                }

                if exponent < 1 && mantissa < 1 {
                    // no search necessary, quantize to zero or one
                    let polarity = if sample_wet.is_sign_positive() { 1_f32 } else { -1_f32 };
                    let sample_wet = quantizer.quantize_abs(1., 0., sample_wet.abs()) * polarity;
                    *sample = mix_dry_wet(sample_dry, dry_gain, sample_wet, wet_gain);
                    continue;
                }
                
                let sample_wet = {
                    // if exponent == 0 {
                        search_mantissa(
                            mantissa,
                            m_bias,
                            SampleRange::new(1., 0.),
                            sample_wet,
                            quantizer
                        )
                    // } else {
                    //     let sample_abs = sample_wet.abs();
                    //     let mut position = Position {
                    //         sample: 1.,
                    //         index: 0,
                    //         range: 0.5,
                    //         error: 1. - sample_abs,
                    //     };
                    //
                    //     loop {
                    //         let other_index = position.index + (exponent as f32 * position.range).round() as i32;
                    //         let other_sample = 1. / (e_bias * 2.).powi(other_index);
                    //         let other_err = other_sample - sample_abs;
                    //
                    //         // if a sample is already in a quantized position, return it
                    //         if other_sample == sample_abs { break sample_wet; }
                    //
                    //         if position.index - other_index <= 1 {
                    //             break search_mantissa(
                    //                 mantissa,
                    //                 m_bias,
                    //                 (position.sample, other_sample),
                    //                 sample_wet,
                    //                 quantizer
                    //             )
                    //         }
                    //
                    //         if other_err.is_sign_negative() {
                    //             position.range *= 0.5;
                    //         } else {
                    //             position.index = other_index;
                    //             position.sample = other_sample;
                    //             position.error = other_err;
                    //         }
                    //     }
                    // }
                };
                *sample = (sample_dry * dry_gain) + (sample_wet * wet_gain)
            }
        }

        ProcessStatus::Normal
    }
}

fn mix_dry_wet(dry: f32, dry_gain: f32, wet: f32, wet_gain: f32) -> f32 {
    (dry * dry_gain) + (wet * wet_gain)
}

#[derive(Clone, Copy)]
 enum Quantizator {
    RoundUp,
    Nearest,
    RoundDown,
}

impl Quantizator {
    pub fn from_i32(int: i32) -> Self {
        if int == 0 {
            Self::Nearest
        } else if int > 0 {
            Self::RoundUp
        } else {
            Self::RoundDown
        }
    }

    pub fn quantize_abs(&self, upper_bound: f32, lower_bound: f32, sample: f32) -> f32 {
        match self {
            Self::Nearest => {
                let midpoint = (upper_bound + lower_bound) / 2.;

                if sample > midpoint { upper_bound }
                else { lower_bound }
            },
            Self::RoundUp => upper_bound,
            Self::RoundDown => lower_bound,
        }
    }
}

struct SearchRange {
    start: u32,
    length: u32,
    mantissa: u32,
    range: SampleRange,
    sample: f32,
}

impl SearchRange {
    pub fn new(mantissa: u32, range: SampleRange, sample: f32) -> anyhow::Result<Self> {
        if !range.in_range(sample) { anyhow::bail!("not in range") }
        else {
            Ok(Self { start: 0, length: mantissa, mantissa, range, sample })
        }
    }

    pub fn center(&self) -> u32 {
        self.start + self.half_length()
    }

    fn half_length(&self) -> u32 {
        (self.length as f32 / 2.).floor() as u32
    }

    pub fn cull(&mut self) -> CullResult {
        let c_sample = self.center_sample();
        if self.length == 1 {
            let start = find_m_sample(
                self.range.high,
                self.range.distance(),
                self.mantissa,
                self.start,
                1.
            );
            let end = find_m_sample(
                self.range.high,
                self.range.distance(),
                self.mantissa,
                self.start + 1,
                1.
            );
            CullResult::TwoLeft(start, end, self.sample)
        } else if c_sample == self.sample {
            CullResult::ExactMatch(self.sample)
        } else if c_sample > self.sample {
            self.start = self.center();
            self.length = self.length - self.half_length();
            CullResult::CutHalf
        } else {
            self.length = self.length - self.half_length();
            CullResult::CutHalf
        }
    }

    pub fn center_sample(&self) -> f32 {
        find_m_sample(
            self.range.high,
            self.range.distance(),
            self.mantissa,
            self.center(),
            1.
        )
    }
}

enum CullResult {
    CutHalf,
    TwoLeft(f32, f32, f32),
    ExactMatch(f32)
}

#[derive(Clone, Copy)]
struct SampleRange {
    high: f32, 
    low: f32
}

impl SampleRange {
    pub fn new(a: f32, b: f32) -> SampleRange {
        if a >= b { SampleRange { high: a, low: b } }
        else { SampleRange { high: b, low: a } }
    }

    pub fn distance (&self) -> f32 {
        self.high - self.low
    }

    pub fn in_range(&self, sample: f32) -> bool {
        sample >= self.low && sample <= self.high
    }
}

fn search_mantissa(mantissa: u32, m_bias: f32, range: SampleRange, sample: f32, quantizer: Quantizator) -> f32 {
    let sample_abs = sample.abs();
    let polarity = sample / sample_abs;

    if sample_abs < range.low {
        return quantizer.quantize_abs(range.low, 0., sample_abs) * polarity;
    }

    if mantissa == 0 {
        return quantizer.quantize_abs(range.high, range.low, sample_abs) * polarity;
    }

    let mut search_range = SearchRange::new(mantissa, range, sample).unwrap();


    loop {
        let result = search_range.cull();
        match result {
            CullResult::ExactMatch(sample) => break sample,
            CullResult::TwoLeft(upper, lower, sample) => 
                break quantizer.quantize_abs(upper, lower, sample),
            CullResult::CutHalf => (),
        }
    }

}

fn find_m_sample(high_end: f32, sample_range: f32, mantissa: u32, index: u32, m_bias: f32) -> f32 {
    high_end - if m_bias == 1. {
        let step_size = sample_range / mantissa as f32;
        step_size * index as f32
    } else {
        // normalize mantissa to 0.0 - 1.0
        let m = index as f32 / mantissa as f32;
        let position = (m_bias.powf(m) - 1.) / (m_bias - 1.);
        position * sample_range
    }
}

impl ClapPlugin for FloatCrush {
    const CLAP_ID: &'static str = "com.your-domain.float-crush";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("low bit floating point quantization?");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = 
        &[ClapFeature::AudioEffect, ClapFeature::Distortion, ClapFeature::Glitch];
}

impl Vst3Plugin for FloatCrush {
    const VST3_CLASS_ID: [u8; 16] = *b"Exactly16Chars!!";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(FloatCrush);
nih_export_vst3!(FloatCrush);

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! linear_mantissa {
        ($bits:expr, $sample:expr, $expected:expr) => {
            paste::item! {
                #[test]
                fn [ < linear_mantissa_ $bits > ] () {
                    let sample_one = search_mantissa(
                        $bits,
                        1.,
                        SampleRange::new(1., 0.),
                        $sample,
                        Quantizator::Nearest,
                    );

                    assert_eq!($expected, sample_one);
                }
            }
        };
    }

    linear_mantissa!(0, 0.6, 1.);
    linear_mantissa!(1, 0.6, 1.);
    linear_mantissa!(2, 0.6, 0.5);
    linear_mantissa!(4, 0.6, 0.5);
    linear_mantissa!(8, 0.6, 0.625);
    linear_mantissa!(16, 0.6, 0.625);
    linear_mantissa!(32, 0.6, 0.59375);
    linear_mantissa!(64, 0.6, 0.59375);

    #[test]
    fn test_find_m_sample() {
        let s = find_m_sample(1., 1., 10, 0, 1.);
        assert_eq!(s, 1.);
        let s = find_m_sample(1., 1., 10, 1, 1.);
        assert_eq!(s, 0.9);
        let s = find_m_sample(1., 1., 10, 2, 1.);
        assert_eq!(s, 0.8);
        let s = find_m_sample(1., 1., 4, 3, 1.);
        assert_eq!(s, 0.25);
    }

    #[test]
    fn sample_range() {
        let t = SampleRange::new(4., 5.);

        assert_eq!(t.high, 5.);
        assert_eq!(t.low, 4.);
        assert_eq!(t.distance(), 1.);
    }
}
