//====================================================================

use std::{collections::HashSet, error::Error, fmt::Display, hash::BuildHasherDefault};

use cabat_common::Size;
use cosmic_text::{CacheKey, FontSystem, SwashCache};
use etagere::{euclid::Size2D, AllocId, BucketedAtlasAllocator};
use lru::LruCache;
use rustc_hash::FxHasher;

use crate::{render_tools, texture::Texture};

//====================================================================

#[derive(Debug)]
pub struct CacheGlyphError;

impl Error for CacheGlyphError {}

impl Display for CacheGlyphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO
        write!(f, "Failed to cache glyph")
    }
}

//====================================================================

pub(super) struct GlyphData {
    alloc_id: AllocId,
    pub size: [f32; 2],
    pub uv_start: [f32; 2],
    pub uv_end: [f32; 2],
}

type Hasher = BuildHasherDefault<FxHasher>;

pub struct TextAtlas {
    packer: BucketedAtlasAllocator,

    glyphs_in_use: HashSet<CacheKey, Hasher>,
    cached_glyphs: LruCache<CacheKey, GlyphData, Hasher>,

    texture: Texture,
    texture_size: Size<u32>,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl TextAtlas {
    pub fn new(device: &wgpu::Device) -> Self {
        const DEFAULT_START_SIZE: u32 = 256;

        let packer = BucketedAtlasAllocator::new(Size2D::new(
            DEFAULT_START_SIZE as i32,
            DEFAULT_START_SIZE as i32,
        ));
        let glyphs_in_use = HashSet::with_hasher(Hasher::default());
        let cached_glyphs = LruCache::unbounded_with_hasher(Hasher::default());

        let texture_size = Size::new(DEFAULT_START_SIZE, DEFAULT_START_SIZE);
        let texture = Texture::from_size(device, texture_size, Some("Text Atlas Texture"), None);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Text Atlas Bind Group Layout"),
            entries: &[
                render_tools::bgl_texture_entry(0),
                render_tools::bgl_sampler_entry(1),
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text Atlas Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        Self {
            packer,
            glyphs_in_use,
            cached_glyphs,
            texture,
            texture_size,
            bind_group_layout,
            bind_group,
        }
    }

    #[inline]
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    #[inline]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

//--------------------------------------------------

impl TextAtlas {
    // Cache glyph if not already and then promote in LRU
    pub fn use_glyph(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
        key: &CacheKey,
    ) -> Result<&GlyphData, ()> {
        // Already has glyph cached
        if self.cached_glyphs.contains(key) {
            self.cached_glyphs.promote(key);
            self.glyphs_in_use.insert(*key);

            let data = self.cached_glyphs.get(&key).unwrap();
            Ok(data)
        }
        // Try to cache glyph
        else {
            let image = match swash_cache.get_image_uncached(font_system, *key) {
                Some(img) => img,

                // No glyph available??
                None => todo!(),
            };

            match self.cache_glypyh(device, queue, key, &image) {
                // Successfully cached and uploaded glyph
                Ok(_) => {
                    self.cached_glyphs.promote(key);
                    self.glyphs_in_use.insert(*key);

                    let data = self.cached_glyphs.get(&key).unwrap();
                    Ok(data)
                }

                // Failed to cache glyph - return error
                Err(_) => todo!(),
            }
        }
    }

    fn cache_glypyh(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: &CacheKey,
        image: &cosmic_text::SwashImage,
    ) -> Result<(), CacheGlyphError> {
        let image_width = image.placement.width;
        let image_height = image.placement.height;

        let size = etagere::Size::new(image_width.max(1) as i32, image_height.max(1) as i32);

        let allocation = loop {
            match self.packer.allocate(size) {
                Some(allocation) => break allocation,

                // Keep trying to free space until error or can allocate
                None => self.free_space(device)?,
            }
        };

        let x = allocation.rectangle.min.x as u32;
        let y = allocation.rectangle.min.y as u32;

        self.texture
            .update_area(queue, &image.data, x, y, image_width, image_height);

        let size = [image_width as f32, image_height as f32];

        let uv_start = [
            allocation.rectangle.min.x as f32 / self.texture_size.width as f32,
            allocation.rectangle.min.y as f32 / self.texture_size.height as f32,
        ];

        let uv_end = [
            allocation.rectangle.max.x as f32 / self.texture_size.width as f32,
            allocation.rectangle.max.y as f32 / self.texture_size.height as f32,
        ];

        log::trace!(
            "Allocated glyph id {:?}, with size {:?} and uv ({:?}, {:?})",
            &key.glyph_id,
            size,
            uv_start,
            uv_end
        );

        let glyph_data = GlyphData {
            alloc_id: allocation.id,
            size,
            uv_start,
            uv_end,
        };

        self.cached_glyphs.put(*key, glyph_data);

        Ok(())
    }

    fn free_space(&mut self, _device: &wgpu::Device) -> Result<(), CacheGlyphError> {
        //
        match self.cached_glyphs.peek_lru() {
            Some((key, _)) => {
                if self.glyphs_in_use.contains(key) {
                    // TODO - Try to grow glyph cache - Make sure to re-set all glyph data UVs
                    todo!("Growing texture atlas not implemented yet")
                }
            }
            None => {
                // Issues with size of lru
                todo!()
            }
        };

        let (key, val) = self.cached_glyphs.pop_lru().unwrap();

        self.packer.deallocate(val.alloc_id);
        self.cached_glyphs.pop(&key);

        return Ok(());
    }
}

//====================================================================
