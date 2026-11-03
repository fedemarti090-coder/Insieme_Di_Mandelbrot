use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

const WIDTH: u32 = 800;
const HEIGTH: u32 = 600;

fn calcola_pixel(x: f32, y: f32, width: f32, heigth: f32) ->u32 {

    let a = -2.0 + (x / width) * 3.0;
    let b =  1.2 + (y / heigth) * 2.4;

    let mut z_a = 0.0;
    let mut z_b = 0.0;

    let max_iter = 100;

    for i in 1..max_iter {

        if (z_a * z_a + z_b * z_b) > 4.0 {
            return i;
        }

        let mut new_z_a = z_a * z_a - z_b * z_b;
        let mut new_z_b = 2.0 * z_a * z_b ;

        new_z_a += a;
        new_z_b += b;

        z_a = new_z_a;
        z_b = new_z_b;
        
    }

    max_iter
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let event_loop = EventLoop::new()?;

    let window = WindowBuilder::new()
        .with_title("Mandelbrot Fractal - Rust")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)?;

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                elwt.exit();
            }
            _ => (),
        }
    })?;

    Ok(())
}
