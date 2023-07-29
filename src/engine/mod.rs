pub mod vertex;
pub mod buffer;
pub mod descriptor;

use std::{ffi::{CString, CStr}, ptr::null};
use ash::vk;
use glfw::Window;
use winapi::um::libloaderapi::GetModuleHandleW;
use self::vertex::Vertex;

pub struct Engine {
    glfw: glfw::Glfw,
    window: glfw::Window,
    entry: ash::Entry,
    // vulkan
    instance: ash::Instance,
    gpu: vk::PhysicalDevice,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    device: ash::Device,
    device_queue: vk::Queue,
    debug_utils: Option<ash::extensions::ext::DebugUtils>,
    debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
    surface_khr: vk::SurfaceKHR,
    surface_util: ash::extensions::khr::Surface,
    swapchain: vk::SwapchainKHR,
    swapchain_util: ash::extensions::khr::Swapchain,
    swapchain_format: vk::Format,
    swapchain_present_mode: vk::PresentModeKHR,
    extent: vk::Extent2D,
    viewport: vk::Viewport,
    scissor: vk::Rect2D,
    swapchain_image_views: Vec<vk::ImageView>,
    swapchain_framebuffers: Vec<vk::Framebuffer>,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,
    command_pool: vk::CommandPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    // drawing
    draw_command_buffer: vk::CommandBuffer,
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
    image_index: u32
}

impl Engine {
    pub const WIDTH: u32 = 800;
    pub const HEIGHT: u32 = 800;
    pub const TITLE: &'static str = "Vulkan Snake in Rust";

    pub const DEBUG: bool = false;

    pub fn new() -> Engine {
        let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
        glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

        let (window, _events) = glfw.create_window(Engine::WIDTH, Engine::HEIGHT, Engine::TITLE, glfw::WindowMode::Windowed)
            .expect("Failed to create GLFW window.");
        if Engine::DEBUG {
            println!("Created Window");
        }

        let mut supported_extensions = glfw.get_required_instance_extensions().unwrap();
        supported_extensions.push("VK_EXT_debug_utils".to_string());
        let supported_extensions = supported_extensions.iter().map(|e| format!("{e}\0")).collect::<Vec<_>>();
        let supported_extesnions_ptrs = supported_extensions.iter().map(|e| e.as_ptr() as *const i8).collect::<Vec<_>>();

        let enabled_layers = [
            "VK_LAYER_KHRONOS_validation\0",
        ];
        let enabled_layers_ptrs = enabled_layers.iter().map(|e| e.as_ptr() as *const i8).collect::<Vec<_>>();

        unsafe {
            let entry = ash::Entry::load().unwrap();

            let instance = {
                let c_name = CString::new(Engine::TITLE).unwrap();

                let app_info = vk::ApplicationInfo::builder()
                    .application_name(&c_name)
                    .application_version(vk::make_api_version(0, 1, 0, 0))
                    .engine_name(&c_name)
                    .engine_version(vk::make_api_version(0, 1, 0, 0))
                    .api_version(vk::make_api_version(0, 1, 3, 0))
                    .build();

                let create_info = vk::InstanceCreateInfo::builder()
                    .application_info(&app_info)
                    .enabled_extension_names(&supported_extesnions_ptrs)
                    .enabled_layer_names(&enabled_layers_ptrs)
                    .build();

                entry.create_instance(&create_info, None).unwrap()
            };
            if Engine::DEBUG {
                println!("Created Vulkan Instance");
            }

            let (debug_utils, debug_messenger) = if Engine::DEBUG {
                let debug_utils = ash::extensions::ext::DebugUtils::new(&entry, &instance);
                println!("Created debug utils");

                let debug_messenger = {
                    let create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                        .message_severity(
                            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                            | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                            | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                        )
                        .message_type(
                            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                        )
                        .pfn_user_callback(Some(vulkan_debug_callback))
                        .build();

                    debug_utils.create_debug_utils_messenger(&create_info, None).unwrap()
                };

                (Some(debug_utils), Some(debug_messenger))
            } else {
                (None, None)
            };

            let gpu = instance
                .enumerate_physical_devices()
                .unwrap()
                .into_iter()
                .filter(|p| {
                    let properties = instance.get_physical_device_properties(*p);

                    properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU ||
                    properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU
                })
                .nth(0)
                .expect("No discrete or intergrated gpu found");
            if Engine::DEBUG {
                let properties = instance.get_physical_device_properties(gpu);

                let name = CStr::from_ptr(properties.device_name.as_ptr()).to_string_lossy();
                println!("Found GPU: {}", name);
                println!("GPU Type {:#?}", properties.device_type);
            }

            let memory_properties = instance.get_physical_device_memory_properties(gpu);

            let queue_familys = instance.get_physical_device_queue_family_properties(gpu);
            let queue_family = queue_familys
                .iter()
                .enumerate()
                .find(|(_, p)| {
                    p.queue_flags.contains(vk::QueueFlags::GRAPHICS) &&
                    p.queue_flags.contains(vk::QueueFlags::TRANSFER)
                })
                .expect("No graphics queue family found");

            let device = {
                let mut physical_device_features = instance.get_physical_device_features(gpu);
                physical_device_features.sampler_anisotropy = 1;
                let device_extensions = [
                    "VK_KHR_swapchain\0",
                ];
                let device_extensions_ptrs = device_extensions.iter().map(|e| e.as_ptr() as *const i8).collect::<Vec<_>>();

                let create_info = vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&[
                        vk::DeviceQueueCreateInfo::builder()
                            .queue_family_index(queue_family.0 as u32)
                            .queue_priorities(&[1.0])
                            .build(),
                    ])
                    .enabled_extension_names(&device_extensions_ptrs)
                    .enabled_features(&physical_device_features)
                    .build();

                instance.create_device(gpu, &create_info, None).unwrap()
            };
            if Engine::DEBUG {
                println!("Created Vulkan Device");
            }

            let device_queue = device.get_device_queue(queue_family.0 as u32, 0);

            let surface_util = ash::extensions::khr::Surface::new(&entry, &instance);
            let win32_surface = ash::extensions::khr::Win32Surface::new(&entry, &instance);
            let mut surface_khr = {
                let create_info = vk::Win32SurfaceCreateInfoKHR::builder()
                    .hinstance(GetModuleHandleW(null()).cast())
                    .hwnd(window.get_win32_window())
                    .build();

                win32_surface.create_win32_surface(&create_info, None).unwrap()
            };
            if window.create_window_surface(instance.handle(), null(), &mut surface_khr).result().is_err() {
                panic!("Failed to create vulkan surface");
            }
            if Engine::DEBUG {
                println!("Created Vulkan Surface");
            }
            
            let swapchain_util = ash::extensions::khr::Swapchain::new(&instance, &device);

            let (swapchain, swapchain_format, swapchain_present_mode, extent) = {
                let capabilities = surface_util.get_physical_device_surface_capabilities(gpu, surface_khr).unwrap();
                let formats = surface_util.get_physical_device_surface_formats(gpu, surface_khr).unwrap();
                let present_modes = surface_util.get_physical_device_surface_present_modes(gpu, surface_khr).unwrap();

                let format = formats.clone().into_iter().find(|f| {
                    f.format == vk::Format::B8G8R8A8_SRGB &&
                    f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
                }).unwrap_or_else(|| {
                    println!("Failed to find suitable format so selected the first one");
                    formats[0]
                });

                let present_mode = present_modes.into_iter().find(|p| {
                    *p == vk::PresentModeKHR::IMMEDIATE
                }).unwrap();

                let framebuffer_size = window.get_framebuffer_size();
                let extent = vk::Extent2D {
                    width: (framebuffer_size.0 as u32).clamp(capabilities.min_image_extent.width, capabilities.max_image_extent.width),
                    height: (framebuffer_size.1 as u32).clamp(capabilities.min_image_extent.height, capabilities.max_image_extent.height),
                };

                let create_info = vk::SwapchainCreateInfoKHR::builder()
                    .surface(surface_khr)
                    .min_image_count(capabilities.min_image_count + 1)
                    .image_format(format.format)
                    .image_color_space(format.color_space)
                    .image_extent(extent)
                    .image_array_layers(1)
                    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .pre_transform(capabilities.current_transform)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .present_mode(present_mode)
                    .clipped(true)
                    .old_swapchain(vk::SwapchainKHR::null())
                    .build();

                (
                    swapchain_util.create_swapchain(&create_info, None).unwrap(),
                    format.format,
                    present_mode,
                    extent
                )
            };
            if Engine::DEBUG {
                println!("Created Swapchain");
            }

            let swapchain_images = swapchain_util.get_swapchain_images(swapchain).unwrap();

            let swapchain_image_views = swapchain_images.iter().map(|image| {
                let create_info = vk::ImageViewCreateInfo::builder()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(swapchain_format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .build();
                
                device.create_image_view(&create_info, None).unwrap()
            }).collect::<Vec<_>>();
            if Engine::DEBUG {
                println!("Created Swapchain Image Views");
            }

            let vertex_shader_module = {
                let binary = include_bytes!("../../shaders/default.vert.spv");

                let create_info = vk::ShaderModuleCreateInfo {
                    s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
                    code_size: binary.len(),
                    p_code: binary.as_ptr() as *const u32,
                    ..Default::default()
                };

                device.create_shader_module(&create_info, None).unwrap()
            };

            let fragment_shader_module = {
                let binary = include_bytes!("../../shaders/default.frag.spv");

                let create_info = vk::ShaderModuleCreateInfo {
                    s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
                    code_size: binary.len(),
                    p_code: binary.as_ptr() as *const u32,
                    ..Default::default()
                };

                device.create_shader_module(&create_info, None).unwrap()
            };
            if Engine::DEBUG {
                println!("Created shader modules");
            }

            let entry_point_name = CString::new("main").unwrap();
            let shader_stages = [
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(vertex_shader_module)
                    .name(&entry_point_name)
                    .build(),
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(fragment_shader_module)
                    .name(&entry_point_name)
                    .build()
            ];

            let dynamic_states = [
                vk::DynamicState::VIEWPORT,
                vk::DynamicState::SCISSOR
            ];

            let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(&dynamic_states)
                .build();
            if Engine::DEBUG {
                println!("Created dynamic state");
            }

            let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_binding_descriptions(&[Vertex::get_binding_description()])
                .vertex_attribute_descriptions(&Vertex::get_attribute_descriptions())
                .build();
            if Engine::DEBUG {
                println!("Created vertex input info");
            }

            let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                .primitive_restart_enable(false)
                .build();
            if Engine::DEBUG {
                println!("Created input assembly info");
            }

            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            };

            let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
                .viewport_count(1)
                .scissor_count(1)
                .build();
            if Engine::DEBUG {
                println!("Created viewport state info");
            }

            let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0)
                .cull_mode(vk::CullModeFlags::BACK)
                .front_face(vk::FrontFace::CLOCKWISE)
                .depth_bias_enable(false)
                .depth_bias_constant_factor(0.0)
                .depth_bias_clamp(0.0)
                .depth_bias_slope_factor(0.0)
                .build();
            if Engine::DEBUG {
                println!("Created rasterizer info");
            }

            let multisample_info = vk::PipelineMultisampleStateCreateInfo::builder()
                .sample_shading_enable(false)
                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                .build();
            if Engine::DEBUG {
                println!("Created multisample info");
            }

            let color_blend_attachments = [
                vk::PipelineColorBlendAttachmentState::builder()
                    .color_write_mask(vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A)
                    .blend_enable(false)
                    .build()
            ];

            let color_blend_info = vk::PipelineColorBlendStateCreateInfo::builder()
                .logic_op_enable(false)
                .attachments(&color_blend_attachments)
                .build();
            if Engine::DEBUG {
                println!("Created color blend info");
            }

            let ubo_descriptor_binding = vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build();

            let descriptor_set_layout = {
                let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
                    .bindings(&[ubo_descriptor_binding])
                    .build();

                device.create_descriptor_set_layout(&create_info, None).unwrap()
            };

            let pipeline_layout = {
                let create_info = vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&[
                        descriptor_set_layout
                    ])
                    .push_constant_ranges(&[])
                    .build();

                device.create_pipeline_layout(&create_info, None).unwrap()
            };

            let command_pool = {
                let create_info = vk::CommandPoolCreateInfo::builder()
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .queue_family_index(queue_family.0 as u32)
                    .build();

                device.create_command_pool(&create_info, None).unwrap()
            };

            let render_pass = {
                let attachment_description = vk::AttachmentDescription::builder()
                    .format(swapchain_format)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                    .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                    .initial_layout(vk::ImageLayout::UNDEFINED)
                    .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .build();

                let color_attachment_ref = vk::AttachmentReference::builder()
                    .attachment(0)
                    .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .build();

                let subpass_description = vk::SubpassDescription::builder()
                    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                    .color_attachments(&[color_attachment_ref])
                    .build();

                let subpass_dependency = vk::SubpassDependency::builder()
                    .src_subpass(vk::SUBPASS_EXTERNAL)
                    .dst_subpass(0)
                    .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                    .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                    .src_access_mask(vk::AccessFlags::empty())
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .build();

                let create_info = vk::RenderPassCreateInfo::builder()
                    .attachments(&[attachment_description])
                    .subpasses(&[subpass_description])
                    .dependencies(&[subpass_dependency])
                    .build();

                device.create_render_pass(&create_info, None).unwrap()
            };
            if Engine::DEBUG {
                println!("Created render pass");
            }

            let graphics_pipeline = {
                let create_info = vk::GraphicsPipelineCreateInfo::builder()
                    .stages(&shader_stages)
                    .dynamic_state(&dynamic_state)
                    .vertex_input_state(&vertex_input_info)
                    .input_assembly_state(&input_assembly_info)
                    .viewport_state(&viewport_state_info)
                    .rasterization_state(&rasterizer_info)
                    .multisample_state(&multisample_info)
                    .color_blend_state(&color_blend_info)
                    .layout(pipeline_layout)
                    .render_pass(render_pass)
                    .subpass(0)
                    .build();

                device.create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None).unwrap()[0]
            };
            if Engine::DEBUG {
                println!("Created graphics pipeline");
            }

            let framebuffers = swapchain_image_views.iter().map(|image_view| {
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(&[*image_view])
                    .width(extent.width)
                    .height(extent.height)
                    .layers(1)
                    .build();

                device.create_framebuffer(&create_info, None).unwrap()
            }).collect::<Vec<_>>();

            let draw_command_buffer = {
                let create_info = vk::CommandBufferAllocateInfo::builder()
                    .command_pool(command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1)
                    .build();

                device.allocate_command_buffers(&create_info).unwrap()[0]
            };

            let (image_available_semaphore, render_finished_semaphore) = {
                let create_info = vk::SemaphoreCreateInfo::builder().build();

                (device.create_semaphore(&create_info, None).unwrap(), device.create_semaphore(&create_info, None).unwrap())
            };

            let in_flight_fence = {
                let create_info = vk::FenceCreateInfo::builder()
                    .flags(vk::FenceCreateFlags::SIGNALED)
                    .build();

                device.create_fence(&create_info, None).unwrap()
            };

            Engine {
                glfw,
                window,
                entry,
                instance,
                gpu,
                memory_properties,
                device,
                device_queue,
                debug_utils,
                debug_messenger,
                surface_khr,
                surface_util,
                swapchain,
                swapchain_util,
                swapchain_format,
                swapchain_present_mode,
                extent,
                viewport,
                scissor,
                swapchain_image_views,
                swapchain_framebuffers: framebuffers,
                render_pass,
                pipeline_layout,
                graphics_pipeline,
                command_pool,
                descriptor_set_layout,
                draw_command_buffer,
                image_available_semaphore,
                render_finished_semaphore,
                in_flight_fence,
                image_index: 0
            }
        }
    }

    pub fn begin_draw(&mut self) {
        unsafe {
            self.device.wait_for_fences(&[self.in_flight_fence], true, std::u64::MAX).unwrap();

            self.image_index = self.swapchain_util.acquire_next_image(self.swapchain, std::u64::MAX, self.image_available_semaphore, vk::Fence::null()).unwrap().0;

            self.device.reset_fences(&[self.in_flight_fence]).unwrap();
            self.device.begin_command_buffer(self.draw_command_buffer, &vk::CommandBufferBeginInfo::builder().build()).unwrap();

            self.device.cmd_begin_render_pass(
                self.draw_command_buffer,
                &vk::RenderPassBeginInfo::builder()
                    .render_pass(self.render_pass)
                    .framebuffer(self.swapchain_framebuffers[self.image_index as usize])
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.extent
                    })
                    .clear_values(&[
                        vk::ClearValue {
                            color: vk::ClearColorValue {
                                float32: [0.0, 0.0, 0.0, 1.0],
                            }
                        }
                    ])
                    .build(),
                vk::SubpassContents::INLINE
            );

            self.device.cmd_bind_pipeline(self.draw_command_buffer, vk::PipelineBindPoint::GRAPHICS, self.graphics_pipeline);

            self.device.cmd_set_viewport(self.draw_command_buffer, 0, &[self.viewport]);
            self.device.cmd_set_scissor(self.draw_command_buffer, 0, &[self.scissor]);
        }
    }

    pub fn end_draw(&mut self) {
        unsafe {
            self.device.cmd_end_render_pass(self.draw_command_buffer);
            self.device.end_command_buffer(self.draw_command_buffer).unwrap();

            self.device.queue_submit(
                self.device_queue,
                &[
                    vk::SubmitInfo::builder()
                        .command_buffers(&[self.draw_command_buffer])
                        .wait_semaphores(&[self.image_available_semaphore])
                        .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                        .signal_semaphores(&[self.render_finished_semaphore])
                        .build()
                ],
                self.in_flight_fence
            ).unwrap();

            self.swapchain_util.queue_present(
                self.device_queue,
                &vk::PresentInfoKHR::builder()
                    .wait_semaphores(&[self.render_finished_semaphore])
                    .swapchains(&[self.swapchain])
                    .image_indices(&[self.image_index])
                    .build()
            ).unwrap();
        }
    }

    pub fn begin_single_exec_command(&self) -> vk::CommandBuffer {
        unsafe {
            let command_buffer = self.device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_pool(self.command_pool)
                    .command_buffer_count(1)
                    .build()
            ).unwrap()[0];

            self.device.begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                    .build()
            ).unwrap();

            command_buffer
        }
    }

    pub fn end_single_exec_command(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device.end_command_buffer(command_buffer).unwrap();

            self.device.queue_submit(
                self.device_queue,
                &[
                    vk::SubmitInfo::builder()
                        .command_buffers(&[command_buffer])
                        .build()
                ],
                vk::Fence::null()
            ).unwrap();

            self.device.queue_wait_idle(self.device_queue).unwrap();

            self.device.free_command_buffers(self.command_pool, &[command_buffer]);
        }
    }

    pub fn running(&mut self) -> bool {
        self.glfw.poll_events();
        !self.window.should_close()
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn device(&self) -> ash::Device {
        self.device.clone()
    }

    pub fn memory_properties(&self) -> vk::PhysicalDeviceMemoryProperties {
        self.memory_properties
    }

    pub fn draw_command_buffer(&self) -> vk::CommandBuffer {
        self.draw_command_buffer
    }

    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }

    pub fn pipeline_layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }
}

/// yoinked from ash examples
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        std::borrow::Cow::from("")
    } else {
        std::ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        std::borrow::Cow::from("")
    } else {
        std::ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
    );

    vk::FALSE
}

fn find_memory_type(
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags
) -> Option<u32> {
    for i in 0..memory_properties.memory_type_count {
        if (type_filter & (1 << i)) > 0 && ((memory_properties.memory_types[i as usize].property_flags & properties) == properties) {
            return Some(i as u32);
        }
    }

    None
}

pub fn px_to_screen(x_px: f32, y_px: f32) -> [f32; 2] {
    [
        ((x_px * 2.0) / Engine::WIDTH as f32) - 1.0,
        (((y_px * 2.0) / Engine::HEIGHT as f32) - 1.0) * -1.0
    ]
}
