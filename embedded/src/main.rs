#![no_main]
#![no_std]

use core::mem::MaybeUninit;

use embassy_stm32::{bind_interrupts, peripherals};
use embedded as _; // global logger + panicking-behavior + memory layout

rtic_monotonics::systick_monotonic!(Mono, 1_000);

type AudioSample = i32;
type I2sSample = u32;
const BUFFER_SAMPLES: usize = 1024;
type SampleBuffer = [AudioSample; BUFFER_SAMPLES];
const SAMPLE_RATE_U: u32 = 48000;
const SAMPLE_RATE: f64 = SAMPLE_RATE_U as f64;
const MIDDLE_C: f64 = 261.6256;

#[unsafe(link_section = ".axisram")]
static SHARED_DATA: MaybeUninit<embassy_stm32::SharedData> = MaybeUninit::uninit();
#[unsafe(link_section = ".axisram")]
static mut DMA_BUFFER: [I2sSample; BUFFER_SAMPLES] = [0; BUFFER_SAMPLES];

bind_interrupts!(struct Irqs {
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
    struct Shared {}

    // Local resources go here
    #[local]
    struct Local {
        square_osc: Sine<ConstHz>,
        i2s2: i2s::I2S<'static, u32>,
    }

    #[init()]
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
        let dma_buf: &'static mut [I2sSample; BUFFER_SAMPLES] =
            unsafe { &mut *core::ptr::addr_of_mut!(DMA_BUFFER) };
        let i2s2 = i2s::I2S::new_txonly_nomck(
            spi2, i2s2_sdo, i2s2_ws, i2s2_ck, dma1_ch0, dma_buf, Irqs, i2s_config,
        );

        let mono_driver = Mono::start(cp.SYST, 480_000_000); // 480 MHz System Clock
        debug!("Monotonic Started");
        write_osc_to_i2s::spawn().unwrap();
        (
            Shared {},
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

    fn fill_buffer(buf: &mut SampleBuffer, osc: &mut Sine<ConstHz>) {
        fn f64_to_sample(s: f64) -> AudioSample {
            (s * AudioSample::MAX as f64).clamp(AudioSample::MIN as f64, AudioSample::MAX as f64)
                as AudioSample
        }
        buf.iter_mut().for_each(|s| *s = f64_to_sample(osc.next()));
    }

    #[task(local = [square_osc, i2s2], priority = 1)]
    async fn write_osc_to_i2s(mut cx: write_osc_to_i2s::Context) -> ! {
        info!("Starting write osc to i2s");
        let mut counter = 0u32;
        cx.local.i2s2.start();
        debug!("i2s2 Started");
        loop {
            let mut buf = [0; BUFFER_SAMPLES];
            fill_buffer(&mut buf, cx.local.square_osc);
            let to_send: &[u32] =
                unsafe { core::slice::from_raw_parts(buf.as_ptr() as *const u32, buf.len()) };
            debug!("Filled buffer, {}", counter);
            cx.local.i2s2.write(to_send);
            debug!("Sent buffer, {}", counter);
            counter += 1;
        }
    }
}
