use wasm_bindgen::prelude::*;

use crate::grid;
use crate::grid::MemoryGrid;
use crate::Geoid;

#[wasm_bindgen]
pub struct Egm96GeoidGrid {
    geoid: MemoryGrid<'static>,
}

#[wasm_bindgen]
impl Egm96GeoidGrid {
    #[wasm_bindgen(js_name = "getHeight")]
    pub fn get_height(&self, lng: f64, lat: f64) -> f64 {
        self.geoid.get_height(lng, lat)
    }

    #[wasm_bindgen(js_name = "getHeights")]
    pub fn get_heights(&self, lngs: &[f64], lats: &[f64]) -> Vec<f64> {
        lngs.iter()
            .zip(lats.iter())
            .map(|(lng, lat)| self.geoid.get_height(*lng, *lat))
            .collect()
    }
}

#[wasm_bindgen(js_name = "loadEmbeddedEGM96")]
pub fn load_embedded_egm96_grid15() -> Egm96GeoidGrid {
    Egm96GeoidGrid {
        geoid: grid::load_embedded_egm96_grid15(),
    }
}
