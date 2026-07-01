#![no_std]

pub mod biquad;
pub mod fft;
pub mod oscillators;

pub use oscillators::*;

/// Represents a DSP module that inputs I, and outputs O
/// I and O represent the per-sample IO types.
pub trait Module<I, O> {
    fn process(&mut self, input: I) -> O;
}

/// This macro has 4 parameters:
/// generate_process_enum!(EnumName, InputType, OutputType, (Module1, Module2, Module3))
///
/// This generates an enum with the name specified that wraps a collection of modules.
/// Each module must implement the Module<I, O> trait for a concrete input and output type I, and O.
/// You must also match the I and O types you are implementing this for with InputType and OutputType
///
/// The macro implements a function `get_process` on the enum it creates.
/// This function returns a function pointer to the `process`
/// method from each Module's `Module` trait implementation.
///
/// So, if Module<f32, f32> is implemented for each module in your list, you can call:
/// generate_process_enum!(ModuleEnum, f32, f32, (Module1, Module2));
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
/// If your modules have multiple common implementations of Module with different I, O types, you
/// can use this macro multiple times to generate multiple enums for the different I, O types.
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
    ($enum:ident, $input_t:ty, $output_t:ty, ($($type:ident),+)) => { use paste::paste; paste! {
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
            fn [<process_ $type:lower>](enum_input: &mut $enum, input: $input_t) -> $output_t {
                 // TODO: debug_assert! that the varient matches then replace
                 // unreachable! with unreachable_unchecked!
                 match enum_input {
                     $enum::$type(inner) => inner.process(input),
                     _ => unreachable!(),
                 }
            }
            )+

            pub fn get_process(&self) -> for<'a> fn(&'a mut $enum, $input_t) -> $output_t {
                match self {
                    $(
                    $enum::$type(_) => Self::[<process_ $type:lower>],
                    )+
                }
            }
        }
    }};
}
