The goal of this project is to build a synthesizer with an STM32H755 utilizing a hardware-agnostic synthesizer library.

Right now the project consists of the following components:

[dsp](https://github.com/GregShiner/unnamed-synth-dsp):

  The primary synthesizer and digital signal processing library. This is a hardware-agnostic, `#![no_std]` library crate. 
  This means that it can be built to target both normal application CPUs and OSs, but also embedded systems
  While it is being purpose built for this STM32H755 based project, there isn't a whole lot stopping you from using it
  in just about any other project. It's primary dependancy is [dasp](https://docs.rs/dasp/latest/dasp/index.html), which itself has no other dependancies.
  It is built without any features that require allocations or the standard library.

[test interface](https://github.com/GregShiner/unnamed-synth-test-interface):

  This is a simple desktop GUI application that serves to demonstrate, test, and debug the functionality of the dsp library without the need for an embedded board. 
  It contains some simple controls to configure an oscillator, and a simple oscilloscope to visualize the output. This is a very rudementary program primarily used for
  my own development efforts on the core dsp library. Don't expect too much here lol.
