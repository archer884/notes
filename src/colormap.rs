use field::Field;

pub struct ColorMap {
    width: usize,
    height: usize,
    cells: Vec<Vec<u8>>
}

impl ColorMap {
    pub fn create(height: usize, width: usize) -> ColorMap {
        ColorMap {
            width: width,
            height: height,
            cells: vec![ vec![0; height]; width ],
        }
    }

    pub fn add_field(&mut self, field: &Field) {
        let max_y = self.height;
        let max_x = self.width;
        for (x, y) in field.coords().filter(|&(x, y)| x < max_x && y < max_y) {
            self.cells[x][y] = field.color;
        }
    }

    pub fn colors<'a>(&'a self) -> Box<Iterator<Item=&u8> + 'a> {
        box self.cells.iter().flat_map(|row| row.iter())
    }
}
