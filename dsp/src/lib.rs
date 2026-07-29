#![no_std]

pub mod biquad;
pub mod fft;
pub mod graph;
pub mod oscillators;

pub use oscillators::*;

const MAX_NODE_INPUTS: usize = 3;
const MAX_NODE_OUTPUTS: usize = 2;
const MAX_NODE_SCRATCH_BUFFERS: usize = 1;

pub type AudioSample = f32;
pub type AudioBuffer<const BUF_SIZE: usize> = [AudioSample; BUF_SIZE];
pub type BigFuckinBuffer<const BUF_SIZE: usize, const NUM_BUFS: usize> =
    [AudioBuffer<BUF_SIZE>; NUM_BUFS];

/// Represents a DSP module that inputs I, and outputs O
/// I and O represent the per-sample IO types.
pub trait Module {
    const INPUTS: usize;
    const OUTPUTS: usize;
    const SCRATCH_BUFFERS: usize;

    fn process(
        &mut self,
        input_buffers: &[&[f32]],
        output_buffers: &mut [&mut [f32]],
        scratch_buffers: &mut [&mut [f32]],
    ) -> ();
}

/// This macro has 2 parameters:
/// generate_process_enum!(EnumName, (Module1, Module2, Module3))
///
/// This generates an enum with the name specified that wraps a collection of modules.
/// Each module must implement the Module trait.
///
/// The macro implements a function `get_process` on the enum it creates.
/// This function returns a function pointer to the `process`
/// method from each Module's `Module` trait implementation.
///
/// So, if Module is implemented for each module in your list, you can call:
/// generate_process_enum!(ModuleEnum, (Module1, Module2));
/// This will create an enum called `ModuleEnum` with 2 variants:
/// pub enum ModuleEnum {
///     Module1(Module1),
///     Module2(Module2),
/// }
/// NOTE: The variant names generated are just the same as the type name
///
/// You can then call ModuleEnum::get_process(&self) on an instance of ModuleEnum
/// to get that variant's process function.
///
/// Technical Detail:
/// Rust doesn't let you get a function pointer to a trait method, so this macro generates a small
/// wrapper function that destructures the enum and calls the enclosed function.
/// The idea behind this is that rather than implementing the `Module` trait on the enum by pattern
/// matching every time you call `process()`, you pattern match only when the patch changes, and
/// then you just store the function pointer to the process function. TBH, I don't know if this is
/// really an necessary, or even effective, optimization. I just really felt like writting a macro.
#[macro_export]
macro_rules! generate_process_enum {
    ($enum:ident, ($($type:ident),+)) => { use paste::paste; paste! {
        // Generates an enum with the Module types specified
        // The variant name will be the same as the enclosed type
        pub enum $enum {
            $(
            $type($type),
            )+
        }

        impl $enum {
            // For each type, create a function that calls process for the enclosed type
            $(
            fn [<process_ $type:lower>](
                enum_input: &mut $enum,
                input_buffers: &[&[f32]],
                output_buffers: &mut [&mut [f32]],
                scratch_buffers: &mut [&mut [f32]]
            ) {
                 // TODO: debug_assert! that the varient matches then replace
                 // unreachable! with unreachable_unchecked!
                 match enum_input {
                     $enum::$type(inner) => inner.process(input_buffers, output_buffers, scratch_buffers),
                     _ => unreachable!(),
                 }
            }
            )+

            // holy crap this is ugly
            pub fn get_process(&self) -> for<'a, 'b, 'c, 'd, 'e, 'f, 'g> fn(&'a mut Oscillator, &'b [&'c [f32]], &'d mut [&'e mut [f32]], &'f mut [&'g mut [f32]]) {
                match self {
                    $(
                    $enum::$type(_) => Self::[<process_ $type:lower>],
                    )+
                }
            }
        }
    }};
}
