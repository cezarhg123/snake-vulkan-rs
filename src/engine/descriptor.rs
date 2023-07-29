use ash::vk;
use super::Engine;

#[repr(C)]
pub struct UBO {
    pub color: [f32; 3]
}

#[derive(Debug, Clone)]
pub struct DescriptorSet {
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
    buffer_info: vk::DescriptorBufferInfo
}

pub struct DescriptorBuilder {
    bindings: Vec<vk::DescriptorSetLayoutBinding>,
    uniform: Option<vk::Buffer>,
    uniform_size: Option<u64>
}

impl DescriptorBuilder {
    pub fn add_binding(mut self, binding: vk::DescriptorSetLayoutBinding) -> DescriptorBuilder {
        self.bindings.push(binding);
        self
    }

    pub fn uniform(mut self, buffer: vk::Buffer, size: u64) -> DescriptorBuilder {
        self.uniform = Some(buffer);
        self.uniform_size = Some(size);
        self
    }

    pub fn build(self, engine: &Engine) -> DescriptorSet {
        unsafe {
            let descriptor_pool = {
                let pool_sizes = self.bindings
                    .iter()
                    .map(|binding| {
                        vk::DescriptorPoolSize::builder()
                            .ty(binding.descriptor_type)
                            .descriptor_count(1)
                            .build()
                    })
                    .collect::<Vec<_>>();

                let create_info = vk::DescriptorPoolCreateInfo::builder()
                    .pool_sizes(&pool_sizes)
                    .max_sets(1)
                    .build();

                engine.device().create_descriptor_pool(&create_info, None).unwrap()
            };

            let descriptor_set = {
                let create_info = vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(descriptor_pool)
                    .set_layouts(&[engine.descriptor_set_layout()])
                    .build();

                engine.device().allocate_descriptor_sets(&create_info).unwrap()[0]
            };

            let buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.uniform.unwrap())
                .offset(0)
                .range(self.uniform_size.unwrap())
                .build();

            DescriptorSet {
                descriptor_pool,
                descriptor_set,
                buffer_info
            }
        }
    }
}

impl DescriptorSet {
    pub fn builder() -> DescriptorBuilder {
        DescriptorBuilder {
            bindings: Vec::new(),
            uniform: None,
            uniform_size: None
        }
    }

    pub fn write_descriptor_set(&self, device: &ash::Device) {
        unsafe {
            let write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(self.descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&[self.buffer_info])
                .build();

            device.update_descriptor_sets(&[write_descriptor_set], &[]);
        }
    }

    pub fn descriptor_set(&self) -> vk::DescriptorSet {
        self.descriptor_set
    }
}
