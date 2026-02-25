use std::sync::Arc;

use cgmath::{perspective, Deg, EuclideanSpace, InnerSpace, Matrix4, Point3, Vector3};
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;

#[derive(Clone)]
struct Enemy {
    position: Vector3<f32>,
    alive: bool,
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
}

struct Input {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    shoot: bool,
    mouse_dx: f32,
    mouse_dy: f32,
    last_mouse_x: f64,
    last_mouse_y: f64,
    first_mouse: bool,
}

impl Input {
    fn new() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            shoot: false,
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
            first_mouse: true,
        }
    }
}

struct App {
    window: Option<Arc<Window>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    camera: Camera,
    input: Input,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    index_count: u32,
    camera_buffer: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
    pipeline: Option<wgpu::RenderPipeline>,
    enemies: Vec<Enemy>,
    bullets: Vec<Bullet>,
    ui_pipeline: Option<wgpu::RenderPipeline>,
    ui_vertex_buffer: Option<wgpu::Buffer>,
    ui_index_buffer: Option<wgpu::Buffer>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            device: None,
            queue: None,
            surface: None,
            camera: Camera::new(),
            input: Input::new(),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
            camera_buffer: None,
            bind_group: None,
            pipeline: None,
            enemies: vec![Enemy {
                position: Vector3::new(0.0, 1.0, -10.0),
                alive: true,
            }],
            bullets: Vec::new(),
            ui_pipeline: None,
            ui_vertex_buffer: None,
            ui_index_buffer: None,
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

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            desired_maximum_frame_latency: 1,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // Floor vertices (blue)
        let vertices: Vec<[f32; 6]> = vec![
            // Floor (blue) - large quad
            [-50.0, 0.0, -50.0, 0.0, 0.0, 1.0],
            [50.0, 0.0, -50.0, 0.0, 0.0, 1.0],
            [50.0, 0.0, 50.0, 0.0, 0.0, 1.0],
            [-50.0, 0.0, -50.0, 0.0, 0.0, 1.0],
            [50.0, 0.0, 50.0, 0.0, 0.0, 1.0],
            [-50.0, 0.0, 50.0, 0.0, 0.0, 1.0],
            // Cube 1 (red) - at (0, 1, -10)
            // Front
            [-1.0, 0.0, -9.0, 1.0, 0.0, 0.0],
            [1.0, 0.0, -9.0, 1.0, 0.0, 0.0],
            [1.0, 2.0, -9.0, 1.0, 0.0, 0.0],
            [-1.0, 0.0, -9.0, 1.0, 0.0, 0.0],
            [1.0, 2.0, -9.0, 1.0, 0.0, 0.0],
            [-1.0, 2.0, -9.0, 1.0, 0.0, 0.0],
            // Back
            [-1.0, 0.0, -11.0, 0.5, 0.0, 0.0],
            [-1.0, 2.0, -11.0, 0.5, 0.0, 0.0],
            [1.0, 2.0, -11.0, 0.5, 0.0, 0.0],
            [-1.0, 0.0, -11.0, 0.5, 0.0, 0.0],
            [1.0, 2.0, -11.0, 0.5, 0.0, 0.0],
            [1.0, 0.0, -11.0, 0.5, 0.0, 0.0],
            // Top
            [-1.0, 2.0, -9.0, 0.7, 0.0, 0.0],
            [1.0, 2.0, -9.0, 0.7, 0.0, 0.0],
            [1.0, 2.0, -11.0, 0.7, 0.0, 0.0],
            [-1.0, 2.0, -9.0, 0.7, 0.0, 0.0],
            [1.0, 2.0, -11.0, 0.7, 0.0, 0.0],
            [-1.0, 2.0, -11.0, 0.7, 0.0, 0.0],
            // Right
            [1.0, 0.0, -9.0, 0.6, 0.0, 0.0],
            [1.0, 0.0, -11.0, 0.6, 0.0, 0.0],
            [1.0, 2.0, -11.0, 0.6, 0.0, 0.0],
            [1.0, 0.0, -9.0, 0.6, 0.0, 0.0],
            [1.0, 2.0, -11.0, 0.6, 0.0, 0.0],
            [1.0, 2.0, -9.0, 0.6, 0.0, 0.0],
            // Left
            [-1.0, 0.0, -11.0, 0.8, 0.0, 0.0],
            [-1.0, 0.0, -9.0, 0.8, 0.0, 0.0],
            [-1.0, 2.0, -9.0, 0.8, 0.0, 0.0],
            [-1.0, 0.0, -11.0, 0.8, 0.0, 0.0],
            [-1.0, 2.0, -9.0, 0.8, 0.0, 0.0],
            [-1.0, 2.0, -11.0, 0.8, 0.0, 0.0],
        ];

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

        // Create UI pipeline for crosshairs and gun
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

        // Crosshair vertices (screen space coordinates)
        let ui_vertices: Vec<[f32; 5]> = vec![
            // Crosshair - horizontal line
            [-0.02, 0.0, 1.0, 1.0, 1.0],
            [0.02, 0.0, 1.0, 1.0, 1.0],
            // Crosshair - vertical line
            [0.0, -0.02, 1.0, 1.0, 1.0],
            [0.0, 0.02, 1.0, 1.0, 1.0],
            // Gun rectangle (bottom right)
            [0.7, -0.5, 0.3, 0.3, 0.3],
            [0.9, -0.5, 0.3, 0.3, 0.3],
            [0.9, -0.5, 0.3, 0.3, 0.3],
            [0.9, -0.9, 0.3, 0.3, 0.3],
            [0.9, -0.9, 0.3, 0.3, 0.3],
            [0.7, -0.9, 0.3, 0.3, 0.3],
            [0.7, -0.9, 0.3, 0.3, 0.3],
            [0.7, -0.5, 0.3, 0.3, 0.3],
            // Gun barrel
            [0.75, -0.5, 0.5, 0.5, 0.5],
            [0.75, -0.3, 0.5, 0.5, 0.5],
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

        // Now assign everything to self
        self.window = Some(window);
        self.device = Some(device);
        self.queue = Some(queue);
        self.surface = Some(surface);
        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.index_count = vertices.len() as u32;
        self.camera_buffer = Some(camera_buffer);
        self.bind_group = Some(bind_group);
        self.pipeline = Some(pipeline);
        self.ui_pipeline = Some(ui_pipeline);
        self.ui_vertex_buffer = Some(ui_vertex_buffer);
        self.ui_index_buffer = Some(ui_index_buffer);
    }

    fn update(&mut self) {
        let speed = 0.1;

        // Movement - standard FPS controls
        let forward = Vector3::new(-self.camera.yaw.sin(), 0.0, -self.camera.yaw.cos());
        let right = Vector3::new(self.camera.yaw.cos(), 0.0, -self.camera.yaw.sin());

        if self.input.forward {
            self.camera.position += forward * speed;
        }
        if self.input.backward {
            self.camera.position -= forward * speed;
        }
        if self.input.left {
            self.camera.position -= right * speed;
        }
        if self.input.right {
            self.camera.position += right * speed;
        }

        // Mouse look
        self.camera.yaw -= self.input.mouse_dx * 0.002;  // Negative for natural mouse look
        self.camera.pitch -= self.input.mouse_dy * 0.002;  // Negative for natural mouse look
        self.camera.pitch = self.camera.pitch.clamp(-1.5, 1.5);

        self.input.mouse_dx = 0.0;
        self.input.mouse_dy = 0.0;

        // Shooting
        if self.input.shoot {
            let forward = self.camera.get_forward_vector().normalize();
            self.bullets.push(Bullet {
                position: self.camera.position,
                direction: forward,
                lifetime: 5.0,
            });
            self.input.shoot = false;
        }

        // Update bullets
        let bullet_speed = 0.5;
        self.bullets.retain_mut(|bullet| {
            bullet.position += bullet.direction * bullet_speed;
            bullet.lifetime -= 0.016; // Approximate frame time
            bullet.lifetime > 0.0
        });

        // Check bullet-enemy collisions
        for bullet in &self.bullets {
            for enemy in &mut self.enemies {
                if !enemy.alive {
                    continue;
                }
                let dist = (bullet.position - enemy.position).magnitude();
                if dist < 1.5 {
                    enemy.alive = false;
                    println!("Enemy destroyed!");
                }
            }
        }
    }

    fn render(&mut self) {
        let device = self.device.as_ref().unwrap();
        let queue = self.queue.as_ref().unwrap();
        let surface = self.surface.as_ref().unwrap();
        let pipeline = self.pipeline.as_ref().unwrap();
        let bind_group = self.bind_group.as_ref().unwrap();
        let camera_buffer = self.camera_buffer.as_ref().unwrap();

        let vp = self.camera.get_view_projection();
        let vp_arr: [[f32; 4]; 4] = vp.into();
        let vp_flat: [f32; 16] = [
            vp_arr[0][0],
            vp_arr[0][1],
            vp_arr[0][2],
            vp_arr[0][3],
            vp_arr[1][0],
            vp_arr[1][1],
            vp_arr[1][2],
            vp_arr[1][3],
            vp_arr[2][0],
            vp_arr[2][1],
            vp_arr[2][2],
            vp_arr[2][3],
            vp_arr[3][0],
            vp_arr[3][1],
            vp_arr[3][2],
            vp_arr[3][3],
        ];
        queue.write_buffer(camera_buffer, 0, bytemuck::cast_slice(&[vp_flat]));

        // Build dynamic geometry for enemies and bullets
        let mut vertices: Vec<[f32; 6]> = vec![
            // Floor (blue) - large quad
            [-50.0, 0.0, -50.0, 0.0, 0.0, 1.0],
            [50.0, 0.0, -50.0, 0.0, 0.0, 1.0],
            [50.0, 0.0, 50.0, 0.0, 0.0, 1.0],
            [-50.0, 0.0, -50.0, 0.0, 0.0, 1.0],
            [50.0, 0.0, 50.0, 0.0, 0.0, 1.0],
            [-50.0, 0.0, 50.0, 0.0, 0.0, 1.0],
        ];

        // Add alive enemies
        for enemy in &self.enemies {
            if !enemy.alive {
                continue;
            }
            let pos = enemy.position;
            let enemy_verts = vec![
                // Front
                [pos.x - 1.0, pos.y - 1.0, pos.z + 1.0, 1.0, 0.0, 0.0],
                [pos.x + 1.0, pos.y - 1.0, pos.z + 1.0, 1.0, 0.0, 0.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z + 1.0, 1.0, 0.0, 0.0],
                [pos.x - 1.0, pos.y - 1.0, pos.z + 1.0, 1.0, 0.0, 0.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z + 1.0, 1.0, 0.0, 0.0],
                [pos.x - 1.0, pos.y + 1.0, pos.z + 1.0, 1.0, 0.0, 0.0],
                // Back
                [pos.x - 1.0, pos.y - 1.0, pos.z - 1.0, 0.5, 0.0, 0.0],
                [pos.x - 1.0, pos.y + 1.0, pos.z - 1.0, 0.5, 0.0, 0.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z - 1.0, 0.5, 0.0, 0.0],
                [pos.x - 1.0, pos.y - 1.0, pos.z - 1.0, 0.5, 0.0, 0.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z - 1.0, 0.5, 0.0, 0.0],
                [pos.x + 1.0, pos.y - 1.0, pos.z - 1.0, 0.5, 0.0, 0.0],
                // Top
                [pos.x - 1.0, pos.y + 1.0, pos.z + 1.0, 0.7, 0.0, 0.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z + 1.0, 0.7, 0.0, 0.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z - 1.0, 0.7, 0.0, 0.0],
                [pos.x - 1.0, pos.y + 1.0, pos.z + 1.0, 0.7, 0.0, 0.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z - 1.0, 0.7, 0.0, 0.0],
                [pos.x - 1.0, pos.y + 1.0, pos.z - 1.0, 0.7, 0.0, 0.0],
                // Right
                [pos.x + 1.0, pos.y - 1.0, pos.z + 1.0, 0.6, 0.0, 0.0],
                [pos.x + 1.0, pos.y - 1.0, pos.z - 1.0, 0.6, 0.0, 0.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z - 1.0, 0.6, 0.0, 0.0],
                [pos.x + 1.0, pos.y - 1.0, pos.z + 1.0, 0.6, 0.0, 0.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z - 1.0, 0.6, 0.0, 0.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z + 1.0, 0.6, 0.0, 0.0],
                // Left
                [pos.x - 1.0, pos.y - 1.0, pos.z - 1.0, 0.8, 0.0, 0.0],
                [pos.x - 1.0, pos.y - 1.0, pos.z + 1.0, 0.8, 0.0, 0.0],
                [pos.x - 1.0, pos.y + 1.0, pos.z + 1.0, 0.8, 0.0, 0.0],
                [pos.x - 1.0, pos.y - 1.0, pos.z - 1.0, 0.8, 0.0, 0.0],
                [pos.x - 1.0, pos.y + 1.0, pos.z + 1.0, 0.8, 0.0, 0.0],
                [pos.x - 1.0, pos.y + 1.0, pos.z - 1.0, 0.8, 0.0, 0.0],
            ];
            vertices.extend(enemy_verts);
        }

        // Add bullets (small yellow cubes)
        for bullet in &self.bullets {
            let pos = bullet.position;
            let size = 0.1;
            let bullet_verts = vec![
                // Front
                [pos.x - size, pos.y - size, pos.z + size, 1.0, 1.0, 0.0],
                [pos.x + size, pos.y - size, pos.z + size, 1.0, 1.0, 0.0],
                [pos.x + size, pos.y + size, pos.z + size, 1.0, 1.0, 0.0],
                [pos.x - size, pos.y - size, pos.z + size, 1.0, 1.0, 0.0],
                [pos.x + size, pos.y + size, pos.z + size, 1.0, 1.0, 0.0],
                [pos.x - size, pos.y + size, pos.z + size, 1.0, 1.0, 0.0],
                // Back
                [pos.x - size, pos.y - size, pos.z - size, 0.8, 0.8, 0.0],
                [pos.x - size, pos.y + size, pos.z - size, 0.8, 0.8, 0.0],
                [pos.x + size, pos.y + size, pos.z - size, 0.8, 0.8, 0.0],
                [pos.x - size, pos.y - size, pos.z - size, 0.8, 0.8, 0.0],
                [pos.x + size, pos.y + size, pos.z - size, 0.8, 0.8, 0.0],
                [pos.x + size, pos.y - size, pos.z - size, 0.8, 0.8, 0.0],
                // Top
                [pos.x - size, pos.y + size, pos.z + size, 0.9, 0.9, 0.0],
                [pos.x + size, pos.y + size, pos.z + size, 0.9, 0.9, 0.0],
                [pos.x + size, pos.y + size, pos.z - size, 0.9, 0.9, 0.0],
                [pos.x - size, pos.y + size, pos.z + size, 0.9, 0.9, 0.0],
                [pos.x + size, pos.y + size, pos.z - size, 0.9, 0.9, 0.0],
                [pos.x - size, pos.y + size, pos.z - size, 0.9, 0.9, 0.0],
                // Bottom
                [pos.x - size, pos.y - size, pos.z - size, 0.7, 0.7, 0.0],
                [pos.x + size, pos.y - size, pos.z - size, 0.7, 0.7, 0.0],
                [pos.x + size, pos.y - size, pos.z + size, 0.7, 0.7, 0.0],
                [pos.x - size, pos.y - size, pos.z - size, 0.7, 0.7, 0.0],
                [pos.x + size, pos.y - size, pos.z + size, 0.7, 0.7, 0.0],
                [pos.x - size, pos.y - size, pos.z + size, 0.7, 0.7, 0.0],
                // Right
                [pos.x + size, pos.y - size, pos.z + size, 0.85, 0.85, 0.0],
                [pos.x + size, pos.y - size, pos.z - size, 0.85, 0.85, 0.0],
                [pos.x + size, pos.y + size, pos.z - size, 0.85, 0.85, 0.0],
                [pos.x + size, pos.y - size, pos.z + size, 0.85, 0.85, 0.0],
                [pos.x + size, pos.y + size, pos.z - size, 0.85, 0.85, 0.0],
                [pos.x + size, pos.y + size, pos.z + size, 0.85, 0.85, 0.0],
                // Left
                [pos.x - size, pos.y - size, pos.z - size, 0.75, 0.75, 0.0],
                [pos.x - size, pos.y - size, pos.z + size, 0.75, 0.75, 0.0],
                [pos.x - size, pos.y + size, pos.z + size, 0.75, 0.75, 0.0],
                [pos.x - size, pos.y - size, pos.z - size, 0.75, 0.75, 0.0],
                [pos.x - size, pos.y + size, pos.z + size, 0.75, 0.75, 0.0],
                [pos.x - size, pos.y + size, pos.z - size, 0.75, 0.75, 0.0],
            ];
            vertices.extend(bullet_verts);
        }

        // Recreate vertex/index buffers with dynamic data
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
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.2,
                            a: 1.0,
                        }),
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

        // Render UI (crosshairs and gun)
        {
            let ui_pipeline = self.ui_pipeline.as_ref().unwrap();
            let ui_vertex_buffer = self.ui_vertex_buffer.as_ref().unwrap();
            let ui_index_buffer = self.ui_index_buffer.as_ref().unwrap();

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
            pass.draw_indexed(0..14, 0, 0..1); // 14 vertices for crosshair + gun
        }

        queue.submit([encoder.finish()]);
        output.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::Window::default_attributes()
                        .with_title("Simple Quake")
                        .with_inner_size(winit::dpi::PhysicalSize::new(
                            WINDOW_WIDTH,
                            WINDOW_HEIGHT,
                        )),
                )
                .unwrap(),
        );

        let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined);
        window.set_cursor_visible(false);

        self.init_rendering(window);
        event_loop.set_control_flow(ControlFlow::Poll);

        println!("=== SIMPLE QUAKE ===");
        println!("WASD: Move | Mouse: Look | Left Click: Shoot | ESC: Quit");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                // Could resize here
            }
            WindowEvent::RedrawRequested => {
                self.update();
                self.render();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state,
                        ..
                    },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                if let PhysicalKey::Code(code) = physical_key {
                    match code {
                        KeyCode::KeyW => self.input.forward = pressed,
                        KeyCode::KeyS => self.input.backward = pressed,
                        KeyCode::KeyA => self.input.left = pressed,
                        KeyCode::KeyD => self.input.right = pressed,
                        KeyCode::Escape => event_loop.exit(),
                        _ => {}
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.input.first_mouse {
                    self.input.last_mouse_x = position.x;
                    self.input.last_mouse_y = position.y;
                    self.input.first_mouse = false;
                }

                self.input.mouse_dx = (position.x - self.input.last_mouse_x) as f32;
                self.input.mouse_dy = (position.y - self.input.last_mouse_y) as f32;
                self.input.last_mouse_x = position.x;
                self.input.last_mouse_y = position.y;
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
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
