#![feature(box_syntax, slice_patterns)]

mod colormap;
mod error;
mod field;

use colormap::ColorMap;
use error::CliError;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{ BufRead, BufReader };
use std::path::Path;

fn main() {
    let output = std::env::args().nth(1).ok_or(CliError::Args)
        .and_then(load_file)
        .and_then(build_map)
        .map(score_map);

    match output {
        Err(e) => println!("{:?}", e),
        Ok(values) => {
            for (color, count) in values {
                println!("{}: {}", color, count);
            }
        }
    }
}

fn score_map(map: ColorMap) -> Vec<(u32, u32)> {
    map.colors().fold(BTreeMap::new(), |mut map, color| { 
        *map.entry(color).or_insert(0) += 1; 
        map
    }).into_iter().map(|(&a, b)| (a, b)).collect()
    
}

fn build_map<R: BufRead>(reader: R) -> Result<ColorMap, CliError> {
    let mut lines = reader.lines();

    // get width and height for map
    let (map_x, map_y) = try!(lines.next()
                              .unwrap()
                              .map_err(|e| CliError::Io(e))
                              .and_then(read_map_size));

    Ok(lines
       .filter_map(|input| input.ok())
       .filter_map(|input| input.parse().ok())
       .fold(ColorMap::create(map_x, map_y), |mut map, field| {
           map.add_field(&field);
           map
       }))
}

fn read_map_size(input: String) -> Result<(usize, usize), CliError> {
    let data: Vec<&str> = input.split(' ').collect();
    match &data[..] {
        [ref x, ref y] => Ok((try!(x.parse()), try!(y.parse()))),
        _ => Err(CliError::Map),
    }
}

fn load_file(path: String) -> Result<BufReader<File>, CliError> {
    Ok(BufReader::new(try!(File::open(&Path::new(&path)))))
}
