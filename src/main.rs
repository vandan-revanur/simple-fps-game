use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Instant;

use cgmath::{perspective, Deg, EuclideanSpace, InnerSpace, Matrix4, Point3, Vector3};
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;

// --- Mouse accumulator (lock-free, stores scaled fractional deltas) ---
const MOUSE_SCALE: f32 = 1000.0;
struct MouseAccum {
    dx: AtomicI32,
    dy: AtomicI32,
    active: std::sync::atomic::AtomicBool,
}
impl MouseAccum {
    fn new() -> Self {
        Self {
            dx: AtomicI32::new(0),
            dy: AtomicI32::new(0),
            active: std::sync::atomic::AtomicBool::new(false),
        }
    }
    fn add_raw(&self, dx: i32, dy: i32) {
        // raw int deltas (from /dev/input/mice). Scale to preserve fractions.
        self.dx.fetch_add((dx as f32 * MOUSE_SCALE) as i32, Ordering::Relaxed);
        self.dy.fetch_add((dy as f32 * MOUSE_SCALE) as i32, Ordering::Relaxed);
    }
    fn add_f64(&self, dx: f64, dy: f64) {
        self.dx.fetch_add((dx as f64 * MOUSE_SCALE as f64) as i32, Ordering::Relaxed);
        self.dy.fetch_add((dy as f64 * MOUSE_SCALE as f64) as i32, Ordering::Relaxed);
    }
    fn take(&self) -> (f32, f32) {
        let dx = self.dx.swap(0, Ordering::Relaxed);
        let dy = self.dy.swap(0, Ordering::Relaxed);
        (dx as f32 / MOUSE_SCALE, dy as f32 / MOUSE_SCALE)
    }
    fn set_active(&self) { self.active.store(true, Ordering::Relaxed); }
    fn is_active(&self) -> bool { self.active.load(Ordering::Relaxed) }
}

fn spawn_mouse_thread(accum: Arc<MouseAccum>) {
    std::thread::spawn(move || {
        use std::io::Read;
        // Write a small log so the user can inspect if the thread started
        let _ = std::fs::write("/tmp/simple_quake_mouse.log", "mouse thread starting\n");
        match std::fs::File::open("/dev/input/mice") {
            Ok(mut f) => {
                let _ = std::fs::write("/tmp/simple_quake_mouse.log", "opened /dev/input/mice\n");
                accum.set_active();
                let mut buf = [0u8; 3];
                let mut count = 0u64;
                loop {
                    if f.read_exact(&mut buf).is_err() { break; }
                    // PS/2 3-byte packet: buf[1]=dx, buf[2]=dy (signed)
                    let dx = buf[1] as i8 as i32;
                    let dy = -(buf[2] as i8 as i32); // invert Y
                    accum.add_raw(dx, dy);
                    count += 1;
                    if count % 200 == 0 {
                        let _ = std::fs::write("/tmp/simple_quake_mouse.log",
                            format!("mouse events: {} last_dx={} last_dy={}\n", count, dx, dy));
                    }
                }
                let _ = std::fs::write("/tmp/simple_quake_mouse.log", "mouse thread exiting\n");
            }
            Err(e) => {
                let _ = std::fs::write("/tmp/simple_quake_mouse.log",
                    format!("failed to open /dev/input/mice: {}\n", e));
            }
        }
    });
}

#[derive(Clone)]
struct Enemy {
    position: Vector3<f32>,
    alive: bool,
    angle_degrees: i32,
}

struct Bullet {
    position: Vector3<f32>,
    direction: Vector3<f32>,
    lifetime: f32,
}

struct Camera {
    position: Vector3<f32>,
    yaw: f32,
    pitch: f32,
    aspect: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: Vector3::new(0.0, 1.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            aspect: WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32,
        }
    }

    fn get_view_matrix(&self) -> Matrix4<f32> {
        let forward = Vector3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        );
        let target = self.position + forward;
        Matrix4::look_at_rh(
            Point3::from_vec(self.position),
            Point3::from_vec(target),
            Vector3::unit_y(),
        )
    }

    fn get_projection_matrix(&self) -> Matrix4<f32> {
        perspective(Deg(70.0), self.aspect, 0.1, 100.0)
    }

    fn get_view_projection(&self) -> Matrix4<f32> {
        self.get_projection_matrix() * self.get_view_matrix()
    }

    fn get_forward_vector(&self) -> Vector3<f32> {
        Vector3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }

    /// Returns right vector (perpendicular to forward, horizontal)
    fn get_right_vector(&self) -> Vector3<f32> {
        let forward = self.get_forward_vector().normalize();
        forward.cross(Vector3::unit_y()).normalize()
    }

    /// Returns up vector relative to camera
    fn get_up_vector(&self) -> Vector3<f32> {
        let forward = self.get_forward_vector().normalize();
        let right = self.get_right_vector();
        right.cross(forward).normalize()
    }
}

struct Input {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    shoot: bool,
    look_left: bool,
    look_right: bool,
    look_up: bool,
    look_down: bool,
}

impl Input {
    fn new() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            shoot: false,
            look_left: false,
            look_right: false,
            look_up: false,
            look_down: false,
        }
    }
}

struct App {
    window: Option<Arc<Window>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    camera: Camera,
    input: Input,
    camera_buffer: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
    pipeline: Option<wgpu::RenderPipeline>,
    enemies: Vec<Enemy>,
    bullets: Vec<Bullet>,
    ui_pipeline: Option<wgpu::RenderPipeline>,
    ui_vertex_buffer: Option<wgpu::Buffer>,
    ui_index_buffer: Option<wgpu::Buffer>,
    last_update: Instant,
    mouse_accum: Arc<MouseAccum>,
}

impl App {
    fn new() -> Self {
        // Create enemies at every 5 degrees in a 180-degree arc in front of the player
        let mut enemies = Vec::new();
        let distance = 10.0; // Distance from origin
        let height = 1.0;

        // Generate enemies from -90 to +90 degrees (180 degree field)
        for angle_deg in (-90..=90).step_by(5) {
            let angle_rad = (angle_deg as f32).to_radians();
            // The player starts looking down negative Z axis (yaw = 0)
            // So we place enemies in a semicircle around that
            let x = angle_rad.sin() * distance;
            let z = -angle_rad.cos() * distance;

            enemies.push(Enemy {
                position: Vector3::new(x, height, z),
                alive: true,
                angle_degrees: angle_deg,
            });
        }

        let mouse_accum = Arc::new(MouseAccum::new());
        spawn_mouse_thread(Arc::clone(&mouse_accum));

        Self {
            window: None,
            device: None,
            queue: None,
            surface: None,
            surface_config: None,
            camera: Camera::new(),
            input: Input::new(),
            camera_buffer: None,
            bind_group: None,
            pipeline: None,
            enemies,
            bullets: Vec::new(),
            ui_pipeline: None,
            ui_vertex_buffer: None,
            ui_index_buffer: None,
            last_update: Instant::now(),
            mouse_accum,
        }
    }

    fn init_rendering(&mut self, window: Arc<Window>) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to find an appropriate adapter");

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .or_else(|| surface_caps.formats.first().copied())
            .expect("No compatible surface format found");

        // Use the actual physical size of the window (important on HiDPI displays)
        let phys = window.inner_size();
        let surf_width  = phys.width.max(1);
        let surf_height = phys.height.max(1);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: surf_width,
            height: surf_height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            desired_maximum_frame_latency: 1,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // Camera uniform as flat array
        let camera_data = [0.0f32; 16];
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&camera_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                r#"
                struct Camera {
                    view_proj: mat4x4<f32>,
                }
                @group(0) @binding(0) var<uniform> camera: Camera;

                struct VertexInput {
                    @location(0) position: vec3<f32>,
                    @location(1) color: vec3<f32>,
                }

                struct VertexOutput {
                    @builtin(position) clip_position: vec4<f32>,
                    @location(0) color: vec3<f32>,
                }

                @vertex
                fn vs_main(input: VertexInput) -> VertexOutput {
                    var out: VertexOutput;
                    out.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
                    out.color = input.color;
                    return out;
                }

                @fragment
                fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
                    return vec4<f32>(input.color, 1.0);
                }
            "#,
            )),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 24,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        // --- UI pipeline (crosshair) ---
        let ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                r#"
                struct VertexInput {
                    @location(0) position: vec2<f32>,
                    @location(1) color: vec3<f32>,
                }

                struct VertexOutput {
                    @builtin(position) clip_position: vec4<f32>,
                    @location(0) color: vec3<f32>,
                }

                @vertex
                fn vs_main(input: VertexInput) -> VertexOutput {
                    var out: VertexOutput;
                    out.clip_position = vec4<f32>(input.position, 0.0, 1.0);
                    out.color = input.color;
                    return out;
                }

                @fragment
                fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
                    return vec4<f32>(input.color, 1.0);
                }
            "#,
            )),
        });

        let ui_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("UI Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI Pipeline"),
            layout: Some(&ui_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ui_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 8,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &ui_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        // Crosshair vertices
        let ui_vertices: Vec<[f32; 5]> = vec![
            [-0.02, 0.0, 1.0, 1.0, 1.0],
            [0.02,  0.0, 1.0, 1.0, 1.0],
            [0.0, -0.02, 1.0, 1.0, 1.0],
            [0.0,  0.02, 1.0, 1.0, 1.0],
        ];

        let ui_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Vertex Buffer"),
            contents: bytemuck::cast_slice(&ui_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let ui_indices: Vec<u16> = (0..ui_vertices.len() as u16).collect();
        let ui_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Index Buffer"),
            contents: bytemuck::cast_slice(&ui_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.window = Some(window);
        self.device = Some(device);
        self.queue = Some(queue);
        self.surface = Some(surface);
        self.surface_config = Some(config);
        self.camera.aspect = surf_width as f32 / surf_height as f32;
        self.camera_buffer = Some(camera_buffer);
        self.bind_group = Some(bind_group);
        self.pipeline = Some(pipeline);
        self.ui_pipeline = Some(ui_pipeline);
        self.ui_vertex_buffer = Some(ui_vertex_buffer);
        self.ui_index_buffer = Some(ui_index_buffer);
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32().min(0.05);
        self.last_update = now;

        let move_speed = 5.0;

        let forward = Vector3::new(-self.camera.yaw.sin(), 0.0, -self.camera.yaw.cos());
        let right   = Vector3::new( self.camera.yaw.cos(), 0.0, -self.camera.yaw.sin());

        if self.input.forward  { self.camera.position += forward * move_speed * dt; }
        if self.input.backward { self.camera.position -= forward * move_speed * dt; }
        if self.input.left     { self.camera.position -= right   * move_speed * dt; }
        if self.input.right    { self.camera.position += right   * move_speed * dt; }

        // Consume accumulated mouse deltas once per frame (both raw thread and DeviceEvent write here)
        let (mdx, mdy) = self.mouse_accum.take();
        if mdx.abs() > 0.0001 || mdy.abs() > 0.0001 {
            const SENSITIVITY: f32 = 0.003;
            self.camera.yaw   -= mdx * SENSITIVITY;
            self.camera.pitch -= mdy * SENSITIVITY;
        }

        // Keyboard look (arrow keys) — fallback
        let look_speed = 2.5; // radians per second
        if self.input.look_left  { self.camera.yaw   += look_speed * dt; }
        if self.input.look_right { self.camera.yaw   -= look_speed * dt; }
        if self.input.look_up    { self.camera.pitch  += look_speed * dt; }
        if self.input.look_down  { self.camera.pitch  -= look_speed * dt; }

        use std::f32::consts::PI;
        if self.camera.yaw > PI       { self.camera.yaw -= 2.0 * PI; }
        else if self.camera.yaw < -PI { self.camera.yaw += 2.0 * PI; }
        self.camera.pitch = self.camera.pitch.clamp(-1.5, 1.5);

        if self.input.shoot {
            let dir    = self.camera.get_forward_vector().normalize();
            let muzzle = Self::gun_barrel_tip(&self.camera);
            self.bullets.push(Bullet { position: muzzle, direction: dir, lifetime: 5.0 });
            self.input.shoot = false;
        }

        let bullet_speed = 20.0;
        self.bullets.retain_mut(|bullet| {
            bullet.position += bullet.direction * bullet_speed * dt;
            bullet.lifetime -= dt;
            bullet.lifetime > 0.0
        });

        for bullet in &self.bullets {
            for enemy in &mut self.enemies {
                if !enemy.alive { continue; }
                if (bullet.position - enemy.position).magnitude() < 1.5 {
                    enemy.alive = false;
                    let sign = if enemy.angle_degrees >= 0 { "positive" } else { "negative" };
                    println!("Enemy at {} angle with value: {} degrees, destroyed!",
                        sign, enemy.angle_degrees.abs());
                }
            }
        }
    }

    /// Build vertices for a cuboid given min/max corners and a flat color [r,g,b].
    /// Returns triangles (6 faces × 2 triangles × 3 verts = 36 verts).
    /// Emit 36 vertices for an oriented box defined by its 8 corners in world space.
    ///
    /// Corner layout (r = right axis, u = up axis, f = forward axis):
    ///   0: (-r, -u, -f)   "back-bottom-left"
    ///   1: (+r, -u, -f)   "back-bottom-right"
    ///   2: (+r, +u, -f)   "back-top-right"
    ///   3: (-r, +u, -f)   "back-top-left"
    ///   4: (-r, -u, +f)   "front-bottom-left"
    ///   5: (+r, -u, +f)   "front-bottom-right"
    ///   6: (+r, +u, +f)   "front-top-right"
    ///   7: (-r, +u, +f)   "front-top-left"
    fn oriented_box(corners: [Vector3<f32>; 8], color: [f32; 3], dark: [f32; 3]) -> Vec<[f32; 6]> {
        let c = color;
        let d = dark;
        // helper to pack a vertex
        let v = |p: Vector3<f32>, col: [f32; 3]| -> [f32; 6] {
            [p.x, p.y, p.z, col[0], col[1], col[2]]
        };
        let [c0, c1, c2, c3, c4, c5, c6, c7] = corners;
        // shade each face slightly differently for visibility
        let front  = c;
        let back   = d;
        let top    = [c[0]*0.8, c[1]*0.8, c[2]*0.8];
        let bottom = [d[0]*0.6, d[1]*0.6, d[2]*0.6];
        let right  = [c[0]*0.7, c[1]*0.7, c[2]*0.7];
        let left   = [c[0]*0.9, c[1]*0.9, c[2]*0.9];
        vec![
            // Front face  (c4 c5 c6 c7)
            v(c4,front), v(c5,front), v(c6,front),
            v(c4,front), v(c6,front), v(c7,front),
            // Back face   (c1 c0 c3 c2)
            v(c1,back),  v(c0,back),  v(c3,back),
            v(c1,back),  v(c3,back),  v(c2,back),
            // Top face    (c3 c2 c6 c7)  -- note: c2/c3 are back-top, c6/c7 front-top
            v(c3,top),   v(c2,top),   v(c6,top),
            v(c3,top),   v(c6,top),   v(c7,top),
            // Bottom face (c0 c1 c5 c4)
            v(c0,bottom),v(c1,bottom),v(c5,bottom),
            v(c0,bottom),v(c5,bottom),v(c4,bottom),
            // Right face  (c1 c2 c6 c5)
            v(c1,right), v(c2,right), v(c6,right),
            v(c1,right), v(c6,right), v(c5,right),
            // Left face   (c0 c4 c7 c3)
            v(c0,left),  v(c4,left),  v(c7,left),
            v(c0,left),  v(c7,left),  v(c3,left),
        ]
    }

    /// Build oriented-box corners for a box whose axes are (right, up, fwd) in world space,
    /// spanning [-hw..+hw] along right, [-hh..+hh] along up, [f0..f1] along fwd,
    /// all relative to `anchor`.
    fn box_corners(
        anchor: Vector3<f32>,
        right: Vector3<f32>,
        up: Vector3<f32>,
        fwd: Vector3<f32>,
        hw: f32, hh: f32,
        f0: f32, f1: f32,
    ) -> [Vector3<f32>; 8] {
        let p = |r: f32, u: f32, f: f32| anchor + right * r + up * u + fwd * f;
        [
            p(-hw, -hh, f0), // 0 back-bottom-left
            p( hw, -hh, f0), // 1 back-bottom-right
            p( hw,  hh, f0), // 2 back-top-right
            p(-hw,  hh, f0), // 3 back-top-left
            p(-hw, -hh, f1), // 4 front-bottom-left
            p( hw, -hh, f1), // 5 front-bottom-right
            p( hw,  hh, f1), // 6 front-top-right
            p(-hw,  hh, f1), // 7 front-top-left
        ]
    }

    /// Build 3D gun vertices positioned in world space relative to the camera,
    /// so it always appears in the lower-right of the screen like a classic FPS.
    fn build_gun_verts(camera: &Camera) -> Vec<[f32; 6]> {
        let fwd   = camera.get_forward_vector().normalize();
        let right = camera.get_right_vector();
        let up    = camera.get_up_vector();

        // Gun anchor: slightly in front, to the right, and below the camera
        let anchor = camera.position
            + fwd   *  0.6
            + right *  0.35
            + up    * -0.28;

        // Dimensions
        let barrel_hw = 0.03_f32;
        let barrel_hh = 0.03_f32;

        let body_hw = 0.06_f32;
        let body_hh = 0.08_f32;
        let body_len = 0.20_f32;

        let grip_hw     = 0.04_f32;
        let grip_height = 0.12_f32;
        let grip_len    = 0.08_f32;

        // Colours
        let barrel_col : [f32; 3] = [0.25, 0.25, 0.25];
        let barrel_dark: [f32; 3] = [0.15, 0.15, 0.15];
        let body_col   : [f32; 3] = [0.35, 0.30, 0.20];
        let body_dark  : [f32; 3] = [0.20, 0.17, 0.10];
        let grip_col   : [f32; 3] = [0.20, 0.15, 0.10];
        let grip_dark  : [f32; 3] = [0.12, 0.09, 0.06];

        // Barrel: centred on anchor, runs forward from f=0 to f=0.55
        let barrel_corners = Self::box_corners(anchor, right, up, fwd,
            barrel_hw, barrel_hh, 0.0, 0.55);

        // Body: centred on anchor, runs backward from f=0 to f=-body_len
        let body_corners = Self::box_corners(anchor, right, up, fwd,
            body_hw, body_hh, -body_len, 0.0);

        // Grip: hangs below the rear of the body.
        let grip_anchor = anchor + up * (-body_hh);
        let grip_corners = Self::box_corners(grip_anchor, right, up, fwd,
            grip_hw, grip_height * 0.5,
            -body_len + grip_len * 2.0, -body_len + grip_len * 4.0);

        let mut verts = Vec::new();
        verts.extend(Self::oriented_box(barrel_corners, barrel_col, barrel_dark));
        verts.extend(Self::oriented_box(body_corners,   body_col,   body_dark));
        verts.extend(Self::oriented_box(grip_corners,   grip_col,   grip_dark));
        verts
    }

    /// Returns the world-space position of the barrel muzzle — identical offsets to build_gun_verts.
    fn gun_barrel_tip(camera: &Camera) -> Vector3<f32> {
        let fwd   = camera.get_forward_vector().normalize();
        let right = camera.get_right_vector();
        let up    = camera.get_up_vector();
        let anchor = camera.position
            + fwd   *  0.6
            + right *  0.35
            + up    * -0.28;
        // barrel tip = anchor + fwd * 0.55 (the front face of the barrel box)
        anchor + fwd * 0.55
    }

    fn render(&mut self) {
        let device = self.device.as_ref().unwrap();
        let queue  = self.queue.as_ref().unwrap();
        let surface = self.surface.as_ref().unwrap();
        let pipeline   = self.pipeline.as_ref().unwrap();
        let bind_group = self.bind_group.as_ref().unwrap();
        let camera_buffer = self.camera_buffer.as_ref().unwrap();

        // Upload camera matrix
        let vp = self.camera.get_view_projection();
        let vp_arr: [[f32; 4]; 4] = vp.into();
        let vp_flat: [f32; 16] = [
            vp_arr[0][0], vp_arr[0][1], vp_arr[0][2], vp_arr[0][3],
            vp_arr[1][0], vp_arr[1][1], vp_arr[1][2], vp_arr[1][3],
            vp_arr[2][0], vp_arr[2][1], vp_arr[2][2], vp_arr[2][3],
            vp_arr[3][0], vp_arr[3][1], vp_arr[3][2], vp_arr[3][3],
        ];
        queue.write_buffer(camera_buffer, 0, bytemuck::cast_slice(&[vp_flat]));

        // Build world geometry
        let mut vertices: Vec<[f32; 6]> = vec![
            // Floor (blue)
            [-50.0, 0.0, -50.0, 0.0, 0.0, 1.0],
            [ 50.0, 0.0, -50.0, 0.0, 0.0, 1.0],
            [ 50.0, 0.0,  50.0, 0.0, 0.0, 1.0],
            [-50.0, 0.0, -50.0, 0.0, 0.0, 1.0],
            [ 50.0, 0.0,  50.0, 0.0, 0.0, 1.0],
            [-50.0, 0.0,  50.0, 0.0, 0.0, 1.0],
        ];

        // Alive enemies (red cuboids)
        for enemy in &self.enemies {
            if !enemy.alive { continue; }
            let pos = enemy.position;
            let ev = Self::oriented_box(
                Self::box_corners(pos, Vector3::unit_x(), Vector3::unit_y(), Vector3::unit_z(),
                    1.0, 1.0, -1.0, 1.0),
                [1.0, 0.0, 0.0],
                [0.5, 0.0, 0.0],
            );
            vertices.extend(ev);
        }

        // Bullets (small yellow cuboids)
        for bullet in &self.bullets {
            let pos = bullet.position;
            let s = 0.1_f32;
            let bv = Self::oriented_box(
                Self::box_corners(pos, Vector3::unit_x(), Vector3::unit_y(), Vector3::unit_z(),
                    s, s, -s, s),
                [1.0, 1.0, 0.0],
                [0.7, 0.7, 0.0],
            );
            vertices.extend(bv);
        }

        // 3D gun cuboids (view-space, attached to camera)
        let gun_verts = Self::build_gun_verts(&self.camera);
        vertices.extend(gun_verts);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indices: Vec<u16> = (0..vertices.len() as u16).collect();
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let output = surface.get_current_texture().unwrap();
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 3D world pass (floor + enemies + bullets + 3D gun)
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.1, b: 0.2, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..vertices.len() as u32, 0, 0..1);
        }

        // 2D UI pass (crosshair)
        {
            let ui_pipeline      = self.ui_pipeline.as_ref().unwrap();
            let ui_vertex_buffer = self.ui_vertex_buffer.as_ref().unwrap();
            let ui_index_buffer  = self.ui_index_buffer.as_ref().unwrap();

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            pass.set_pipeline(ui_pipeline);
            pass.set_vertex_buffer(0, ui_vertex_buffer.slice(..));
            pass.set_index_buffer(ui_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..4, 0, 0..1);
        }

        queue.submit([encoder.finish()]);
        output.present();
        // No request_redraw() here — about_to_wait drives the loop
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::Window::default_attributes()
                        .with_title("Simple Quake")
                        .with_inner_size(winit::dpi::PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT)),
                )
                .unwrap(),
        );

        // Try Locked first (gives raw device events). Fall back to Confined.
        if window.set_cursor_grab(winit::window::CursorGrabMode::Locked).is_err() {
            let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined);
            let _ = std::fs::write("/tmp/simple_quake_mouse.log", "using confined cursor\n");
        } else {
            let _ = std::fs::write("/tmp/simple_quake_mouse.log", "using locked cursor\n");
        }
        let _ = window.set_cursor_visible(false);

        // Log whether the background /dev/input/mice reader is already active (if it started in new())
        let active = self.mouse_accum.is_active();
        let _ = std::fs::write("/tmp/simple_quake_mouse.log",
            format!("mouse_accum active: {}\n", active));

        self.init_rendering(window);
        event_loop.set_control_flow(ControlFlow::Poll);

        println!("=== SIMPLE QUAKE ===");
        println!("Controls:");
        println!("  WASD or Arrow Keys: Move");
        println!("  Arrow Keys: Look around");
        println!("  Left Click: Shoot");
        println!("  ESC: Quit");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => { event_loop.exit(); }
            WindowEvent::Resized(new_size) => {
                let w = new_size.width.max(1);
                let h = new_size.height.max(1);
                if let (Some(device), Some(surface), Some(config)) = (
                    self.device.as_ref(),
                    self.surface.as_ref(),
                    self.surface_config.as_mut(),
                ) {
                    config.width  = w;
                    config.height = h;
                    surface.configure(device, config);
                }
                self.camera.aspect = w as f32 / h as f32;
            }
            WindowEvent::RedrawRequested => {
                self.update();
                self.render();
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent { physical_key, state, repeat, .. }, ..
            } => {
                if repeat { return; }
                let pressed = state == ElementState::Pressed;
                if let PhysicalKey::Code(code) = physical_key {
                    match code {
                        // Movement (WASD + Arrow keys both work)
                        KeyCode::KeyW  => self.input.forward   = pressed,
                        KeyCode::KeyS  => self.input.backward  = pressed,
                        KeyCode::KeyA  => self.input.left      = pressed,
                        KeyCode::KeyD  => self.input.right     = pressed,
                        KeyCode::ArrowUp    => self.input.forward   = pressed,
                        KeyCode::ArrowDown  => self.input.backward  = pressed,
                        KeyCode::ArrowLeft  => self.input.left      = pressed,
                        KeyCode::ArrowRight => self.input.right     = pressed,
                        // NOTE: Space removed — shooting now uses left mouse button
                        KeyCode::Escape => event_loop.exit(),
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left && state == ElementState::Pressed {
                    self.input.shoot = true;
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        // Always accept DeviceEvent mouse motion as a source of mouse look (fractional preserved)
        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            // log occasionally
            let _ = std::fs::write("/tmp/simple_quake_mouse.log",
                format!("dev_event dx={:.3} dy={:.3}\n", delta.0, delta.1));
            self.mouse_accum.add_f64(delta.0, delta.1);
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}



