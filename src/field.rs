pub struct Field {
    pub color: u32,
    x: usize,
    y: usize,
    height: usize,
    width: usize,
}

impl Field {
    pub fn coords(&self) -> Box<Iterator<Item=(usize, usize)>> {
        let y = self.y;
        let max_y = self.y + self.height;

        let x = self.x;
        let max_x = self.x + self.width;

        box() (y..max_y).flat_map(move |y| (x..max_x).map(move |x| (x, y)))
    }
}

impl ::std::str::FromStr for Field {
    type Err = ::error::CliError;

    fn from_str(s: &str) -> Result<Field, Self::Err> {
        let data: Vec<&str> = s.split(' ').collect();
        match &data[..] {
            [ref color, ref x, ref y, ref height, ref width] => Ok(Field {
                color: try!(color.parse()),
                x: try!(x.parse()),
                y: try!(y.parse()),
                height: try!(height.parse()),
                width: try!(width.parse()),
            }),
            _ => Err(::error::CliError::Map),
        }
    }
}
