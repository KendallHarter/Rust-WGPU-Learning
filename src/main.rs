struct State<'window> {
   surface: wgpu::Surface<'window>,
   device: wgpu::Device,
   queue: wgpu::Queue,
   config: wgpu::SurfaceConfiguration,
   size: winit::dpi::PhysicalSize<u32>,
   render_pipeline: wgpu::RenderPipeline,
   texture_bind_group: wgpu::BindGroup,
   tiles_buffer: wgpu::Buffer,
   tiles_bind_group: wgpu::BindGroup,
   tiles: [[u32; 4]; 1024],
   frame_times: Vec<f64>,
}

const NUM_TILES: u32 = 1024;
const TILE_WIDTH: u32 = 8;
const TILE_HEIGHT: u32 = 8;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src"]
struct ScriptAssets;

impl<'window> State<'window> {
   async fn new(window: &'window winit::window::Window) -> Self {
      let size = window.inner_size();

      let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
         backends: wgpu::Backends::all(),
         ..Default::default()
      });

      let surface = instance.create_surface(window).unwrap();

      let adapter = instance
         .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
         })
         .await
         .unwrap();

      let (device, queue) = adapter
         .request_device(
            &wgpu::DeviceDescriptor {
               label: None,
               required_features: wgpu::Features::empty(),
               required_limits: wgpu::Limits::default(),
            },
            None,
         )
         .await
         .unwrap();

      let surface_caps = surface.get_capabilities(&adapter);
      // Get sRGB color space (standard space)
      let surface_format = surface_caps
         .formats
         .iter()
         .find(|&f| f.is_srgb())
         .unwrap_or(&surface_caps.formats[0]);

      let config = wgpu::SurfaceConfiguration {
         usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
         format: *surface_format,
         width: size.width,
         height: size.height,
         present_mode: surface_caps.present_modes[0],
         alpha_mode: surface_caps.alpha_modes[0],
         view_formats: vec![],
         desired_maximum_frame_latency: 2,
      };
      surface.configure(&device, &config);

      let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
         label: Some("Shader"),
         source: wgpu::ShaderSource::Wgsl(
            String::from_utf8_lossy(&ScriptAssets::get("shader.wgsl").unwrap().data.into_owned()).into(),
         ),
      });

      let image_data = image::load_from_memory(include_bytes!("../assets/bg_pal0/font.png")).unwrap();
      let image_bytes = image_data.to_rgba8().into_vec();

      let texture_size = wgpu::Extent3d {
         width: TILE_WIDTH * NUM_TILES,
         height: TILE_HEIGHT,
         depth_or_array_layers: 1,
      };

      let texture = device.create_texture(&wgpu::TextureDescriptor {
         size: texture_size,
         mip_level_count: 1,
         sample_count: 1,
         dimension: wgpu::TextureDimension::D2,
         format: wgpu::TextureFormat::Rgba8UnormSrgb,
         usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
         label: Some("texture"),
         view_formats: &[],
      });

      let offset_per_iter = image_data.width() * 4 * TILE_HEIGHT;
      for i in 0..image_data.height() / 8 {
         queue.write_texture(
            wgpu::ImageCopyTexture {
               texture: &texture,
               mip_level: 0,
               origin: wgpu::Origin3d {
                  x: i * image_data.width(),
                  y: 0,
                  z: 0,
               },
               aspect: wgpu::TextureAspect::All,
            },
            &image_bytes[(offset_per_iter * i) as usize..],
            wgpu::ImageDataLayout {
               offset: 0,
               bytes_per_row: Some(image_data.width() * 4),
               rows_per_image: Some(TILE_HEIGHT),
            },
            wgpu::Extent3d {
               width: image_data.width(),
               height: 8,
               depth_or_array_layers: 1,
            },
         );
      }

      let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
      let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
         address_mode_u: wgpu::AddressMode::ClampToEdge,
         address_mode_v: wgpu::AddressMode::ClampToEdge,
         address_mode_w: wgpu::AddressMode::ClampToEdge,
         mag_filter: wgpu::FilterMode::Nearest,
         min_filter: wgpu::FilterMode::Nearest,
         mipmap_filter: wgpu::FilterMode::Nearest,
         ..Default::default()
      });

      let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
         entries: &[
            wgpu::BindGroupLayoutEntry {
               binding: 0,
               visibility: wgpu::ShaderStages::FRAGMENT,
               ty: wgpu::BindingType::Texture {
                  multisampled: false,
                  view_dimension: wgpu::TextureViewDimension::D2,
                  sample_type: wgpu::TextureSampleType::Float { filterable: true },
               },
               count: None,
            },
            wgpu::BindGroupLayoutEntry {
               binding: 1,
               visibility: wgpu::ShaderStages::FRAGMENT,
               ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
               count: None,
            },
         ],
         label: Some("texture_bind_group_layout"),
      });

      let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
         layout: &texture_bind_group_layout,
         entries: &[
            wgpu::BindGroupEntry {
               binding: 0,
               resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
               binding: 1,
               resource: wgpu::BindingResource::Sampler(&sampler),
            },
         ],
         label: Some("texture_bind_group"),
      });

      let tiles_buffer = device.create_buffer(&wgpu::BufferDescriptor {
         label: Some("Tiles Buffer"),
         size: (NUM_TILES * 4 * 4) as u64,
         usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
         mapped_at_creation: false,
      });

      let tiles_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
         label: Some("tiles_bind_group"),
      });

      let tiles_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
         layout: &tiles_bind_group_layout,
         entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: tiles_buffer.as_entire_binding(),
         }],
         label: Some("tiles_bind_group"),
      });

      let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
         label: Some("Render Pipeline Layout"),
         bind_group_layouts: &[&texture_bind_group_layout, &tiles_bind_group_layout],
         push_constant_ranges: &[],
      });

      let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
         label: Some("Render Pipeline"),
         layout: Some(&render_pipeline_layout),
         vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
         },
         fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
               format: config.format,
               blend: Some(wgpu::BlendState::REPLACE),
               write_mask: wgpu::ColorWrites::ALL,
            })],
         }),
         primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Cw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
         },
         depth_stencil: None,
         multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
         },
         multiview: None,
      });

      Self {
         surface,
         device,
         queue,
         config,
         size,
         render_pipeline,
         texture_bind_group,
         tiles_buffer,
         tiles_bind_group,
         tiles: [[0; 4]; 1024],
         frame_times: vec![],
      }
   }

   fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
      if size.width > 0 && size.height > 0 {
         self.size = size;
         self.config.width = self.size.width;
         self.config.height = self.size.height;
         self.surface.configure(&self.device, &self.config);
      }
   }

   fn render(&mut self, frametime: f64) -> Result<(), wgpu::SurfaceError> {
      if self.frame_times.len() == 100 {
         self.frame_times.rotate_left(1);
         self.frame_times[99] = frametime;
      } else {
         self.frame_times.push(frametime);
      }

      let output = self.surface.get_current_texture()?;
      let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
      let mut encoder = self
         .device
         .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render") });

      // Clear out any potential old text
      for i in self.tiles.as_mut_slice() {
         i[0] = 0;
      }

      let avg_frame_time = self.frame_times.iter().sum::<f64>() / self.frame_times.len() as f64;
      let avg_frame_rate = format!("{}", 1.0 / avg_frame_time);
      for (i, c) in avg_frame_rate.as_bytes().iter().enumerate() {
         self.tiles[i][0] = *c as u32;
      }

      self.queue.write_buffer(&self.tiles_buffer, 0, &unsafe {
         std::mem::transmute::<[[u32; 4]; 1024], [u8; 1024 * 4 * 4]>(self.tiles)
      });

      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
         label: Some("Render"),
         color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
               load: wgpu::LoadOp::Clear(wgpu::Color {
                  r: 0.5,
                  g: 0.5,
                  b: 0.5,
                  a: 1.0,
               }),
               store: wgpu::StoreOp::Store,
            },
         })],
         depth_stencil_attachment: None,
         occlusion_query_set: None,
         timestamp_writes: None,
      });

      render_pass.set_pipeline(&self.render_pipeline);
      render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
      render_pass.set_bind_group(1, &self.tiles_bind_group, &[]);

      render_pass.draw(0..32 * 32 * 6, 0..1);

      // Force render
      drop(render_pass);

      self.queue.submit(std::iter::once(encoder.finish()));
      output.present();

      Ok(())
   }
}

pub fn run() {
   const WINDOW_SIZE: winit::dpi::PhysicalSize<u32> = winit::dpi::PhysicalSize::<u32>::new(240, 160);
   env_logger::init();
   let event_loop = winit::event_loop::EventLoop::new().unwrap();
   let window = winit::window::WindowBuilder::new()
      .with_inner_size(WINDOW_SIZE)
      .build(&event_loop)
      .unwrap();
   event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

   let mut state = pollster::block_on(State::new(&window));

   let mut frame_start_time = std::time::Instant::now();
   let mut frame_time = 1.0;

   event_loop
      .run(|event, elwt| {
         use winit::event::{Event, WindowEvent};
         match event {
            Event::WindowEvent { window_id, ref event } if window_id == window.id() => match event {
               WindowEvent::CloseRequested => elwt.exit(),
               WindowEvent::RedrawRequested => match state.render(frame_time) {
                  Ok(_) => {}
                  Err(wgpu::SurfaceError::Lost) => {
                     state.resize(state.size);
                  }
                  Err(wgpu::SurfaceError::OutOfMemory) => {
                     elwt.exit();
                  }
                  Err(_) => {}
               },
               WindowEvent::Resized(new_size) => {
                  state.resize(*new_size);
               }
               _ => {}
            },
            Event::AboutToWait => {
               if frame_start_time.elapsed() >= std::time::Duration::from_secs_f64(1.0 / 59.7275) {
                  frame_time = frame_start_time.elapsed().as_secs_f64();
                  frame_start_time = std::time::Instant::now();
                  window.request_redraw();
               } else {
                  let time_to_sleep_to = frame_start_time + std::time::Duration::from_secs_f64(1.0 / 59.7275);
                  spin_sleep::sleep(time_to_sleep_to - std::time::Instant::now());
               }
            }
            _ => {}
         }
      })
      .unwrap();
}

fn main() {
   run()
}
