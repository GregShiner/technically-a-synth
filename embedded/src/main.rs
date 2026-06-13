#![no_main]
#![no_std]

use core::mem::MaybeUninit;

use embassy_stm32::{bind_interrupts, peripherals};
use embedded as _; // global logger + panicking-behavior + memory layout

rtic_monotonics::systick_monotonic!(Mono, 1_000);

#[unsafe(link_section = ".ram_d3")]
static SHARED_DATA: MaybeUninit<embassy_stm32::SharedData> = MaybeUninit::uninit();

type AudioSample = i16;
const BUFFER_SAMPLES: usize = 256;
type SampleBuffer = [AudioSample; BUFFER_SAMPLES];
const SAMPLE_RATE_U: u32 = 44100;
const SAMPLE_RATE: f64 = SAMPLE_RATE_U as f64;
const MIDDLE_C: f64 = 261.6256;

#[derive(PartialEq)]
pub enum BufferState {
    PendingRead,
    PendingWrite,
}

bind_interrupts!(struct Irqs {
    // I dont think I need these
    //OTG_HS_EP1_OUT => embassy_stm32::usb::InterruptHandler<peripherals::USB_OTG_HS>;
    //OTG_HS_EP1_IN  => embassy_stm32::usb::InterruptHandler<peripherals::USB_OTG_HS>;
    //OTG_HS_WKUP    => embassy_stm32::usb::InterruptHandler<peripherals::USB_OTG_HS>;
    DMA1_STREAM0 => embassy_stm32::dma::InterruptHandler<peripherals::DMA1_CH0>;
});

#[rtic::app(
    device = embassy_stm32,
    dispatchers = [DFSDM1_FLT0, DFSDM1_FLT1, DFSDM1_FLT2, DFSDM1_FLT3],
    peripherals = true
)]
mod app {
    use dasp::{
        Signal,
        signal::{ConstHz, Sine, Square},
    };
    use defmt::{debug, info, warn};
    use embassy_stm32::{self as hal, gpio, i2s, rcc, spi, time};
    use fugit::ExtU32;
    use rtic_monotonics::Monotonic;

    use super::*;
    use dsp::{sine_oscillator, square_oscillator};

    // Shared resources go here
    #[shared]
    struct Shared {
        ping: SampleBuffer,
        pong: SampleBuffer,
        ping_state: BufferState,
        pong_state: BufferState,
    }

    // Local resources go here
    #[local]
    struct Local {
        square_osc: Sine<ConstHz>,
        i2s2: i2s::I2S<'static, u32>,
    }

    #[init(local = [dma_buffer: [u32; 512] = [0u32; 512]])]
    fn init(cx: init::Context) -> (Shared, Local) {
        info!("init");

        let cp = cx.core;

        let mut config = hal::Config::default();
        config.rcc.pll1 = Some(rcc::Pll {
            source: rcc::PllSource::HSI,    // 64 MHz -> DIVM1
            prediv: rcc::PllPreDiv::DIV4,   // DIVM1 = 4: 16 MHz -> DIVN1
            mul: rcc::PllMul::MUL60,        // DIVN1 = 60: 960 MHz -> DIVP1 + DIVQ1 + DIVR1
            divp: Some(rcc::PllDiv::DIV2),  // DIVP1 = 2: 480 MHz -> System clock + more
            divq: Some(rcc::PllDiv::DIV16), // DIVQ1 = 16: 60 MHz -> SPI1 + more
            divr: None,                     // Disabled
        });
        // Allow higher clock speed
        // (This isn't technically necessary since Scale0 is the default
        config.rcc.voltage_scale = rcc::VoltageScale::Scale0;
        // Set the system clock source to PLL1
        config.rcc.sys = rcc::Sysclk::PLL1_P;
        // Divide some peripheral prescalers to keep them within limits
        config.rcc.ahb_pre = rcc::AHBPrescaler::DIV2; // HPRE Prescaler
        config.rcc.apb1_pre = rcc::APBPrescaler::DIV2; // D2PRE1
        config.rcc.apb2_pre = rcc::APBPrescaler::DIV2; // D2PRE2
        config.rcc.apb3_pre = rcc::APBPrescaler::DIV2; // D1PRE
        config.rcc.apb4_pre = rcc::APBPrescaler::DIV2; // D3PRE
        config.rcc.supply_config = rcc::SupplyConfig::DirectSMPS; // THIS MAKES EVERYTHING WORK!

        debug!("Initializing HAL...");
        let p = hal::init_primary(config, &SHARED_DATA);
        debug!("HAL Initialized");

        // PB15 SPI2 I2S2_SDO
        // PB12 SPI2 I2S2_WS
        // PD3  SPI2 I2S2_CK
        let spi2 = p.SPI2;
        let i2s2_sdo = p.PB15;
        let i2s2_ws = p.PB12;
        let i2s2_ck = p.PD3;
        let dma1_ch0 = p.DMA1_CH0;
        let mut i2s_config = i2s::Config::default();
        // I'm pretty sure this is audio frequency since the default is 48kHz
        // It would suck if this was bit frequency
        i2s_config.frequency = time::hz(SAMPLE_RATE_U);
        // I don't really know whats required for this, but this is the default
        // Hopefully my clock config can handle this
        i2s_config.gpio_speed = gpio::Speed::VeryHigh;
        // The MCU will be the device thats sending
        i2s_config.mode = i2s::Mode::Master;
        // PCM5102A in "standard i2s" mode is AKA "Philips" according to one guide I found
        // https://nodeloop.org/guides/i2s-interface-guide/
        i2s_config.standard = i2s::Standard::Philips;
        // The PCM5102A has 32 bit channels and might as well use all 32 of em
        i2s_config.format = i2s::Format::Data32Channel32;
        // idrk, this was the default
        i2s_config.clock_polarity = i2s::ClockPolarity::IdleLow;
        // The PCM5102A has an onboard clock that it will set based on the bit clock and lrck
        i2s_config.master_clock = false;
        let i2s2 = i2s::I2S::new_txonly_nomck(
            spi2,
            i2s2_sdo,
            i2s2_ws,
            i2s2_ck,
            dma1_ch0,
            cx.local.dma_buffer,
            Irqs,
            i2s_config,
        );

        let mono_driver = Mono::start(cp.SYST, 480_000_000); // 480 MHz System Clock
        debug!("Monotonic Started");
        fill_audio::spawn().unwrap();
        send_audio::spawn().unwrap();
        (
            Shared {
                ping: [0; BUFFER_SAMPLES],
                pong: [0; BUFFER_SAMPLES],
                ping_state: BufferState::PendingWrite,
                pong_state: BufferState::PendingWrite,
            },
            Local {
                square_osc: sine_oscillator(MIDDLE_C, SAMPLE_RATE),
                i2s2,
            },
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        info!("idle");

        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(shared = [ping, pong, ping_state, pong_state], priority = 1)]
    async fn send_audio(mut cx: send_audio::Context) {
        info!("Sending data!");
        loop {
            debug!("Starting send audio loop");
            let read_ping = cx
                .shared
                .ping_state
                .lock(|s| *s == BufferState::PendingRead);
            // NOTE: Re soundness: The buffer state can theoretically be changed between the above
            // line and the following line. However, that will never happen as long as send_audio()
            // is the only other task that may modify it, and its at the same priority as this task.
            // In RTIC, only higher priority tasks can preempt.
            if read_ping {
                debug!("Reading ping");
                // TODO: Rewrite this as no copy
                // This can be accomplished by just copying out the pointer to the array and using
                // that. But, that can only be done soundly once it's confirmed that the writer will
                // never write to a buffer while its reading.
                let mut local_buf = [0i16; BUFFER_SAMPLES];
                cx.shared.ping.lock(|buf| local_buf.copy_from_slice(buf));
                let bytes: &[u8; BUFFER_SAMPLES * size_of::<AudioSample>()] =
                    // SAFETY: The internal representation of the data doesn't really matter as long
                    // as whatever is reading from USB interprets it correctly. The only thing that
                    // would matter here is the endianess of the i16s. They should (probably) be
                    // little-endian but this might be worth double checking at some point.
                    // If the type of AudioSample changes from i16, this may need to be adjusted.
                    unsafe { core::mem::transmute(&local_buf) };
                // TODO: Write to i2s
                cx.shared
                    .ping_state
                    .lock(|s| *s = BufferState::PendingWrite);
            }

            let read_pong = cx
                .shared
                .pong_state
                .lock(|s| *s == BufferState::PendingRead);
            // NOTE: Re soundness: ditto
            if read_pong {
                debug!("Reading pong");
                // TODO: Ditto
                let mut local_buf = [0i16; BUFFER_SAMPLES];
                cx.shared.pong.lock(|buf| local_buf.copy_from_slice(buf));
                let bytes: &[u8; BUFFER_SAMPLES * size_of::<AudioSample>()] =
                    // SAFETY: Ditto
                    unsafe { core::mem::transmute(&local_buf) };
                // TODO: Write to i2s
                cx.shared
                    .pong_state
                    .lock(|s| *s = BufferState::PendingWrite);
            }
            // The other branches will await/yield when it calls write_packet(chunk)
            // But that does not cover the case when neither buffer is ready to be read from.
            if !read_ping && !read_pong {
                warn!("NO BUFFERS TO READ");
            }
            Mono::delay(1000.micros()).await;
        }
    }

    fn fill_buffer(buf: &mut SampleBuffer, osc: &mut Sine<ConstHz>) {
        fn f64_to_sample(s: f64) -> AudioSample {
            (s * AudioSample::MAX as f64).clamp(AudioSample::MIN as f64, AudioSample::MAX as f64)
                as AudioSample
        }
        buf.iter_mut().for_each(|s| *s = f64_to_sample(osc.next()));
    }

    #[task(local = [square_osc], shared = [ping, pong, ping_state, pong_state], priority = 1)]
    async fn fill_audio(mut cx: fill_audio::Context) -> ! {
        info!("Started fill_audio");
        loop {
            debug!("Starting fill audio loop");
            let write_ping = cx
                .shared
                .ping_state
                .lock(|s| *s == BufferState::PendingWrite);
            // NOTE: Re soundness: ditto
            if write_ping {
                debug!("Writting ping");
                cx.shared
                    .ping
                    .lock(|buf| fill_buffer(buf, cx.local.square_osc));
                cx.shared.ping_state.lock(|s| *s = BufferState::PendingRead);
            }
            Mono::delay(1000.micros()).await;

            let write_pong = cx
                .shared
                .pong_state
                .lock(|s| *s == BufferState::PendingWrite);
            // NOTE: Re soundness: ditto
            if write_pong {
                debug!("Writting pong");
                cx.shared
                    .pong
                    .lock(|buf| fill_buffer(buf, cx.local.square_osc));
                cx.shared.pong_state.lock(|s| *s = BufferState::PendingRead);
            }
            Mono::delay(1000.micros()).await;
        }
    }
}
