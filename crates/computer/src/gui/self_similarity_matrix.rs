use wgpu::{
    util::DeviceExt, BindGroup, BindGroupDescriptor, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Buffer, CommandBuffer, Device, Extent3d, PrimitiveTopology, Queue,
    RenderPipeline, ShaderStages, Texture, TextureFormat, TextureView,
};

use crate::Sample;

pub struct SelfSimilarityMatrix {
    horizontal_signal_size: usize,

    horizontal_texture: Texture,
    horizontal_texture_size: Extent3d,
    vertical_texture: Texture,
    vertical_texture_size: Extent3d,

    uniform_buffer: Buffer,
    bind_group: BindGroup,
    render_pipeline: RenderPipeline,
}

impl SelfSimilarityMatrix {
    pub fn new(
        horizontal_signal_size: usize,
        vertical_signal_size: usize,
        device: &Device,
        swapchain_format: TextureFormat,
    ) -> Self {
        let (horizontal_texture, horizontal_texture_size, horizontal_texture_view) =
            get_1d_texture_size_view(device, horizontal_signal_size);

        let (vertical_texture, vertical_texture_size, vertical_texture_view) =
            get_1d_texture_size_view(device, vertical_signal_size);

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: &0.0f32.to_le_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sampler = device.create_sampler(&Default::default());

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D1,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D1,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(4),
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&horizontal_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&vertical_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader =
            device.create_shader_module(wgpu::include_wgsl!("self_similarity_matrix_frag.wgsl"));

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            horizontal_signal_size,
            horizontal_texture,
            horizontal_texture_size,
            vertical_texture,
            vertical_texture_size,
            uniform_buffer,
            bind_group,
            render_pipeline,
        }
    }

    pub fn render<'a, I>(
        &self,
        horizontal_signal: I,
        vertical_signal: I,
        delay_samples: usize,
        queue: &Queue,
        target: TextureView,
        device: &Device,
    ) -> CommandBuffer
    where
        I: IntoIterator<Item = &'a Sample>,
    {
        let hor_samples = horizontal_signal
            .into_iter()
            .flat_map(|item| item.to_le_bytes())
            .collect::<Vec<_>>();
        let ver_samples = vertical_signal
            .into_iter()
            .flat_map(|item| item.to_le_bytes())
            .collect::<Vec<_>>();

        queue.write_texture(
            // Tells wgpu where to copy the data
            wgpu::ImageCopyTexture {
                texture: &self.horizontal_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // The actual data
            &hor_samples,
            // The layout of the texture
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(hor_samples.len() as u32),
                rows_per_image: Some(1),
            },
            self.horizontal_texture_size,
        );

        queue.write_texture(
            // Tells wgpu where to copy the data
            wgpu::ImageCopyTexture {
                texture: &self.vertical_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // The actual data
            &ver_samples,
            // The layout of the texture
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(ver_samples.len() as u32),
                rows_per_image: Some(1),
            },
            self.vertical_texture_size,
        );

        let delay_relative = 1.0 - delay_samples as f32 / self.horizontal_signal_size as f32;
        queue.write_buffer(&self.uniform_buffer, 0, &delay_relative.to_le_bytes());

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..4, 0..1);
        drop(render_pass);

        encoder.finish()
    }
}

fn get_1d_texture_size_view(device: &Device, size: usize) -> (Texture, Extent3d, TextureView) {
    let texture_size = wgpu::Extent3d {
        width: size as u32,
        height: 1,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D1,
        format: wgpu::TextureFormat::R32Float,
        // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
        // COPY_DST means that we want to copy data to this texture
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("horizontal texture"),
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    (texture, texture_size, view)
}
