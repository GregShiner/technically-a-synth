The goal of this project is to build a synthesizer with an STM32H755 utilizing a hardware-agnostic synthesizer library.

Right now the project consists of the following components:

DSP:

  The primary synthesizer and digital signal processing library. This is a hardware-agnostic, `#![no_std]` library crate.
  This means that it can be built to target both normal application CPUs and OSes, but also embedded systems
  While it is being purpose built for this STM32H755 based project, there isn't much stopping you from using it
  in just about any other project. Its primary dependency is [dasp](https://docs.rs/dasp/latest/dasp/index.html), which itself has no other dependencies.
  It is built without any features that require allocations or the standard library.

Test Interface:

  This is a simple desktop GUI application that serves to demonstrate, test, and debug the functionality of the DSP library without the need for an embedded board.
  It contains some simple controls to configure an oscillator, and a simple oscilloscope to visualize the output. This is a very rudimentary program primarily used for
  my own development efforts on the core DSP library. Don't expect too much here lol.

VST:

  This is largely to replace the Test Interface. This is a simple VST implementation of the DSP library once again meant to debug the DSP library.

# TODO:
## DSP:
- [x] 3 Basic Oscillators
- [x] Biquad Filter
- [x] Low Pass Filter
- [ ] Envelope
- [ ] LFO
- [ ] LFO/Envelope Controlled Parameters
- [ ] Attenuation
- [ ] Polyphony
- [ ] Reverb
- [ ] Distortion
- [ ] Down Sample
- [ ] Bit Crush
## Hardware:
- [ ] Write an oscillator to a serial bus
- [ ] Apply a filter
- [ ] Apply an envelope
- [ ] DMA
- [ ] DAC
- [ ] Get a line level out
- [ ] Take a key MIDI input
- [ ] Hook up a display
- [ ] Take control MIDI inputs
