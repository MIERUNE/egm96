use crate::Geoid;
use std::borrow::Cow;
use std::io::{self, BufRead, Read, Write};

/// Gridded geoid model
pub trait Grid {
    fn grid_info(&self) -> &GridInfo;
    fn lookup_grid_points(&self, ix: u32, iy: u32) -> f64;

    #[inline]
    fn get_interpolated_value(&self, x: f64, y: f64) -> f64 {
        use std::f64::NAN;
        let grid = self.grid_info();
        let grid_x = (x - grid.x_min as f64) * (grid.x_denom as f64);
        let grid_y = (y - grid.y_min as f64) * (grid.y_denom as f64);
        if grid_x < 0.0 || grid_y < 0.0 {
            return NAN;
        }

        let ix = grid_x.floor() as u32;
        let iy = grid_y.floor() as u32;
        let x_residual = grid_x - ix as f64;
        let y_residual = grid_y - iy as f64;

        if ix >= grid.x_num || iy >= grid.y_num {
            NAN
        } else {
            let lookup_or_nan = |x, y, cond: bool| {
                if cond {
                    self.lookup_grid_points(x, y)
                } else {
                    NAN
                }
            };

            bilinear(
                x_residual,
                y_residual,
                self.lookup_grid_points(ix, iy),
                lookup_or_nan(ix + 1, iy, ix < grid.x_num - 1),
                lookup_or_nan(ix, iy + 1, iy < grid.y_num - 1),
                lookup_or_nan(ix + 1, iy + 1, ix < grid.x_num - 1 && iy < grid.y_num - 1),
            )
        }
    }
}

/// Bilinear interpolation
fn bilinear(x: f64, y: f64, v00: f64, v01: f64, v10: f64, v11: f64) -> f64 {
    if x == 0.0 && y == 0.0 {
        v00
    } else if x == 0.0 {
        v00 * (1.0 - y) + v10 * y
    } else if y == 0.0 {
        v00 * (1.0 - x) + v01 * x
    } else {
        v00 * (1.0 - x) * (1.0 - y) + v01 * x * (1.0 - y) + v10 * (1.0 - x) * y + v11 * x * y
    }
}

/// Grid parameters
pub struct GridInfo {
    /// Number of grid points along X-axis
    x_num: u32,
    /// Number of grid points along Y-axis
    y_num: u32,
    /// Denominator of grid interval along X-axis
    x_denom: u32,
    /// Denominator of grid interval along Y-axis
    y_denom: u32,
    /// Minimum value of X-axis
    x_min: f32,
    /// Minimum value of Y-axis
    y_min: f32,
}

/// In-memory gridded geoid model
pub struct MemoryGrid<'a> {
    pub grid_info: GridInfo,
    points: Cow<'a, [i32]>,
}

impl<'a> Grid for MemoryGrid<'a> {
    /// Gets grid parameters
    fn grid_info(&self) -> &GridInfo {
        &self.grid_info
    }

    /// Gets the value of the grid point at (ix, iy)
    #[inline]
    fn lookup_grid_points(&self, ix: u32, iy: u32) -> f64 {
        self.points[(self.grid_info.x_num * iy + ix) as usize] as f64 / 1000.0
    }
}

impl<'a> Geoid for MemoryGrid<'a> {
    /// Gets the height of the geoid at (lng, lat)
    #[inline]
    fn get_height(&self, lng: f64, lat: f64) -> f64 {
        if !(-90. ..=90.).contains(&lat) {
            return std::f64::NAN;
        }
        let normalized_lng = ((lng % 360.) + 360.) % 360.;
        self.get_interpolated_value(normalized_lng, lat)
    }
}

impl<'a> MemoryGrid<'a> {
    /// Loads the geoid model from a binary file.
    pub fn from_binary_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        // Read header
        let mut buf = [0; 16];
        reader.read_exact(&mut buf)?;
        let grid_info = GridInfo {
            x_num: u16::from_le_bytes(buf[0..2].try_into().unwrap()) as u32,
            y_num: u16::from_le_bytes(buf[2..4].try_into().unwrap()) as u32,
            x_denom: u16::from_le_bytes(buf[4..6].try_into().unwrap()) as u32,
            y_denom: u16::from_le_bytes(buf[6..8].try_into().unwrap()) as u32,
            x_min: f32::from_le_bytes(buf[8..12].try_into().unwrap()),
            y_min: f32::from_le_bytes(buf[12..16].try_into().unwrap()),
        };

        // Read grid point values
        let mut points = Vec::with_capacity((grid_info.x_num * grid_info.y_num) as usize);
        let mut buf = [0; 4];
        let mut prev_x1y1 = 13606;
        let mut prev_x1 = 13606;
        for pos in 0..(grid_info.y_num * grid_info.x_num) as usize {
            // linear prediction
            let prev_y1 = match pos {
                _ if pos < grid_info.x_num as usize => 0,
                _ => points[pos - grid_info.x_num as usize],
            };
            reader.read_exact(&mut buf)?;
            let predicted = prev_x1 + prev_y1 - prev_x1y1;
            let curr = predicted + i32::from_le_bytes(buf);
            points.push(curr);
            (prev_x1, prev_x1y1) = (curr, prev_y1);
        }

        Ok(MemoryGrid {
            grid_info,
            points: points.into(),
        })
    }

    /// Dumps the geoid model to a binary file.
    pub fn to_binary_writer<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Write header
        writer.write_all(&(self.grid_info.x_num as u16).to_le_bytes())?;
        writer.write_all(&(self.grid_info.y_num as u16).to_le_bytes())?;
        writer.write_all(&(self.grid_info.x_denom as u16).to_le_bytes())?;
        writer.write_all(&(self.grid_info.y_denom as u16).to_le_bytes())?;
        writer.write_all(&self.grid_info.x_min.to_le_bytes())?;
        writer.write_all(&self.grid_info.y_min.to_le_bytes())?;

        // Write grid point values
        let mut prev_x1y1 = 13606;
        let mut prev_x1 = 13606;
        for pos in 0..(self.grid_info.y_num * self.grid_info.x_num) as usize {
            // linear prediction
            let curr = self.points[pos];
            let prev_y1 = match pos {
                _ if pos < self.grid_info.x_num as usize => 0,
                _ => self.points[pos - self.grid_info.x_num as usize],
            };
            let predicted = prev_x1 + prev_y1 - prev_x1y1;
            let d = curr - predicted;
            writer.write_all(&d.to_le_bytes())?;
            (prev_x1, prev_x1y1) = (curr, prev_y1);
        }
        Ok(())
    }

    /// Loads the EGM96 original grid ('15) model in ASCII format.
    pub fn from_ascii_reader<R: BufRead>(reader: &mut R) -> io::Result<Self> {
        use io::{Error, ErrorKind::InvalidData};
        let mut reader = io::BufReader::new(reader);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        // NOTE: Currently, we only supports the EGM96 '15 grid model.
        let c: Vec<&str> = line.split_whitespace().collect();
        if c.len() != 6 {
            return Err(Error::new(InvalidData, "header line must have 6 values"));
        }
        if c[0] != "-90.000000" {
            return Err(Error::new(InvalidData, "min lat must be -90.000000"));
        }
        if c[1] != "90.000000" {
            return Err(Error::new(InvalidData, "max lat must be -90.000000"));
        }
        if c[2] != ".000000" {
            return Err(Error::new(InvalidData, "min lng must be 0.000000"));
        }
        if c[3] != "360.000000" {
            return Err(Error::new(InvalidData, "max lng must be 360.000000"));
        }
        if c[4] != ".250000" {
            return Err(Error::new(InvalidData, "lat interval must be 0.250000"));
        }
        if c[5] != ".250000" {
            return Err(Error::new(InvalidData, "lng interval must be 0.250000"));
        }

        let grid_info = GridInfo {
            x_num: 360 * 4 + 1, // 1441
            y_num: 180 * 4 + 1, // 721
            x_denom: 4,
            y_denom: 4,
            x_min: 0.0,
            y_min: -90.0,
        };

        let mut points = Vec::with_capacity((grid_info.x_num * grid_info.y_num) as usize);
        for line_or_err in reader.lines() {
            match line_or_err {
                Ok(line) => {
                    for s in line.split_ascii_whitespace() {
                        let s = s.replace('.', "");
                        let Ok(n) = s.parse::<i32>() else {
                            return Err(Error::new(InvalidData, "Invalid data"));
                        };
                        points.push(n);
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        // Check the number of data points
        let expected_points = grid_info.x_num as usize * grid_info.y_num as usize;
        if points.len() != expected_points {
            return Err(Error::new(
                InvalidData,
                format!(
                    "{} data points required but found {}",
                    expected_points,
                    points.len()
                ),
            ));
        }

        // Swap y-axis: [90 -> -90] to [-90 -> 90]
        for y in 0..grid_info.y_num as usize / 2 {
            let y1 = y;
            let y2 = grid_info.y_num as usize - 1 - y;
            for x in 0..grid_info.x_num as usize {
                points.swap(
                    y1 * grid_info.x_num as usize + x,
                    y2 * grid_info.x_num as usize + x,
                );
            }
        }

        Ok(MemoryGrid {
            grid_info,
            points: points.into(),
        })
    }
}

/// Loads the embedded EGM96 grid ('15) model.
///
/// ```
/// use egm96::grid::load_embedded_egm96_grid15;
/// use egm96::Geoid;
///
/// let geoid = load_embedded_egm96_grid15();
/// let height = geoid.get_height(138.2839817085188, 37.12378643088312);
/// assert!((height - 39.27814).abs() < 1e-3)
/// ```
pub fn load_embedded_egm96_grid15() -> MemoryGrid<'static> {
    const EMBEDDED_MODEL: &[u8] = include_bytes!("egm96_grid15.bin.lz4");
    MemoryGrid::from_binary_reader(&mut std::io::Cursor::new(
        lz4_flex::decompress_size_prepended(EMBEDDED_MODEL).unwrap(),
    ))
    .unwrap()
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{BufReader, Cursor};

    use super::*;

    #[test]
    fn embedded() {
        let geoid = load_embedded_egm96_grid15();
        let info = geoid.grid_info();
        assert_eq!(info.x_num, 360 * 4 + 1);
        assert_eq!(info.y_num, 180 * 4 + 1);
        assert_eq!(info.x_denom, 4);
        assert_eq!(info.y_denom, 4);
        assert_eq!(info.x_min, 0.0);
        assert_eq!(info.y_min, -90.0);

        let height = geoid.get_height(138.2839817085188, 37.12378643088312);
        assert!((height - 39.27814).abs() < 1e-3);

        let height = geoid.get_height(0., 0.);
        assert!((height - 17.16157).abs() < 1e-3);

        let height = geoid.get_height(66.66, 55.55);
        assert!((height - -21.80307).abs() < 1e-3);

        let height = geoid.get_height(360.0, 90.0);
        assert!((height - 13.606).abs() < 1e-10);

        let height = geoid.get_height(360.0, -90.0);
        assert!((height - -29.534).abs() < 1e-7);

        let height = geoid.get_height(0.0, -91.0);
        assert!(height.is_nan());

        let height = geoid.get_height(0.0, 91.0);
        assert!(height.is_nan());

        assert_eq!(
            geoid.get_height(1.0, 50.0),
            geoid.get_height(1.0 + 360.0, 50.0),
        );
        assert_eq!(
            geoid.get_height(-1.0, 50.0),
            geoid.get_height(-1.0 + 360.0, 50.0),
        );
        assert_eq!(
            geoid.get_height(-721.0, 50.0),
            geoid.get_height(-1.0 + 360.0, 50.0),
        );
    }

    #[test]
    fn ascii_to_binary() {
        // from ascii
        let mut reader = BufReader::new(File::open("./tests/ww15mgh.grd").unwrap());
        let geoid = MemoryGrid::from_ascii_reader(&mut reader).unwrap();

        // to binary
        let mut buffer = Vec::new();
        geoid.to_binary_writer(&mut buffer).unwrap();

        // from binary
        let geoid = MemoryGrid::from_binary_reader(&mut Cursor::new(buffer)).unwrap();

        // to binary (broken data)
        let mut buffer = Vec::new();
        geoid.to_binary_writer(&mut buffer).unwrap();
    }
}
