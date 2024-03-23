use egm96::grid::load_embedded_egm96_grid15;
use egm96::Geoid;

fn main() {
    // Load the embedded EGM96 model.
    let geoid = load_embedded_egm96_grid15();

    // Calculate the geoid height.
    let (lng, lat) = (138.2839817085188, 37.12378643088312);
    let height = geoid.get_height(lng, lat);
    println!("Input: (lng: {lng}, lat: {lat}) -> Geoid height: {height}");

    // Returns NaN if the input is outside the domain.
    assert!(f64::is_nan(geoid.get_height(10.0, 10.0)))
}
