use std::mem::size_of;
use ash::vk;
use super::find_memory_type;

#[derive(Clone)]
pub struct Buffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    size: u64,
    count: u32,
    // vulkan handles
    device: ash::Device
}

impl Buffer {
    pub fn new<T>(
        data: &[T],
        usage: vk::BufferUsageFlags,
        device: ash::Device,
        device_memory_properties: vk::PhysicalDeviceMemoryProperties,
        memory_flags: vk::MemoryPropertyFlags
    ) -> Buffer {
        unsafe {
            let buffer = device.create_buffer(
                &vk::BufferCreateInfo::builder()
                    .size(data.len() as u64 * size_of::<T>() as u64)
                    .usage(usage)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .build(),
                None
            ).unwrap();

            let memory_requirements = device.get_buffer_memory_requirements(buffer);

            let buffer_memory = device.allocate_memory(
                &vk::MemoryAllocateInfo::builder()
                    .allocation_size(memory_requirements.size)
                    .memory_type_index(
                        find_memory_type(
                            device_memory_properties,
                            memory_requirements.memory_type_bits,
                            memory_flags
                        ).unwrap()
                    ),
                None
            ).unwrap();

            device.bind_buffer_memory(buffer, buffer_memory, 0).unwrap();
            let buffer_ptr = device.map_memory(buffer_memory, 0, data.len() as u64 * size_of::<T>() as u64, vk::MemoryMapFlags::empty()).unwrap() as *mut T;
            buffer_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
            device.unmap_memory(buffer_memory);

            Buffer {
                buffer,
                memory: buffer_memory,
                size: data.len() as u64 * size_of::<T>() as u64,
                count: data.len() as u32,
                device
            }
        }
    }

    pub fn handle(&self) -> vk::Buffer {
        self.buffer
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn set_buffer<T>(&self, data: &[T]) {
        unsafe {
            let data_ptr = self.device.map_memory(self.memory, 0, self.size, vk::MemoryMapFlags::empty()).unwrap() as *mut T;
            data_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
            self.device.unmap_memory(self.memory);
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None);
        }
    }
}
