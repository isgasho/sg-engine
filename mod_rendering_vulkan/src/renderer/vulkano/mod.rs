
pub mod vertex;
use self::vertex::Vertex;

pub mod vulkano_win_patch;

use vulkano;
use cgmath;
use game_state::winit;


use vulkano::buffer::BufferUsage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::command_buffer::DynamicState;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::descriptor::descriptor_set::{
    PersistentDescriptorSet,
    PersistentDescriptorSetBuf,
    PersistentDescriptorSetImg
};
use vulkano::descriptor::pipeline_layout::PipelineLayoutAbstract;
use vulkano::device::Device;
use vulkano::framebuffer::Framebuffer;
use vulkano::framebuffer::Subpass;
use vulkano::instance::Instance;
use vulkano::instance::PhysicalDevice;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::depth_stencil::DepthStencil;
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::viewport::Scissor;
use vulkano::swapchain;
use vulkano::swapchain::SurfaceTransform;
use vulkano::swapchain::Surface;
use vulkano::swapchain::Swapchain;
use vulkano::pipeline::input_assembly::PrimitiveTopology;

use vulkano::sync::now;

use vulkano::instance::debug::DebugCallback;

use vulkano::image::attachment::AttachmentImage;
use vulkano::image::{
    StorageImage,
    SwapchainImage,
    ImageViewAccess,
    ImageAccess,
    ImageUsage,
};

//use vulkano::device::QueuesIter;
use vulkano::device::Queue;
use vulkano::sync::GpuFuture;
use vulkano::descriptor::pipeline_layout::{
    PipelineLayout,
    PipelineLayoutDescUnion,
};

use vulkano::framebuffer::{
    RenderPassAbstract,
    FramebufferAbstract
};

use vulkano::pipeline::raster::{
    Rasterization,
    PolygonMode,
    CullMode,
    FrontFace,
    DepthBiasControl
};

// FIXME ju,k.u.m.[yu;j.7;i;.jk.7;.;;li
// k66kj,ku,,777777777777777777777777777777777777

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::mem;

use std::collections::VecDeque;
use std::collections::hash_map::HashMap;

use game_state;
use game_state::utils::fps;
use game_state::{Identity, Identifyable, Renderer};
use game_state::input::InputSource;
use game_state::tree::{ BreadthFirstIterator };
use game_state::state::SceneGraph;
use game_state::state::DrawMode;

use image;

//TODO: compile these elsewhere, at build time?
// These shaders are a PITA, generated by build.rs, dependent on OUT_DIR... *barf
// More importantly, these are actually compiled SPIR-V, ignore the glsl file extension on them
mod vs { include!{concat!(env!("OUT_DIR"), "/assets/shaders/vs.glsl") }}
mod fs { include!{concat!(env!("OUT_DIR"), "/assets/shaders/fs.glsl") }}

pub struct BufferItem {
    pub vertices: Arc<CpuAccessibleBuffer<[Vertex]>>,
    pub indices: Arc<CpuAccessibleBuffer<[u16]>>,
    pub diffuse_map: Arc<CpuAccessibleBuffer<[[u8;4]]>>,
}

type ThisPipelineType =
    GraphicsPipeline<
        SingleBufferDefinition<::renderer::vulkano::vertex::Vertex>,
        Box<PipelineLayoutAbstract + Send + Sync>,
        Arc<vulkano::framebuffer::RenderPassAbstract + Send + Sync>
    >;

type ThisPipelineDescriptorSet =
        PersistentDescriptorSet<
            Arc<ThisPipelineType>,
            (
                (
                    (
                        (),
                        PersistentDescriptorSetImg<Arc<StorageImage<vulkano::format::R8G8B8A8Unorm>>>
                    ),
                    vulkano::descriptor::descriptor_set::PersistentDescriptorSetSampler
                ),
                PersistentDescriptorSetBuf<
                    Arc<
                        CpuAccessibleBuffer<
                            ::renderer::vulkano::vs::ty::Data
                        >
                    >
                >
            ),
            vulkano::descriptor::descriptor_set::StdDescriptorPoolAlloc
        >;

type AMWin = Arc<winit::Window>;
pub struct VulkanoRenderer {
    id: Identity,
    _instance: Arc<Instance>,

    #[allow(dead_code)]
    window: AMWin,

    #[allow(dead_code)]
    surface: Arc<Surface<AMWin>>,

    events_loop: Arc<Mutex<winit::EventsLoop>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain<AMWin>>,
    _images: Vec<Arc<SwapchainImage<AMWin>>>,
    pipeline: Arc<ThisPipelineType>,
    framebuffers: Vec<Arc<FramebufferAbstract + Send + Sync>>,
    texture: Arc<vulkano::image::StorageImage<vulkano::format::R8G8B8A8Unorm>>,
    fps: fps::FPS,

    _renderpass: Arc<RenderPassAbstract + Send + Sync>,

    // descriptor set TODO: move this to BufferItem, so it can be associated with a mesh?
    pipeline_set: Arc<ThisPipelineDescriptorSet>,

    _uniform_buffer: Arc<CpuAccessibleBuffer<::renderer::vulkano::vs::ty::Data>>,
    render_layer_queue: VecDeque<Arc<SceneGraph>>,
    buffer_cache: HashMap<usize, BufferItem>,

    rect: ScreenRect,
    current_mouse_pos: ScreenPoint,
    debug_world_rotation: f32,
    debug_zoom: f32,

    // Enable vulkan debug layers?
    #[allow(dead_code)]
    debug_callback: Option<vulkano::instance::debug::DebugCallback>,

    previous_frame_end: Box<GpuFuture>,
    recreate_swapchain: bool,
    hack_uploaded_tex: bool,
    dynamic_state: DynamicState,
}

impl VulkanoRenderer {

    fn create_swapchain(
        surface: Arc<Surface<AMWin>>,
        device: Arc<Device>,
        queue: Arc<Queue>,
        physical: PhysicalDevice
    ) -> Result<(Arc<Swapchain<AMWin>>, Vec<Arc<SwapchainImage<AMWin>>>), String> {
        let caps = match surface.capabilities(physical.clone()) {
            Ok(caps) => caps,
            Err(err) => {
                return Err(format!("Unable to get capabilities from surface: {:?}", err).to_string())
            }
        };
        let dimensions = caps.current_extent.unwrap_or([1280, 800]);
        let present = caps.present_modes.iter().next().unwrap();
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;
        Ok(Swapchain::new(
            device,
            surface,
            caps.min_image_count,
            format,
            dimensions,
            1,
            caps.supported_usage_flags,
            &queue,
            SurfaceTransform::Identity,
            alpha,
            vulkano::swapchain::PresentMode::Fifo,
            true,
            None
        ).expect("Failed to create swapchain."))
    }

    fn create_descriptor_set(
        texture: Arc<StorageImage<vulkano::format::R8G8B8A8Unorm>>,
        device: Arc<Device>,
        width: u32,
        height: u32,
        uniform_buffer: Arc<CpuAccessibleBuffer<::renderer::vulkano::vs::ty::Data>>,
        queue: Arc<Queue>,
        pipeline: Arc<ThisPipelineType>,
    ) -> Arc<ThisPipelineDescriptorSet> {

        println!("Creating descriptor set...");
        let sampler = vulkano::sampler::Sampler::new(
            device.clone(),
            vulkano::sampler::Filter::Linear,
            vulkano::sampler::Filter::Linear,
            vulkano::sampler::MipmapMode::Nearest,
            vulkano::sampler::SamplerAddressMode::Repeat,
            vulkano::sampler::SamplerAddressMode::Repeat,
            vulkano::sampler::SamplerAddressMode::Repeat,
            0.0, 1.0, 0.0, 0.0
        ).unwrap();

        let ds = PersistentDescriptorSet::start(pipeline, 0)
            .add_sampled_image(texture, sampler)
            .expect("error loading texture")
            .add_buffer(uniform_buffer)
            .expect("error adding uniform buffer")
            .build()
            .unwrap();

        Arc::new(ds)

    }

    // FIXME don't pass a tuple, rather a new struct type that composes these
    pub fn new<'a>(
        window: AMWin,
        events_loop: Arc<Mutex<winit::EventsLoop>>,
        draw_mode: DrawMode
    ) -> Result<Self, String>{

        let instance = {
            let extensions = vulkano_win_patch::required_extensions();
            let app_info = app_info_from_cargo_toml!();
            let i = Instance::new(Some(&app_info), &extensions, None).expect("Failed to create Vulkan instance. ");
            i
        };

        let callback = DebugCallback::errors_and_warnings(
            &instance, |msg| {
                println!("Debug callback: {:?}", msg.description);
            }
        ).ok();

        let physical = vulkano::instance::PhysicalDevice::enumerate(&instance)
            .next().expect("No device available.");

        println!("Creating surface...");
        let surface: Arc<Surface<AMWin>> = unsafe {
           match vulkano_win_patch::winit_to_surface(instance.clone(), window.clone()) {
               Ok(s) => s,
               Err(e) => return Err("unable to create surface..".to_string())
           }
        };

        println!("getting queue");
        let queue = physical.queue_families().find(|q| {
            q.supports_graphics() && surface.is_supported(q.clone()).unwrap_or(false)
        }).expect("Couldn't find a graphical queue family.");

        println!("getting device");
        let (device, mut queues) = {
            let device_ext = vulkano::device::DeviceExtensions {
                khr_swapchain: true,
                .. vulkano::device::DeviceExtensions::none()
            };

            Device::new(physical, physical.supported_features(), &device_ext,
                [(queue, 0.5)].iter().cloned()
            ).expect("Failed to create device.")
        };

        let queue = queues.next().unwrap();

        println!("Creating swapchain...");
        let (swapchain, images) = Self::create_swapchain(surface.clone(), device.clone(), queue.clone(), physical)?;

        // TODO: as part of asset_loader, we should be loading all the shaders we expect to use in a scene
        let vs = vs::Shader::load(device.clone()).expect("failed to create vs shader module");
        let fs = fs::Shader::load(device.clone()).expect("failed to create fs shader module");

        // ----------------------------------
        // Uniform buffer
        // TODO: extract to the notion of a camera
        let proj = cgmath::perspective(
            cgmath::Rad(::std::f32::consts::FRAC_PI_2),
            {
               let d = ImageAccess::dimensions(&images[0]);
               d.width() as f32 / d.height() as f32
            },
            0.01,
            100.0 // depth used for culling!
        );

        // Vulkan uses right-handed coordinates, y positive is down
        let view = cgmath::Matrix4::look_at(
            cgmath::Point3::new(0.0, 0.0, -20.0),   // eye
            cgmath::Point3::new(0.0, 0.0, 0.0),  // center
            cgmath::Vector3::new(0.0, -1.0, 0.0)  // up
        );

        let scale = cgmath::Matrix4::from_scale(1.0);

        let uniform_buffer = CpuAccessibleBuffer::<vs::ty::Data>::from_data(
            device.clone(),
            vulkano::buffer::BufferUsage::all(),
            vs::ty::Data {
                world : <cgmath::Matrix4<f32> as cgmath::SquareMatrix>::identity().into(),
                view : (view * scale).into(),
                proj : proj.into(),
            }).expect("failed to create buffer");
        // ----------------------------------

        let img_usage = ImageUsage {
            transient_attachment: true,
            input_attachment: true,
            ..ImageUsage::none()
        };
        let depth_buffer = AttachmentImage::with_usage(
            device.clone(),
            SwapchainImage::dimensions(&images[0]),
            vulkano::format::D16Unorm,
            img_usage
        ).unwrap();

        #[allow(dead_code)]
        let renderpass = single_pass_renderpass!(device.clone(),
                attachments: {
                    color: {
                        load: Clear,
                        store: Store,
                        format: ImageAccess::format(&images[0]),
                        samples: 1,
                    },
                    depth: {
                        load: Clear,
                        store: Store,
                        format: vulkano::image::ImageAccess::format(&depth_buffer),
                        samples: 1,
                    }
                },
                pass: {
                    color: [color],
                    depth_stencil: {depth}
                }
            ).unwrap();


        let renderpass_arc = Arc::new(renderpass); //as Arc<RenderPassAbstract + Send + Sync>;
        let depth_buffer = Arc::new(depth_buffer);

        let framebuffers = images.iter().map(|image| {
            //let attachments = renderpass_arc.desc().start_attachments()
            //    .color(image.clone()).depth(depth_buffer.clone());
            let dimensions = [ImageAccess::dimensions(image).width(), ImageAccess::dimensions(image).height(), 1];
            let fb =
                Framebuffer::with_dimensions(renderpass_arc.clone(), dimensions)
                .add( image.clone() as Arc<ImageViewAccess + Send + Sync>)
                .unwrap()
                .add( depth_buffer.clone() as Arc<ImageViewAccess + Send + Sync> )
                .unwrap()
                .build()
                .unwrap();
            Arc::new(fb) as Arc<FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>();

        // -----------------------------------------------
        // Rendermodes, fill, lines, points

        let polygonmode = match draw_mode {
            DrawMode::Colored => PolygonMode::Fill,
            DrawMode::Points  => PolygonMode::Point,
            DrawMode::Wireframe => PolygonMode::Line
        };

        let mut raster = Rasterization::default();
        raster.cull_mode = CullMode::Back;
        raster.polygon_mode = polygonmode;
        raster.depth_clamp = true;
        raster.front_face = FrontFace::Clockwise;
        raster.line_width = Some(2.0);
        raster.depth_bias = DepthBiasControl::Dynamic;
        // -------------------------------------------------

        let p = GraphicsPipeline::start()
            .vertex_input_single_buffer()
            .cull_mode_back()
            .polygon_mode_fill()
            .depth_clamp(true)
            .front_face_clockwise()
            .line_width(2.0)
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .depth_stencil_simple_depth()
            .blend_alpha_blending()
            .render_pass(Subpass::from(renderpass_arc.clone() as Arc<RenderPassAbstract + Send + Sync>, 0).unwrap())
            .build(device.clone())
            .unwrap();

        let pipeline = Arc::new(p);

        // TODO: texture sizes?

        let texture = StorageImage::new(
            device.clone(),
            vulkano::image::Dimensions::Dim2d { width: 2048, height: 2048  },
            vulkano::format::R8G8B8A8Unorm,
            Some(queue.family())
        ).unwrap();

        let pipeline_set = Self::create_descriptor_set(
            texture.clone(),
            device.clone(),
            2048,
            2048,
            uniform_buffer.clone(),
            queue.clone(),
            pipeline.clone()
        );

        // finish up by grabbing some initialization values for position and size
        let (x,y) = window.get_position().unwrap_or((0,0));
        let (w,h) = window.get_inner_size_pixels().unwrap_or((0,0));
        // TODO: get actual mouse position... or does it matter at this point when we get it in the
        // event loop instead

        let previous_frame_end = Box::new(now(device.clone())) as Box<GpuFuture>;
        let dimensions = ImageAccess::dimensions(&images[0]);

        Ok(VulkanoRenderer {
            id: game_state::create_next_identity(),
            _instance: instance.clone(),
            window: window,
            surface: surface,
            events_loop: events_loop.clone(),

            device: device,
            //queues: queues,
            queue: queue,
            swapchain: swapchain,
            _images: images,
            pipeline: pipeline,
            framebuffers: framebuffers,
            texture: texture,
            _renderpass: renderpass_arc as Arc<RenderPassAbstract + Send + Sync>,
            pipeline_set: pipeline_set,

            fps: fps::FPS::new(),
            _uniform_buffer: uniform_buffer,
            render_layer_queue: VecDeque::new(),
            buffer_cache: HashMap::new(),


            current_mouse_pos: ScreenPoint::new(0, 0),
            rect: ScreenRect::new(x as i32, y as i32, w as i32, h as i32),
            debug_world_rotation: 0f32,
            debug_zoom: 0f32,

            debug_callback: callback,
            previous_frame_end: previous_frame_end,
            recreate_swapchain: false,
            hack_uploaded_tex: false,
            dynamic_state: DynamicState {
                line_width: None,
                viewports: Some(vec![vulkano::pipeline::viewport::Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [dimensions.width() as f32, dimensions.height() as f32],
                    depth_range: 0.0 .. 1.0,
                }]),
                .. DynamicState::none()
            }

        })

    }

    #[inline]
    fn get_mouse_pos(&self) -> &ScreenPoint { &self.current_mouse_pos }

    #[inline]
    fn set_mouse_pos(&mut self, pos: ScreenPoint) { self.current_mouse_pos = pos; }

    #[allow(dead_code)]
    #[inline] fn get_rect(&self) -> &ScreenRect { &self.rect }

    #[inline]
    fn set_rect(&mut self, new_rect: ScreenRect) {
        // TODO: determine a delta here?
        // TODO: let the renderer know to change things up because we were resized?
        self.rect = new_rect;
    }

    #[inline]
    pub fn insert_buffer(&mut self, id: usize, vertices: &Vec<Vertex>, indices: &Vec<u16>, diffuse_map: &image::DynamicImage) {

        let pixel_buffer = {
            let image = diffuse_map.to_rgba();
            let image_data = image.into_raw().clone();

            let image_data_chunks = image_data.chunks(4).map(|c| [c[0], c[1], c[2], c[3]]);

            // TODO: staging buffer instead
            vulkano::buffer::cpu_access::CpuAccessibleBuffer::<[[u8; 4]]>
                ::from_iter(self.device.clone(), BufferUsage::all(), image_data_chunks)
                .expect("failed to create buffer")
        };

        self.buffer_cache.insert(id,
            BufferItem{
                vertices: CpuAccessibleBuffer::from_iter(
                    self.device.clone(),
                    BufferUsage::all(),
                    vertices.iter().cloned()
                ).expect("Unable to create buffer"),
                indices: CpuAccessibleBuffer::from_iter(
                    self.device.clone(),
                    BufferUsage::all(),
                    indices.iter().cloned()
                ).expect("Unable to create buffer"),
                diffuse_map: pixel_buffer
            }
        );
    }

    fn render(&mut self) {

        &mut self.previous_frame_end.cleanup_finished();

        if self.recreate_swapchain {
            //TODO
            self.recreate_swapchain = false;
            unimplemented!();
        }


        // todo: how are passes organized if textures must be uploaded first?
        // FIXME: use an initialization step rather than this quick hack
        // FIXME: that might look like a new method on Renderer - reload_buffers?

        // todo: re-create swapchain ^kthx
        let (image_num, acquire_future) = swapchain::acquire_next_image(self.swapchain.clone(), None)
                                            .unwrap();

        let mut cmd_buffer_build = AutoCommandBufferBuilder::primary_one_time_submit(
            self.device.clone(),
            self.queue.family()
        ).unwrap(); // catch oom error here

        // THIS MUST HAPPEN OUTSIDE THE RENDER PASS
        if !self.hack_uploaded_tex {
            println!("looking for texture...");
            let maybe_buffer = self.buffer_cache.get(&0usize);
            match maybe_buffer {
                Some(item) => {
                    let &BufferItem { ref diffuse_map, .. } = item;
                    println!("copy_buffer_to_image");
                    cmd_buffer_build = cmd_buffer_build.copy_buffer_to_image(
                        diffuse_map.clone(),
                        self.texture.clone()
                    ).expect("unable to upload texture");
                    self.hack_uploaded_tex = true;
                },
                _ => {
                    println!("unable to find texture");
                }
            }
        }

        cmd_buffer_build = cmd_buffer_build.begin_render_pass(
            self.framebuffers[image_num].clone(), false,
            vec![
                vulkano::format::ClearValue::from([ 0.15, 0.15, 0.15, 1.0 ]),
                vulkano::format::ClearValue::Depth(1.0)
            ]
        ).expect("unable to begin renderpass");

        loop {

            // TODO: implement a notion of a camera
            // TODO: that might be best done through the uniform_buffer, as it's what owns the
            // TODO: projection matrix at this point

            self.debug_world_rotation += 0.01;
            match self.render_layer_queue.pop_front() {
                Some(next_layer) => {

                    // TODO: load assets through mod_asset_loader, put into State
                    // TODO: refactor this to use asset lookups
                    // TODO: refactor this to use WorldEntity collection -> SceneGraph Rc types
                    // TODO: asset lookups should store DescriptorSets with associated textures

                    let iterator = BreadthFirstIterator::new(next_layer.root.clone());
                    for (_node_id, rc) in iterator {
                        let mut node = &mut rc.borrow_mut();

                        let model_mat = node.data.get_model_matrix().clone();
                        let rotation = cgmath::Matrix4::from_angle_y(cgmath::Rad(self.debug_world_rotation));
                        let rot_model = model_mat * rotation;
                        // TODO: updating the world matrices from the parent * child's local matrix
                        match node.parent() {
                            Some(parent) => {
                                let ref parent_model = parent.borrow().data;
                                let global_mat = parent_model.get_world_matrix() * rot_model;
                                node.data.set_world_matrix(global_mat);
                            },
                            None => {
                                node.data.set_world_matrix(rot_model);
                            }
                        }

                        let mesh = node.data.get_mesh();

                        if !self.buffer_cache.contains_key(&(node.data.identify() as usize)) {
                            let vertices: Vec<Vertex> = mesh.vertices.iter().map(|x| Vertex::from(*x)).collect();
                            self.insert_buffer(
                                node.data.identify() as usize,
                                &vertices,
                                &mesh.indices,
                                &node.data.get_diffuse_map()
                            );
                        }

                        let (v, i, _t) = {
                            let item = self.buffer_cache.get(&(node.data.identify() as usize)).unwrap();
                            (item.vertices.clone(), item.indices.clone(), item.diffuse_map.clone())
                        };

                        // Push constants are leveraged here to send per-model matrices into the shaders
                        let push_constants = vs::ty::PushConstants {
                            model: node.data.get_world_matrix().clone().into()
                        };
                        cmd_buffer_build = cmd_buffer_build.draw_indexed(
                                self.pipeline.clone(),
                                self.dynamic_state.clone(),
                                v.clone(),
                                i.clone(),
                                self.pipeline_set.clone(),
                                push_constants // or () - both leak...
                        ).expect("Unable to add command");

                    }
                },
                None => break
            }
        }


        let cmd_buffer = cmd_buffer_build.end_render_pass()
                            .expect("unable to end renderpass ")
                            .build()
                            .unwrap();

        let prev = mem::replace(
            &mut self.previous_frame_end,
            Box::new(now(self.device.clone())) as Box<GpuFuture>
        );
        let after_future =
            prev.join(acquire_future)
                        .then_execute(self.queue.clone(), cmd_buffer)
                        .expect(
                            &format!("VulkanoRenderer(frame {}) - unable to execute command buffer", self.fps.count())
                        )
                        .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_num)
                        .then_signal_fence_and_flush();

        match after_future {
            Ok(future) => {
                self.previous_frame_end = Box::new(future) as Box<_>;
            }
            Err(vulkano::sync::FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Box::new(vulkano::sync::now(self.device.clone())) as Box<_>;
            }
            Err(e) => {
                println!("Error ending frame {:?}", e);
                self.previous_frame_end = Box::new(vulkano::sync::now(self.device.clone())) as Box<_>;
            }
        }


        self.fps.update();
    }

    #[allow(dead_code)]
    fn fps(&self) -> f32 {
        self.fps.get()
    }

}


use game_state::input::events::{
    InputEvent,
    MouseButton,
};

use game_state::input::screen::{
    ScreenPoint,
    ScreenRect,
    DeltaVector,
};

impl Identifyable for VulkanoRenderer {
    fn identify(&self) -> Identity {
        self.id
    }
}

impl Renderer for VulkanoRenderer {
    fn load(&mut self) {
    }

    fn unload(&mut self) {
        self.buffer_cache.clear();
    }

    fn queue_render_layer(&mut self, layer: Arc<SceneGraph>) {
        self.render_layer_queue.push_back(layer);
    }

    fn present(&mut self) {
        self.render();
    }
}

impl Drop for VulkanoRenderer {
    fn drop(&mut self) {
        println!("VulkanRenderer drop");
    }
}

impl InputSource for VulkanoRenderer {
    fn get_input_events(&mut self) -> VecDeque<InputEvent> {

        //println!("get_input_events");
        let mut events = VecDeque::new();
        {
            let event_loop = &mut self.events_loop.lock().unwrap();
            event_loop.poll_events(|e| events.push_back(e.clone()));
        }

        let this_window_id = self.id as u64;

        let mut converted_events = VecDeque::with_capacity(events.len());

        for e in events {

            #[allow(dead_code)]
            match e {
                winit::Event::DeviceEvent{device_id, ref event} => {
                    match event {
                        &winit::DeviceEvent::Added => {},
                        &winit::DeviceEvent::Removed => {},
                        &winit::DeviceEvent::MouseMotion { delta } => {},
                        &winit::DeviceEvent::MouseWheel {delta} => {
                            println!("it's magic {:?}", delta);
                        },
                        &winit::DeviceEvent::Motion { axis, value } => {},
                        &winit::DeviceEvent::Button { button, state } => {},
                        &winit::DeviceEvent::Key(input) => {},
                        &winit::DeviceEvent::Text{codepoint} => {}
                    }
                },
                winit::Event::WindowEvent{ window_id, ref event } => {
                    let maybe_converted_event = match event {
                        // Keyboard Events
                        &winit::WindowEvent::KeyboardInput{device_id, input} => {
                            let e = match input.state {
                                winit::ElementState::Pressed => InputEvent::KeyDown(self.id, input.scancode),
                                winit::ElementState::Released => InputEvent::KeyDown(self.id, input.scancode)
                            };
                            Some(e)
                        }

                        // Mouse Events

                        &winit::WindowEvent::CursorMoved{device_id, position, modifiers} => {
                            let (x,y) = position;
                            let old_pos: ScreenPoint = self.get_mouse_pos().clone();
                            // TODO: resolve f64 truncation to i32 here
                            let new_pos = ScreenPoint::new(x as i32, y as i32);
                            let moved =
                                InputEvent::MouseMove(self.id, new_pos.clone(), DeltaVector::from_points(&old_pos, &new_pos));
                            self.set_mouse_pos(new_pos);
                            Some(moved)
                        },
                        &winit::WindowEvent::MouseInput{device_id, state, button, modifiers} => {
                            let b = match button {
                                winit::MouseButton::Left => MouseButton::Left,
                                winit::MouseButton::Right => MouseButton::Right,
                                winit::MouseButton::Middle => MouseButton::Middle,
                                winit::MouseButton::Other(n) => MouseButton::Other(n)
                            };
                            let e = match state {
                                winit::ElementState::Pressed => InputEvent::MouseDown(self.id, b, self.get_mouse_pos().clone()),
                                winit::ElementState::Released => InputEvent::MouseUp(self.id, b, self.get_mouse_pos().clone())
                            };
                            Some(e)
                        },

                        &winit::WindowEvent::MouseWheel{device_id, delta, phase, modifiers} => {
                            let e = match delta {
                                winit::MouseScrollDelta::LineDelta(x,y) | winit::MouseScrollDelta::PixelDelta(x,y)  =>
                                    InputEvent::MouseWheel(self.id, self.get_mouse_pos().clone(), DeltaVector::new(x as i32, y as i32))
                            };
                            Some(e)
                        },

                        // Window Manager events

                        /*
                        &winit::WindowEvent::MouseEntered => Some(InputEvent::MouseEntered(self.id)),
                        &winit::WindowEvent::MouseLeft => Some(InputEvent::MouseLeft(self.id)),
                        */
                        &winit::WindowEvent::Closed => Some(InputEvent::Closed(self.id)),
                        &winit::WindowEvent::Focused(f) => Some(if f { InputEvent::GainedFocus(self.id) } else { InputEvent::LostFocus(self.id) }),
                        &winit::WindowEvent::Moved(x,y) => {
                            let new_rect = ScreenRect::new(x as i32, y as i32, self.rect.w, self.rect.h);
                            let e = InputEvent::Moved(self.id, ScreenPoint::new(x as i32, y as i32));
                            self.set_rect(new_rect);
                            Some(e)
                        }
                        &winit::WindowEvent::Resized(w, h) => {
                            let new_rect = ScreenRect::new(self.rect.x, self.rect.y, w as i32, h as i32);
                            let e = InputEvent::Resized(self.id, new_rect.clone());
                            self.set_rect(new_rect);
                            Some(e)
                        },
                        _ => None

                    };
                    if maybe_converted_event.is_some() {
                        converted_events.push_back(maybe_converted_event.unwrap());
                    }

                }
                _ => {}
            };
        }
        converted_events
    }
    // FIXME Ruby
}

#[cfg(test)]
mod tests {

    #[test]
    fn rando_test_flatten_vec_of_options(){
        let vals = vec![None, None, Some(1), None, Some(2), Some(3), None, None, None, Some(4)];
        let flat = vals.iter().enumerate().filter(|&(_, x)| x.is_some()).map(|(_, x)| x.unwrap()).collect::<Vec<u32>>();
        assert_eq!(flat, vec![1,2,3,4]);
    }
}

