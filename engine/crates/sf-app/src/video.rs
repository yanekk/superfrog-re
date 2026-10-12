//! The picture to the window (DESIGN §2.7): the game's frame as a texture, drawn at a whole-number
//! scale with nearest-neighbour sampling, centred, black around it. T27 adds the CRT pass here.

use std::sync::Arc;

use sf_core::{Picture, to_rgb12};
use winit::window::Window;

/// `pic` as 8-bit RGBA, row-major: each 4-bit channel times 17, so 0xF is 255.
pub fn picture_rgba(pic: &Picture) -> Vec<u8> {
    to_rgb12(pic)
        .into_iter()
        .flat_map(|c| {
            let ch = |s: u16| ((c >> s) & 0xF) as u8 * 17;
            [ch(8), ch(4), ch(0), 255]
        })
        .collect()
}

/// The largest viewport of `scale` × the frame that fits the surface, centred: `(x, y, w, h)`.
pub fn viewport(surface: (u32, u32), frame: (u32, u32), scale: u32) -> (u32, u32, u32, u32) {
    let fit = (surface.0 / frame.0.max(1)).min(surface.1 / frame.1.max(1)).max(1);
    let s = scale.min(fit);
    let (w, h) = (frame.0 * s, frame.1 * s);
    (surface.0.saturating_sub(w) / 2, surface.1.saturating_sub(h) / 2, w, h)
}

const SHADER: &str = r#"
@group(0) @binding(0) var frame: texture_2d<f32>;
@group(0) @binding(1) var pick: sampler;

struct Out { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex
fn vs(@builtin(vertex_index) i: u32) -> Out {
    // One triangle covering the viewport.
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: Out;
    o.pos = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(p.x, 1.0 - p.y);
    return o;
}

@fragment
fn fs(o: Out) -> @location(0) vec4<f32> {
    return textureSample(frame, pick, o.uv);
}
"#;

pub struct Video {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// The frame texture, its size and the bind group reading it.
    frame: Option<(wgpu::Texture, (u32, u32), wgpu::BindGroup)>,
    scale: u32,
}

impl Video {
    pub fn new(window: Arc<Window>, scale: u32) -> Result<Video, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| format!("no surface for the window: {e}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|e| format!("no GPU adapter: {e}"))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|e| format!("no GPU device: {e}"))?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("the GPU cannot draw to this window")?;
        // A non-sRGB format, so the game's colours reach the screen as written.
        let caps = surface.get_capabilities(&adapter);
        if let Some(f) = caps.formats.iter().find(|f| !f.is_srgb()) {
            config.format = *f;
        }
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &config);

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("present"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("present"),
            bind_group_layouts: &[Some(&layout)],
            ..Default::default()
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("present"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(config.format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        // Nearest: every game pixel is a solid square of `scale` × `scale` screen pixels.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Ok(Video {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            layout,
            sampler,
            frame: None,
            scale,
        })
    }

    pub fn resized(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    /// Upload a frame (RGBA, `size`) as the one to show.
    pub fn upload(&mut self, size: (u32, u32), rgba: &[u8]) {
        if self.frame.as_ref().is_none_or(|f| f.1 != size) {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("frame"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("frame"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            self.frame = Some((texture, size, group));
        }
        let (texture, _, _) = self.frame.as_ref().expect("made above");
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.0 * 4),
                rows_per_image: Some(size.1),
            },
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Draw the uploaded frame to the window.
    pub fn present(&mut self) {
        let Some((_, size, group)) = &self.frame else {
            return;
        };
        let target = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return,
        };
        let view = target.texture.create_view(&Default::default());
        let (x, y, w, h) = viewport(
            (self.config.width, self.config.height),
            *size,
            self.scale * self.window.scale_factor().round().max(1.0) as u32,
        );
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_viewport(x as f32, y as f32, w as f32, h as f32, 0.0, 1.0);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([enc.finish()]);
        self.window.pre_present_notify();
        self.queue.present(target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_core::View;

    #[test]
    fn twelve_bit_colour_widens_to_eight_bits_a_channel() {
        let mut p = Picture::black(View::default());
        p.palettes[0][1] = 0xF80;
        p.pixels[1] = 1;
        let rgba = picture_rgba(&p);
        assert_eq!(rgba.len(), 320 * 256 * 4);
        assert_eq!(&rgba[..8], &[0, 0, 0, 255, 255, 136, 0, 255]);
    }

    #[test]
    fn the_viewport_is_a_whole_multiple_centred_and_shrinks_to_fit() {
        assert_eq!(viewport((320, 256), (320, 256), 1), (0, 0, 320, 256));
        assert_eq!(viewport((1000, 600), (320, 256), 2), (180, 44, 640, 512));
        assert_eq!(viewport((700, 600), (320, 256), 3), (30, 44, 640, 512));
        assert_eq!(viewport((100, 100), (320, 256), 2), (0, 0, 320, 256));
    }
}
