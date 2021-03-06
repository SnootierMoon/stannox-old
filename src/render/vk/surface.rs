use erupt::vk;

pub struct Surface {
    instance: std::rc::Rc<super::Instance>,
    render_pass: vk::RenderPass,
    swapchain: vk::SwapchainKHR,
    depth_image: vk::Image,
    depth_image_memory: vk::DeviceMemory,
    depth_image_view: vk::ImageView,
    swapchain_images: Vec<SwapchainImage>,
    extent: vk::Extent2D,
}

impl Surface {
    pub fn new(instance: std::rc::Rc<super::Instance>, window: &crate::window::Window) -> Self {
        let instance = instance.clone();
        let device = instance.device();
        let surface_info = instance.surface_info(window.size().into());

        // Create Render Pass

        let attachments = [
            vk::AttachmentDescriptionBuilder::new()
                .format(surface_info.surface_format.format)
                .samples(vk::SampleCountFlagBits::_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR),
            vk::AttachmentDescriptionBuilder::new()
                .format(vk::Format::D32_SFLOAT)
                .samples(vk::SampleCountFlagBits::_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
        ];
        let depth_stencil_attachment = vk::AttachmentReferenceBuilder::new()
            .attachment(1)
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
        let color_attachments = vk::AttachmentReferenceBuilder::new()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        let subpass = vk::SubpassDescriptionBuilder::new()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_attachments))
            .depth_stencil_attachment(&depth_stencil_attachment);
        let dependency = vk::SubpassDependencyBuilder::new()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            )
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            )
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            );
        let render_pass_create_info = vk::RenderPassCreateInfoBuilder::new()
            .attachments(&attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(std::slice::from_ref(&dependency));
        let render_pass =
            unsafe { device.create_render_pass(&render_pass_create_info, None) }.unwrap();

        // Create Swapchain

        let (graphics, present) = (instance.graphics(), instance.present());
        let (sharing_mode, queue_families) = if graphics.family == present.family {
            (vk::SharingMode::EXCLUSIVE, vec![graphics.family])
        } else {
            (
                vk::SharingMode::CONCURRENT,
                vec![graphics.family, present.family],
            )
        };
        let min_image_count = surface_info.surface_caps.min_image_count;
        let swapchain_create_info = vk::SwapchainCreateInfoKHRBuilder::new()
            .surface(surface_info.surface)
            .min_image_count(min_image_count)
            .image_format(surface_info.surface_format.format)
            .image_color_space(surface_info.surface_format.color_space)
            .image_extent(surface_info.extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(sharing_mode)
            .queue_family_indices(&queue_families)
            .pre_transform(surface_info.surface_caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagBitsKHR::OPAQUE_KHR)
            .present_mode(surface_info.present_mode)
            .clipped(true);
        let swapchain =
            unsafe { device.create_swapchain_khr(&swapchain_create_info, None) }.unwrap();

        // Create Depth Image Resources

        let image_create_info = vk::ImageCreateInfoBuilder::new()
            .image_type(vk::ImageType::_2D)
            .extent(vk::Extent3D {
                width: surface_info.extent.width,
                height: surface_info.extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(vk::Format::D32_SFLOAT)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .samples(vk::SampleCountFlagBits::_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let depth_image = unsafe { device.create_image(&image_create_info, None) }.unwrap();
        let memory_requirements =
            unsafe { device.get_image_memory_requirements(depth_image) };
        let memory_type_index = instance
            .get_memory_type_index(vk::MemoryPropertyFlags::DEVICE_LOCAL, memory_requirements);
        let memory_allocate_info = vk::MemoryAllocateInfoBuilder::new()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);
        let depth_image_memory =
            unsafe { device.allocate_memory(&memory_allocate_info, None) }.unwrap();
        unsafe { device.bind_image_memory(depth_image, depth_image_memory, 0) }.unwrap();
        let image_view_create_info = vk::ImageViewCreateInfoBuilder::new()
            .image(depth_image)
            .view_type(vk::ImageViewType::_2D)
            .format(vk::Format::D32_SFLOAT)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        let depth_image_view =
            unsafe { device.create_image_view(&image_view_create_info, None) }.unwrap();

        // Create Swapchain Image Resources and Framebuffers

        let images = unsafe { device.get_swapchain_images_khr(swapchain, None) }.unwrap();
        let swapchain_images = images
            .into_iter()
            .map(|image| {
                let view_create_info = vk::ImageViewCreateInfoBuilder::new()
                    .image(image)
                    .view_type(vk::ImageViewType::_2D)
                    .format(surface_info.surface_format.format)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                let view =
                    unsafe { device.create_image_view(&view_create_info, None) }.unwrap();
                let attachments = [view, depth_image_view];
                let framebuffer_create_info = vk::FramebufferCreateInfoBuilder::new()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(surface_info.extent.width)
                    .height(surface_info.extent.height)
                    .layers(1);
                let framebuffer =
                    unsafe { device.create_framebuffer(&framebuffer_create_info, None) }
                        .unwrap();
                SwapchainImage {
                    view,
                    framebuffer,
                    fence: vk::Fence::null(),
                }
            })
            .collect();

        Self {
            instance,
            render_pass,
            swapchain,
            depth_image,
            depth_image_memory,
            depth_image_view,
            swapchain_images,
            extent: surface_info.extent,
        }
    }

    pub fn instance(&self) -> std::rc::Rc<super::Instance> {
        self.instance.clone()
    }

    pub(super) fn borrow_image<F: FnOnce(vk::RenderPassBeginInfoBuilder)>(
        &mut self,
        sync: &super::SyncObject,
        f: F,
    ) -> bool {
        let device = self.instance.device();

        // Acquire Image from Swapchain

        let index = match unsafe {
            device.acquire_next_image_khr(
                self.swapchain,
                u64::MAX,
                Some(sync.image_available),
                None,
            )
        }
        .result()
        {
            Ok(x) => x as usize,
            Err(vk::Result::SUBOPTIMAL_KHR) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                return false;
            }
            Err(e) => panic!("{}", e),
        };
        if !self.swapchain_images[index].fence.is_null() {
            unsafe { device.wait_for_fences(&[self.swapchain_images[index].fence], true, u64::MAX) }
                .unwrap()
        };
        self.swapchain_images[index].fence = sync.in_flight;

        // Run Callback

        const CLEAR_VALUES: [vk::ClearValue; 2] = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];
        let render_pass_begin_info = vk::RenderPassBeginInfoBuilder::new()
            .render_pass(self.render_pass)
            .framebuffer(self.swapchain_images[index].framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.extent,
            })
            .clear_values(&CLEAR_VALUES);
        f(render_pass_begin_info);

        // Present Image

        let image_index = index as u32;
        let present_info = vk::PresentInfoKHRBuilder::new()
            .wait_semaphores(std::slice::from_ref(&sync.render_finished))
            .swapchains(std::slice::from_ref(&self.swapchain))
            .image_indices(std::slice::from_ref(&image_index));
        match unsafe { device.queue_present_khr(self.instance.present().queue, &present_info) }
            .result()
        {
            Ok(_) => (),
            Err(vk::Result::SUBOPTIMAL_KHR) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                return false;
            }
            Err(e) => panic!("{}", e),
        };

        true
    }

    pub(super) fn render_info(&self) -> super::RenderInfo {
        super::RenderInfo {
            render_pass: self.render_pass,
            extent: self.extent,
        }
    }

    pub fn rebuild(&mut self, window: &crate::window::Window) {
        let instance = self.instance.clone();
        unsafe {
            std::mem::drop(std::ptr::read(self));
            std::ptr::write(self, Self::new(instance, window))
        }
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.extent.width as f32 / self.extent.height as f32
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        let device = self.instance.device();
        self.swapchain_images.iter().for_each(|image| unsafe {
            device.destroy_framebuffer(Some(image.framebuffer), None);
            device.destroy_image_view(Some(image.view), None)
        });
        unsafe {
            device.destroy_image_view(Some(self.depth_image_view), None);
            device.free_memory(Some(self.depth_image_memory), None);
            device.destroy_image(Some(self.depth_image), None);
            device.destroy_swapchain_khr(Some(self.swapchain), None);
            device.destroy_render_pass(Some(self.render_pass), None)
        }
    }
}

#[derive(Copy, Clone)]
struct SwapchainImage {
    view: vk::ImageView,
    framebuffer: vk::Framebuffer,
    fence: vk::Fence,
}
