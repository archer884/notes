use field::Field;

pub struct ColorMap {
    cells: Vec<Vec<u32>>
}

impl ColorMap {
    pub fn create(height: usize, width: usize) -> ColorMap {
        ColorMap {
            cells: vec![
                vec![0; height];
                width
            ],
        }
    }

    pub fn add_field(&mut self, field: &Field) {
        for (x, y) in field.coords() {
            self.cells[x][y] = field.color;
        }
    }

    pub fn colors<'a>(&'a self) -> Box<Iterator<Item=&u32> + 'a> {
        box self.cells.iter().flat_map(|row| row.iter())
    }
}
