use std::sync::Mutex;

use anyhow::{Ok, Result};
use colors_transform::{Color, Rgb};
use esp_idf_hal::io::Write;
use esp_idf_hal::ledc::config::TimerConfig;
use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver};
use esp_idf_hal::prelude::Peripherals;
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
mod rgb_led;

const SSID: &str = "lunas-christmas";

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let led_color_mutex = Mutex::new(Rgb::from(0.5, 0.5, 0.5));

    let peripherals = Peripherals::take().unwrap();

    let timer_driver = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::default().frequency(25_u32.kHz().into()),
    )?;
    let red = LedcDriver::new(
        peripherals.ledc.channel0,
        &timer_driver,
        peripherals.pins.gpio9,
    )?;
    let ws_rmt = {
        let mut ws = WS2812RMT::new(peripherals.pins.gpio8, peripherals.rmt.channel0)?;
        let led_color = led_color_mutex.lock().unwrap();
        ws.set_pixel(rgb::RGB {
            r: led_color.get_red() as u8,
            g: led_color.get_green() as u8,
            b: led_color.get_blue() as u8,
        })?;
        ws
    };
    let ws_rmt_mutex = Mutex::new(ws_rmt);

    log::info!("Max duty is: {}", red.get_max_duty());
    let m = Mutex::new(red);

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
    core::mem::forget(wifi);
    core::mem::forget(esp_wifi);

    let mut srv = EspHttpServer::new(&Default::default())?;
    srv.fn_handler("/", Method::Get, |req| {
        log::info!("Run GET handler");
        handle_get_request(req, &led_color_mutex)
    })?;
    srv.fn_handler("/", Method::Post, |req| {
        log::info!("Run POST handler");
        let mut red = m.lock()?;
        handle_post_request(req, &led_color_mutex, &ws_rmt_mutex, &mut red)
    })?;
    core::mem::forget(srv);

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
    ws_rmt: &Mutex<WS2812RMT>,
    red: &mut LedcDriver,
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
                let mut ws = ws_rmt.lock()?;
                ws.set_pixel(rgb)?;
            }
            let red_duty = (c.get_red() / 255.0 * red.get_max_duty() as f32).round() as u32;
            log::info!("Set red duty to: {}", red_duty);
            //red.set_duty(red_duty)?;
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
