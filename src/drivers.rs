use anyhow::{Result, anyhow};
use esp_idf_hal::ledc::LedcDriver;

use crate::rgb_led::WS2812RMT;

pub struct AnodeLeds<'a> {
    pub red: LedcDriver<'a>,
    pub green: LedcDriver<'a>,
    pub blue: LedcDriver<'a>,
}

pub struct CathodeLeds<'a> {
    pub red: LedcDriver<'a>,
    pub green: LedcDriver<'a>,
    pub blue: LedcDriver<'a>,
}

pub struct Leds<'a> {
    pub ws: WS2812RMT<'a>,
    pub anode: AnodeLeds<'a>,
    pub cathode: CathodeLeds<'a>,
}

impl Leds<'_> {
    pub fn set_color(self: &mut Self, rgb: rgb::RGB<u8>) -> Result<()> {

        //log::info!("Max duty is: {}", red.get_max_duty());
        //let red_duty = (c.get_red() / 255.0 * red.get_max_duty() as f32).round() as u32;

        self.ws.set_pixel(rgb)?;

        //Handle anode
        self.anode.red.enable()?;
        self.anode.green.enable()?;
        self.anode.blue.enable()?;
        let red_max_duty = self.anode.red.get_max_duty();
        // We drive Anode here, so invert rgb value
        let red_duty = ((1.0 - rgb.r as f32) * red_max_duty as f32) as u32;
        self.anode.red.set_duty(red_duty).map_err(|err| anyhow!(err))?;
        let green_max_duty = self.anode.green.get_max_duty();
        // We drive Anode here, so invert rgb value
        let green_duty = ((1.0 - rgb.g as f32) * green_max_duty as f32) as u32;
        self.anode.green.set_duty(green_duty).map_err(|err| anyhow!(err))?;
        let blue_max_duty = self.anode.blue.get_max_duty();
        // We drive Anode here, so invert rgb value
        let blue_duty = ((1.0 - rgb.b as f32) * blue_max_duty as f32) as u32;
        self.anode.blue.set_duty(blue_duty).map_err(|err| anyhow!(err))?;

        //Handle cathode
        self.cathode.red.enable()?;
        self.cathode.green.enable()?;
        self.cathode.blue.enable()?;
        let red_max_duty = self.cathode.red.get_max_duty();
        let red_duty = (rgb.r as f32 * red_max_duty as f32) as u32;
        self.cathode.red.set_duty(red_duty).map_err(|err| anyhow!(err))?;
        let green_max_duty = self.cathode.green.get_max_duty();
        let green_duty = (rgb.g as f32 * green_max_duty as f32) as u32;
        self.cathode.green.set_duty(green_duty).map_err(|err| anyhow!(err))?;
        let blue_max_duty = self.cathode.blue.get_max_duty();
        let blue_duty = (rgb.b as f32 * blue_max_duty as f32) as u32;
        self.cathode.blue.set_duty(blue_duty).map_err(|err| anyhow!(err))?;

        log::info!("Set duties: {:?}", (red_duty, green_duty, blue_duty));
        Ok(())
    }
}
