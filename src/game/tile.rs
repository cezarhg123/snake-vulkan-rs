use std::mem::size_of;
use ash::vk;
use crate::engine::{buffer::Buffer, descriptor::{DescriptorSet, UBO}, vertex::Vertex, px_to_screen, Engine};

#[derive(Clone)]
pub struct Tile {
    pub tile_state: TileState,
    vertex_buffer: Buffer,
    uniform_buffer: Buffer,
    descriptor_set: DescriptorSet
}

impl Tile {
    pub fn new(
        position: [f32; 2],
        tile_state: TileState,
        engine: &Engine
    ) -> Tile {
        let vertex_buffer = Buffer::new(
            &[
                Vertex::new(px_to_screen(position[0] + 4.0, position[1] + 4.0)),
                Vertex::new(px_to_screen(position[0] + 4.0, position[1] + 76.0)),
                Vertex::new(px_to_screen(position[0] + 76.0, position[1] + 76.0)),

                Vertex::new(px_to_screen(position[0] + 4.0, position[1] + 4.0)),
                Vertex::new(px_to_screen(position[0] + 76.0, position[1] + 76.0)),
                Vertex::new(px_to_screen(position[0] + 76.0, position[1] + 4.0))
            ],
            vk::BufferUsageFlags::VERTEX_BUFFER,
            engine.device(),
            engine.memory_properties(),
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT
        );

        let uniform_buffer = Buffer::new(
            &[
                UBO {
                    color: match tile_state {
                        TileState::Empty => [0.0, 0.0, 0.0],
                        TileState::Snake => [0.0, 0.0, 1.0],
                        TileState::Apple => [1.0, 0.0, 0.0]
                    }
                }
            ],
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            engine.device(),
            engine.memory_properties(),
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT
        );

        let descriptor_set = DescriptorSet::builder()
            .add_binding(
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                    .build()
            )
            .uniform(uniform_buffer.handle(), size_of::<UBO>() as u64)
            .build(engine);

        Tile {
            tile_state,
            vertex_buffer,
            uniform_buffer,
            descriptor_set
        }
    }

    pub fn draw(&self, draw_command_buffer: vk::CommandBuffer, device: &ash::Device, pipeline_layout: vk::PipelineLayout) {
        unsafe {
            self.uniform_buffer.set_buffer(&[
                UBO {
                    color: match self.tile_state {
                        TileState::Empty => [0.0, 0.0, 0.0],
                        TileState::Snake => [0.0, 0.0, 1.0],
                        TileState::Apple => [1.0, 0.0, 0.0]
                    }
                }
            ]);
            self.descriptor_set.write_descriptor_set(device);

            device.cmd_bind_descriptor_sets(
                draw_command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &[self.descriptor_set.descriptor_set()],
                &[]
            );
            device.cmd_bind_vertex_buffers(
                draw_command_buffer,
                0,
                &[self.vertex_buffer.handle()],
                &[0]
            );
            device.cmd_draw(
                draw_command_buffer,
                self.vertex_buffer.count(),
                1,
                0,
                0
            );
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum TileState {
    Empty,
    Snake,
    Apple
}
