// Format data is based on the structure/explaination found at: http://paulbourke.net/dataformats/ppm/

#![deny(clippy::all)]
#![forbid(unsafe_code)]

extern crate args;

use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
use std::fs::File;
use std::io::{prelude::*, BufReader, SeekFrom};
use std::env;

/// Representation of the application state
#[derive(Clone)]
struct World {
    frame: Option<PPM>,
    single_draw: bool,
    has_been_drawn: bool,
}

impl World {
    fn get_width(self) -> i32 {
        self.frame.unwrap().header.width
    }

    fn get_height(self) -> i32 {
        self.frame.unwrap().header.height
    }
}

#[derive(Debug, Clone)]
struct PPMHeader {
  ppm_type: PpmType,
  width: i32,
  height: i32,
  max_value: i32
}

#[derive(Debug, Clone)]
struct PPM {
    header: PPMHeader,
    values: Vec<PpmValue>,
}

#[derive(Debug, Clone)]
struct PpmValue {
    r: i32,
    g: i32,
    b: i32
}

impl PpmValue {
    fn new(red: i32, green: i32, blue: i32) -> Self {
        PpmValue {
            r: red,
            g: green,
            b: blue
        }
    } 
}

#[derive(Debug, Clone, PartialEq)]
/// Determines the format type of file based on the the first two bytes
/// of the Magic Number
enum PpmType {
    /// P1 is the Bitmap Data in ASCII
    P1,
    /// P2 is the Grayscale Data in ASCII
    P2,
    /// P3 is the RGB Image data in ASCII
    P3,
    /// P4 is the Bitmap Data in Binary Format
    P4,
    /// P5 is the Grayscale Data in Binary Format
    P5,
    /// P6 is the RGB Image Data in Binary Format
    P6,
    /// This is not a valid PPM/PGM/PBM File Format 
    P0,
}

impl PPMHeader {
  fn new() -> Self {
    PPMHeader {
      ppm_type: PpmType::P0,
      width: 0,
      height: 0,
      max_value: 0,
    }
  }
}

impl PPM {
    fn new() -> Self {
        PPM {
            header: PPMHeader::new(),
            values: Vec::new(),
        }
    }
}

/// Reads the ASCII file. Right now we just return the values. This is done because we've already built the header data.
/// However, we have to read over the header, and parse data for now. @TODO: Fix this.
fn read_ppm_ascii_file(path: &str, ppm_type: PpmType) -> Vec<PpmValue> {
    let file = File::open(path).unwrap();
    let mut dat : PPM = PPM::new();
    dat.header.ppm_type = ppm_type;
    let reader = BufReader::new(file);

    let mut skip_first_line : bool = false;
    for line in reader.lines() {
        //println!("{}", line?);
        let va = line.unwrap_or_default();
        // we just skip over this line since we already have the ppm type
        if !skip_first_line {
            skip_first_line = true;
            continue
        }
        // determine if there is a comment at the start of the line;
        if va.clone().chars().next().unwrap_or_default() == '#' {
            //println!("Found Comment on => {:?}", va.clone());
            continue
        }

        if dat.header.width == 0 && dat.header.height == 0 {
            let bar : Vec<i32> = va.split(' ').map(|x| x.parse::<i32>().unwrap()).collect();
            dat.header.width = bar[0];

            dat.header.height = bar[1];
            //println!("This is width & Height: {:?}", bar);
            continue
        }

        if dat.header.max_value == 0 && dat.header.ppm_type != PpmType::P1 {
            dat.header.max_value = va.parse::<i32>().unwrap_or_default();
            //println!("This is width & Height: {:?}", dat.max_value);
            continue
        }
        let offset : usize = if va.clone().find('#').unwrap_or_default() == 0  {
            va.clone().len()
         } else {
            va.clone().find('#').unwrap_or_default()
         };

        let x : Vec<i32> = (&va[0..offset]).split_whitespace().map(|x| x.parse::<i32>().unwrap()).collect();
        if dat.header.ppm_type == PpmType::P3 {
            dat.values.push(PpmValue::new(x[0], x[1], x[2]));
        }
        else if dat.header.ppm_type == PpmType::P2 {
            for val in x {
                dat.values.push(PpmValue::new(
                  ((val as f32/dat.header.max_value as f32) * 255.0) as i32, 
                  ((val as f32/dat.header.max_value as f32) * 255.0) as i32, 
                  ((val as f32/dat.header.max_value as f32) * 255.0) as i32
                ));
                //println!("{:?} / {:?} = {:?}", val as f32, dat.max_value as f32, ((val as f32/dat.max_value as f32) * 255.0) as i32);
            }
        } else if dat.header.ppm_type == PpmType::P1 {
            for val in x {
                let pixel_data = if val == 0 {
                    0
                } else {
                    255
                };
                dat.values.push(PpmValue::new(pixel_data, pixel_data, pixel_data));
                //println!("{:?} / {:?} = {:?}", val as f32, dat.max_value as f32, ((val as f32/dat.max_value as f32) * 255.0) as i32);
            }
        }
    }
    dat.values
}

/// Given a path, it will parse the header information for the PPM family of files
/// and returns the byte position where the header ends as well as the data inside
/// the header object. 
fn read_ppm_header(path: &str) -> (usize, PPMHeader) {
    let mut f = File::open(path).unwrap();
    let mut byte_position: usize = 0;

    let mut header: PPMHeader = PPMHeader::new();
    let mut magic_number = [0; 2];

    /*#region Get the type of PPM file */
    // Get the type of PPM file we are reading
    f.read_exact(&mut magic_number).unwrap();
    let ppm_type = match magic_number {
        [80, 49] => { PpmType::P1 },
        [80, 50] => { PpmType::P2 },
        [80, 51] => { PpmType::P3 },
        [80, 52] => { PpmType::P4 },
        [80, 53] => { PpmType::P5 },
        [80, 54] => { PpmType::P6 },
        _ => { PpmType::P0 }
    };
    header.ppm_type = ppm_type;
    byte_position += 2;
    /* #endregion */

    // if we have found an ASCII ppm file (p3) then we pass this data onto 
    let mut byte_for = [0; 1]; // important note: 0x32 is the whitespace code.
    while let Ok(n) = f.read(&mut byte_for) {
        if header.width != 0 && header.height != 0 && (header.max_value != 0 || (header.ppm_type == PpmType::P1 || header.ppm_type == PpmType::P4)) {
            byte_position += 1;
            break;
        }
        if n != 0 {
            // we need to find out something
            let mut number_byte = Vec::new(); // important note: 0x32 is the whitespace code.

            // ensure we don't double read over an actual piece of information
            if byte_for != [10] && byte_for != [32] && byte_for != [35] && byte_for != [13] {
                number_byte.push(byte_for[0]);
                byte_position += 1;
            }

            /* #region Read until whitespace */
            // read bytes until whitespace or \n
            while let Ok(n) = f.read(&mut byte_for) {
                if n != 0 {
                    // we increment the byte position regardless of what type
                    byte_position += 1;
                    
                    // which byte is found?
                    match byte_for {
                        [10] | [13] | [32] => { // any of these indicates whitespace (I think?)
                            break;
                        },
                        // we have encountered a comment, read until a line break
                        [35] => {
                            while let Ok(z) = f.read(&mut byte_for) {
                                if z!= 0 {
                                    byte_position += 1;
                                    if byte_for == [35] || byte_for == [13] || byte_for == [10] {
                                        break;
                                    }
                                }
                                else {
                                    break;
                                }
                            }
                            continue
                        }
                        _ => {
                            number_byte.push(byte_for[0]);
                            continue
                        },
                    }
                } else {
                    break
                }
            }
            /* #endregion */

            // we need to load up data;
            // converts byte array into integer values
            if header.width == 0 {
                header.width = String::from_utf8_lossy(&number_byte).parse::<i32>().unwrap_or_default();
                continue
            }
            if header.height == 0 {
                header.height = String::from_utf8_lossy(&number_byte).parse::<i32>().unwrap_or_default();
                continue
            }
            if header.max_value == 0 && (header.ppm_type != PpmType::P1 && header.ppm_type != PpmType::P4) {
                header.max_value = String::from_utf8_lossy(&number_byte).parse::<i32>().unwrap_or_default();
                continue
            }
        } else {
            break
        }
    }

    (byte_position, header)
}

fn read_ppm_binary_image_data(path: &str, ppm_object: PPM, start_position: usize) -> Vec<PpmValue> {

    // open the file
    let mut f = File::open(path).unwrap();
    
    // seek to the correct position for the image data to start
    f.seek(SeekFrom::Start((start_position) as u64)).unwrap();

    // create a new vector of PpmValues
    let mut img_data = Vec::<PpmValue>::new();

    if ppm_object.header.ppm_type == PpmType::P6 {
        let mut byte_for = [0; 3]; // important note: 0x32 is the whitespace code.
        while let Ok(n) = f.read(&mut byte_for) {
            if n != 0 {
                img_data.push(PpmValue::new(
                  i32::from_be_bytes([0,0,0,byte_for[0]]),
                  i32::from_be_bytes([0,0,0,byte_for[1]]), 
                  i32::from_be_bytes([0,0,0,byte_for[2]])
                ));
            }
            else {
                break;
            }
        }
    } else if ppm_object.header.ppm_type == PpmType::P5 {
        let mut byte_for = [0; 1]; // important note: 0x32 is the whitespace code.
        while let Ok(n) = f.read(&mut byte_for) {
            if n != 0 {
                let gs_data = i32::from_be_bytes([0,0,0,byte_for[0]]);
                img_data.push(PpmValue::new(
                  ((gs_data as f32 / ppm_object.header.max_value as f32) * 255.0) as i32,
                  ((gs_data as f32 / ppm_object.header.max_value as f32) * 255.0) as i32,
                  ((gs_data as f32 / ppm_object.header.max_value as f32) * 255.0) as i32
                ));
            }
            else {
                break;
            }
        }
    } else if ppm_object.header.ppm_type == PpmType::P4 {
        let mut byte_for = [0; 1]; // important note: 0x32 is the whitespace code.
        while let Ok(n) = f.read(&mut byte_for) {
            if n != 0 {
                for i in (0..8).rev() {
                    
                    let pixel_data = get_bit_at(byte_for[0], i).unwrap();

                    let final_value = if pixel_data {
                        0
                    } else {
                        255
                    };

                    img_data.push(PpmValue::new(
                      final_value, 
                      final_value, 
                      final_value)
                    );
                    //println!("{:?} => {:?} = {:?}", byte_for[0], i, get_bit_at(byte_for[0] as u32, i).unwrap());
                }
            }
            else {
                break;
            }
        }
    }

    img_data
}

fn get_bit_at(input: u8, n: u8) -> Result<bool, ()> {
    if n < 8 {
        Ok(input & (1 << n) != 0)
    } else {
        Err(())
    }
}

fn main() -> Result<(), Error> {

    // get the arguments from the command line
    let args: Vec<String> = env::args().collect();

    // require the filename
    if args.len() <= 1 {
        println!("File Name is required.");
        std::process::exit(0);
    }

    let filename = &args[1];

    if filename.is_empty() {
        println!("File Name is required.");
        std::process::exit(0);
    }

    let mut world = World::new();
    

    let mut ppm = PPM::new();
    let (byte_position, header) : (usize, PPMHeader) = read_ppm_header(filename);
    
    
    ppm.header = header.to_owned();

    if header.ppm_type == PpmType::P1 || header.ppm_type == PpmType::P2 || header.ppm_type == PpmType::P3 {
      ppm.values = read_ppm_ascii_file(filename, header.clone().ppm_type);
    } else if header.ppm_type == PpmType::P6 || header.ppm_type == PpmType::P5 || header.ppm_type == PpmType::P4 {
        // there is an issue where byte were misaligned.
        ppm.values = read_ppm_binary_image_data(filename, ppm.clone(), byte_position);
    }
    world.frame = Some(ppm);

    let w_width = world.clone().get_width();
    let w_height = world.clone().get_height();
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(w_width as f64, w_height as f64);
        WindowBuilder::new()
            .with_title("PPMViewer - by github@VishalRamki")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(w_width as u32, w_height as u32, surface_texture)?
    };

    //let mut graphic = aci_ppm::decode(&input_f, afi::ColorChannels::Rgb).unwrap();
    //world.frame = graphic.pop();
    event_loop.run(move |event, _, control_flow| {
        // Draw the current frame
        if let Event::RedrawRequested(_) = event {
            world.draw(pixels.get_frame());
            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Resize the window
            // @TODO: Ensure we can resize the window;
            
            if let Some(size) = input.window_resized() {
                //pixels.surface_size(size.width, size.height);
                pixels.resize(size.width, size.height);
            }
            
            // Update internal state and request a redraw
            window.request_redraw();
        }
    });
}

impl World {
    /// Create a new `World` instance that can draw a moving box.
    fn new() -> Self {
        Self {
            frame: None,
            single_draw: true,
            has_been_drawn: false
        }
    }

    /// Draw the `World` state to the frame buffer.
    ///
    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    fn draw(&mut self, frame: &mut [u8]) {
        if self.single_draw && self.has_been_drawn {
            return
        }
        if self.frame.as_ref().unwrap().header.ppm_type != PpmType::P0 {
            for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
                let frame_instance = self.frame.as_ref().unwrap();
                let rgba = [frame_instance.values[i].r as u8, frame_instance.values[i].g as u8, frame_instance.values[i].b as u8, 0xff];
                pixel.copy_from_slice(&rgba);
            }
        } 

        if self.single_draw && !self.has_been_drawn {
            self.has_been_drawn = true;
        }
    }
}