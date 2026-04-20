#![no_main]
#![no_std]

use core::mem::MaybeUninit;

use embassy_stm32::{bind_interrupts, peripherals};
use embedded as _; // global logger + panicking-behavior + memory layout

rtic_monotonics::systick_monotonic!(Mono, 1_000);

#[unsafe(link_section = ".axisram")]
static SHARED_DATA: MaybeUninit<embassy_stm32::SharedData> = MaybeUninit::uninit();

type AudioSample = i16;
const BUFFER_SAMPLES: usize = 256;
type SampleBuffer = [AudioSample; BUFFER_SAMPLES];
const SAMPLE_RATE: f64 = 44100.0;
const MIDDLE_C: f64 = 261.6256;

const USB_EP_OUT_BUF_SIZE: usize = 256;
const USB_DESCRIPTOR_BUF_SIZE: usize = 256;
const USB_CONTROL_BUF_SIZE: usize = 64;
const USB_VID: u16 = 0xDEAD;
const USB_PID: u16 = 0xBEEF;

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
    OTG_FS         => embassy_stm32::usb::InterruptHandler<peripherals::USB_OTG_FS>;
});

#[rtic::app(
    device = embassy_stm32,
    dispatchers = [DFSDM1_FLT0, DFSDM1_FLT1, DFSDM1_FLT2, DFSDM1_FLT3],
    peripherals = true
)]
mod app {
    use dasp::{
        Signal,
        signal::{ConstHz, Square},
    };
    use defmt::{debug, info};
    use embassy_stm32::{
        self as hal,
        peripherals::USB_OTG_FS,
        rcc,
        usb::{self, Driver},
    };
    use embassy_usb::{
        Builder, Config, UsbDevice,
        class::cdc_acm::{CdcAcmClass, Sender, State},
    };

    use super::*;
    use dsp::square_oscillator;

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
        square_osc: Square<ConstHz>,
        usb_device: UsbDevice<'static, Driver<'static, USB_OTG_FS>>,
        cdc_sender: Sender<'static, Driver<'static, USB_OTG_FS>>,
    }

    #[init(local = [
        ep_out_buffer: [u8; USB_EP_OUT_BUF_SIZE] = [0u8; USB_EP_OUT_BUF_SIZE],
        usb_config_descriptor_buf: [u8; USB_DESCRIPTOR_BUF_SIZE] = [0u8; USB_DESCRIPTOR_BUF_SIZE],
        usb_bos_descriptor_buf: [u8; USB_DESCRIPTOR_BUF_SIZE] = [0u8; USB_DESCRIPTOR_BUF_SIZE],
        usb_msos_descriptor_buf: [u8; USB_DESCRIPTOR_BUF_SIZE] = [0u8; USB_DESCRIPTOR_BUF_SIZE],
        usb_control_buf: [u8; USB_CONTROL_BUF_SIZE] = [0u8; USB_CONTROL_BUF_SIZE],
        cdc_state: State<'static> = State::new() // Im not sure why rtic isnt automatically making
                                                 // this 'static, but this seems to fix it. Without
                                                 // the lifetime annotation it complains
    ])]
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

        // USB configuration
        // USB has to be clocked to 48MHz so simply use HSI48
        config.rcc.mux.usbsel = rcc::mux::Usbsel::HSI48;
        // This is required when using HSI48
        config.rcc.hsi48 = Some(rcc::Hsi48Config {
            sync_from_usb: true,
        });

        debug!("Initializing HAL...");
        let p = hal::init_primary(config, &SHARED_DATA);
        debug!("HAL Initialized");

        let usb_peripheral_config = usb::Config::default();
        // This may at some point need vbus_detection set to true
        // https://docs.embassy.dev/embassy-stm32/0.6.0/stm32h755zi-cm7/usb/struct.Config.html#structfield.vbus_detection

        let driver = usb::Driver::new_fs(
            p.USB_OTG_FS,
            Irqs,
            p.PA12,
            p.PA11,
            cx.local.ep_out_buffer,
            usb_peripheral_config,
        );

        let mut usb_config = Config::new(USB_VID, USB_PID);
        usb_config.manufacturer = Some("Greg Shiner");
        usb_config.product = Some("TechnicallyASynth");
        usb_config.max_power = 0;

        let mut builder = Builder::new(
            driver,
            usb_config,
            cx.local.usb_config_descriptor_buf,
            cx.local.usb_bos_descriptor_buf,
            cx.local.usb_msos_descriptor_buf,
            cx.local.usb_control_buf,
        );

        // It may be cool to one day use uac1 instead of cdc to support USB audio
        // As of 4/20/26 embassy-usb only supports host -> device uac
        let cdc = CdcAcmClass::new(&mut builder, cx.local.cdc_state, 64);
        let usb_device = builder.build();
        let (cdc_sender, _cdc_receiver) = cdc.split();

        let mono_driver = Mono::start(cp.SYST, 480_000_000); // 480 MHz System Clock
        debug!("Monotonic Started");
        fill_audio::spawn().unwrap();

        (
            Shared {
                ping: [0; BUFFER_SAMPLES],
                pong: [0; BUFFER_SAMPLES],
                ping_state: BufferState::PendingWrite,
                pong_state: BufferState::PendingWrite,
            },
            Local {
                square_osc: square_oscillator(SAMPLE_RATE / 2.0, SAMPLE_RATE),
                usb_device,
                cdc_sender,
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

    fn fill_buffer(buf: &mut SampleBuffer, osc: &mut Square<ConstHz>) {
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
            let write_ping = cx
                .shared
                .ping_state
                .lock(|s| *s == BufferState::PendingWrite);
            if write_ping {
                debug!("Writting ping");
                cx.shared
                    .ping
                    .lock(|buf| fill_buffer(buf, cx.local.square_osc));
                cx.shared.ping_state.lock(|s| *s = BufferState::PendingRead);
            }
            core::future::ready(()).await;

            let write_pong = cx
                .shared
                .pong_state
                .lock(|s| *s == BufferState::PendingWrite);
            if write_pong {
                debug!("Writting pong");
                cx.shared
                    .pong
                    .lock(|buf| fill_buffer(buf, cx.local.square_osc));
                cx.shared.pong_state.lock(|s| *s = BufferState::PendingRead);
            }
            core::future::ready(()).await;
        }
    }
}
