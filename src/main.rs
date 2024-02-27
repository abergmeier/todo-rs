use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use anyhow::{Ok, Result};
use colors_transform::{Color, Rgb};
use drivers::Leds;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::text::TextStyleBuilder;
use epd_waveshare::buffer_len;
use epd_waveshare::epd7in5_v2::{self, Display7in5, Epd7in5};
use epd_waveshare::graphics::Display;
use epd_waveshare::prelude::WaveshareDisplay;
use esp_idf_hal::delay::{Delay, Ets};
use esp_idf_hal::gpio::{AnyIOPin, AnyInputPin, AnyOutputPin};
use esp_idf_hal::io::Write;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi::config::{Config, DriverConfig};
use esp_idf_hal::spi::SpiDeviceDriver;
use esp_idf_hal::units::FromValueType;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::http::server::{EspHttpConnection, EspHttpServer, Request, Response};
use esp_idf_svc::http::Method;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, Configuration, EspWifi};
use heapless;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::{self as _};

use embedded_graphics::{prelude::*, text::Text};

mod drivers;
//mod keep;
mod rgb_led;

const SSID: &str = "todos";

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    log::info!("Setting up Display");
    let mut display = Box::new(Display7in5::default());
    let pins = peripherals.pins;
    let spi = peripherals.spi2;
    //let mut delay = Delay::new_default();
    let mut delay = Ets {};
    let sdo: AnyOutputPin = pins.gpio14.into();
    let busy_in: AnyInputPin = pins.gpio1.into();
    let rst: AnyOutputPin = pins.gpio2.into();
    let dc: AnyOutputPin = pins.gpio3.into();
    let cs: AnyOutputPin = pins.gpio4.into();
    let sclk: AnyOutputPin = pins.gpio5.into();
    
    log::info!("Setting up SPI driver");
    let mut spi_driver = SpiDeviceDriver::new_single(
        spi,
        sclk,
        sdo,
        Option::<AnyIOPin>::None,
        cs.into(),
        &DriverConfig::new(),
        &Config::new()
            .baudrate(40.MHz().into())
            .duplex(esp_idf_hal::spi::config::Duplex::Full),
    )?;
    /*
    let mut delay = Delay::new_default();
    println!("epd");
    let mut epd = Epd7in5::new(
        &mut spi_driver,
        PinDriver::input(busy_in)?,
        PinDriver::output(dc)?,
        PinDriver::output(rst)?,
        &mut delay,
        None,
    )?;

    let style = MonoTextStyle::new(&FONT_6X10, epd_waveshare::color::Color::White);

    let sysloop = EspSystemEventLoop::take()?;

    let nvs = EspDefaultNvsPartition::take()?;

    println!("wifi");
    // TODO: Clarify why it is ok to clone here
    let mut esp_wifi = EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?;

    let mut apc = esp_idf_svc::wifi::AccessPointConfiguration::default();
    apc.ssid = heapless::String::try_from(SSID).unwrap();
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop)?;
    wifi.set_configuration(&Configuration::AccessPoint(apc))?;
    wifi.start()?;
    wifi.wait_netif_up()?;

    let mut srv = EspHttpServer::new(&Default::default())?;

    epd.clear_frame(&mut spi_driver, &mut delay)?;
    epd.display_frame(&mut spi_driver, &mut delay)?;

    // Create a text at position (20, 30) and draw it using the previously defined style
    let text = Text::new("Hello Rust!", Point::new(20, 30), style);
    println!("Drawme");
    text.draw(&mut display)?;
    epd.update_and_display_frame(&mut spi_driver, display.buffer(), &mut delay)?;

    delay.delay_ms(5000);

    epd.sleep(&mut spi_driver, &mut delay)?;
*/
    Ok(())
}
