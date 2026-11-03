use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 950;

fn calcola_pixel(x: f32, y: f32) -> u32 {
    let aspect_ratio = WIDTH as f32 / HEIGHT as f32;
    let x_span = 3.0;
    let y_span = x_span / aspect_ratio;

    let a = -2.0 + (x / WIDTH as f32) * x_span;
    let b = (-y_span / 2.0) + (y / HEIGHT as f32) * y_span;

    let mut z_a = 0.0;
    let mut z_b = 0.0;
    let max_iter = 150;

    for i in 0..max_iter {
        if (z_a * z_a + z_b * z_b) > 4.0 {
            return i;
        }
        let new_z_a = (z_a * z_a) - (z_b * z_b) + a;
        let new_z_b = 2.0 * z_a * z_b + b;
        z_a = new_z_a;
        z_b = new_z_b;
    }
    max_iter
}

fn disegna_mandelbrot(frame: &mut [u8]) {
    for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
        let x = (i as u32) % WIDTH;
        let y = (i as u32) / WIDTH;

        let iterazioni = calcola_pixel(x as f32, y as f32);

        let colore = if iterazioni == 150 {
            [0, 0, 0, 255]
        } else {
            let c = (iterazioni * 5) as u8;
            [c.wrapping_mul(2), c.wrapping_mul(5), 255, 255]
        };

        pixel.copy_from_slice(&colore);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Insieme di Mandelbrot")
            .with_inner_size(LogicalSize::new(WIDTH, HEIGHT))
            .build(&event_loop)?
    );

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window.as_ref());
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    disegna_mandelbrot(pixels.frame_mut());

    let window_clone = Arc::clone(&window);

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                
                WindowEvent::Resized(size) => {
                    if let Err(err) = pixels.resize_surface(size.width, size.height) {
                        eprintln!("Errore nel ridimensionamento: {err}");
                        elwt.exit();
                    }
                }
                
                WindowEvent::RedrawRequested => {
                    if let Err(err) = pixels.render() {
                        eprintln!("Errore di rendering: {err}");
                        elwt.exit();
                    }
                }
                _ => (),
            },
            Event::AboutToWait => {

                window_clone.request_redraw();
            }
            _ => (),
        }
    })?;

    Ok(())
}