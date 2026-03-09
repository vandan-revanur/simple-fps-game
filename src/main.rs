use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = simple_quake::app::App::new();
    event_loop.run_app(&mut app).unwrap();
}
