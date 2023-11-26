use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use anyhow::{Ok, Result};
use colors_transform::{Color, Rgb};
use drivers::Leds;
use esp_idf_hal::io::Write;
use esp_idf_hal::ledc::config::TimerConfig;
use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver};
use esp_idf_hal::prelude::*;
use esp_idf_hal::units::FromValueType;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::server::{
    EspHttpConnection, EspHttpServer, HandlerError, HandlerResult, Request, Response,
};
use esp_idf_svc::http::Method;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, Configuration, EspWifi};
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::{self as _};
use log::info;

use crate::rgb_led::WS2812RMT;
mod drivers;
mod rgb_led;

const SSID: &str = "lunas-christmas";

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let led_color_mutex = Mutex::new(Rgb::from(255.0, 0.5, 0.5));

    let peripherals = Peripherals::take().unwrap();

    let timer_driver = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::default().frequency(25_u32.kHz().into()),
    )?;
    let anode_rgb_drivers = (
        LedcDriver::new(
            peripherals.ledc.channel0,
            &timer_driver,
            peripherals.pins.gpio0,
        )?,
        LedcDriver::new(
            peripherals.ledc.channel1,
            &timer_driver,
            peripherals.pins.gpio1,
        )?,
        LedcDriver::new(
            peripherals.ledc.channel2,
            &timer_driver,
            peripherals.pins.gpio2,
        )?,
    );
    let cathode_rgb_drivers = (
        LedcDriver::new(
            peripherals.ledc.channel3,
            &timer_driver,
            peripherals.pins.gpio4,
        )?,
        LedcDriver::new(
            peripherals.ledc.channel4,
            &timer_driver,
            peripherals.pins.gpio5,
        )?,
        LedcDriver::new(
            peripherals.ledc.channel5,
            &timer_driver,
            peripherals.pins.gpio6,
        )?,
    );
    let ws_rmt = WS2812RMT::new(peripherals.pins.gpio8, peripherals.rmt.channel0)?;

    let mut leds = drivers::Leds{
        ws: ws_rmt,
        anode: drivers::AnodeLeds { red: anode_rgb_drivers.0, green: anode_rgb_drivers.1, blue: anode_rgb_drivers.2 },
        cathode: drivers::CathodeLeds { red: cathode_rgb_drivers.0, green: cathode_rgb_drivers.1, blue: cathode_rgb_drivers.2 },
    };

    let initial_color = {
        let led_color = led_color_mutex.lock().unwrap();
        rgb::RGB {
            r: led_color.get_red() as u8,
            g: led_color.get_green() as u8,
            b: led_color.get_blue() as u8,
        }
    };
    leds.set_color(initial_color)?;

    let leds_mutex = Mutex::new(leds);

    let sysloop = EspSystemEventLoop::take()?;

    let nvs = EspDefaultNvsPartition::take()?;

    // TODO: Clarify why it is ok to clone here
    let mut esp_wifi = EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?;

    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop)?;
    wifi.set_configuration(&Configuration::AccessPoint(
        esp_idf_svc::wifi::AccessPointConfiguration {
            ssid: SSID.into(),
            password: "".into(),
            ..Default::default()
        },
    ))?;
    wifi.start()?;
    wifi.wait_netif_up()?;

    let mut srv = EspHttpServer::new(&Default::default())?;
    srv.fn_handler("/", Method::Get, |req| {
        log::info!("Run GET handler");
        handle_get_request(req, &led_color_mutex)
    })?;
    srv.fn_handler("/", Method::Post, |req| {
        log::info!("Run POST handler");
        handle_post_request(req, &led_color_mutex, &leds_mutex)
    })?;

    loop {
        thread::sleep(Duration::from_secs(60));
    }

    Ok(())
}

fn handle_get_request(req: Request<&mut EspHttpConnection>, rgb_mutex: &Mutex<Rgb>) -> HandlerResult {
    // assume we come from get
    let resp = req.into_ok_response()?;
    log::info!("Ok Response");
    let rgb_value = rgb_mutex.lock()?.clone();
    log::info!("Value cloned");
    write_response(resp, rgb_value)
}

fn handle_post_request(
    mut req: Request<&mut EspHttpConnection>,
    rgb_mutex: &Mutex<Rgb>,
    leds_mutex: &Mutex<Leds>,
) -> HandlerResult {
    log::info!("Start processing post request");
    // assume we come from post
    let mut raw_data = [0; 1024];
    let read_bytes = req.connection().read(&mut raw_data)?;
    log::info!("Post data len is: {}", read_bytes);

    if read_bytes == raw_data.len() {
        raw_data[raw_data.len() - 1] = 0;
    }

    let data = std::str::from_utf8(&raw_data[0..read_bytes])?;
    log::info!("Post data is: {}", data);

    let color_value = url::form_urlencoded::parse(&data.as_bytes()).find_map(|(k, v)| {
        if k == "led_color" {
            Some(v)
        } else {
            None
        }
    });

    let c: Rgb;
    match color_value {
        Some(val) => {
            log::info!("Post color value is: {}", &val);
            c = Rgb::from_hex_str(&val)?;
            {
                let mut v = rgb_mutex.lock()?;
                *v = c;
            }

            let rgb = rgb::RGB {
                r: c.get_red() as u8,
                g: c.get_green() as u8,
                b: c.get_blue() as u8,
            };
            log::info!("Set RGB Pixel to: {}", rgb);
            {
                let mut leds = leds_mutex.lock()?;
                leds.set_color(rgb)?;
            }
        }
        None => {
            let v = rgb_mutex.lock()?;
            c = *v;
        }
    }

    let resp = req.into_ok_response()?;
    log::info!("Stop processing post request");
    write_response(resp, c)
}

fn write_response(mut resp: Response<&mut EspHttpConnection>, rgb: Rgb) -> HandlerResult {
    let c = rgb.to_css_hex_string();
    resp.write_fmt(format_args!(
        r#"
<!DOCTYPE html>
<html>
    <body>
        <form method="post">
            <input name="led_color" id="led_color" type="color" value="{c}">
            <input type="submit">
        </form>
    </body>
</html>
"#
    ))
    .map_err(|err| HandlerError::from(err))
}
