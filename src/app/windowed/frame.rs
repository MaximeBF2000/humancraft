use super::*;

impl RenderState {
    pub(super) fn update(&mut self) {
        let now = Instant::now();
        let delta_seconds = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        let mut dirty_chunks = Vec::new();
        if self.mode == AppMode::InGame && !self.paused && !self.inventory_open {
            if let Some(world) = self.world.as_mut() {
                self.camera.update(&self.input, world, delta_seconds);
                dirty_chunks.extend(world.ensure_chunks_around_render_position_with_store(
                    self.camera.position,
                    MAX_CHUNK_LOADS_PER_FRAME,
                    &self.save_store,
                ));
            }
            if let Some(button) = self.held_block_interaction.repeat_button(delta_seconds) {
                dirty_chunks.extend(self.apply_block_interaction(button));
            }
            if self.held_block_interaction.is_held(MouseButton::Left) {
                dirty_chunks.extend(self.continue_block_break(delta_seconds));
            } else if let Some(world) = self.world.as_mut() {
                world.clear_block_break_progress();
            }
        }
        if self.mode == AppMode::InGame && !self.paused {
            if let Some(world) = self.world.as_mut() {
                world.update_loot(self.camera.position, delta_seconds);
            }
        }
        if !dirty_chunks.is_empty() {
            self.mark_dirty_chunks_for_save(&dirty_chunks);
            self.queue_chunk_remeshes(&dirty_chunks);
        }
        let remesh_chunks = self.take_pending_chunk_remeshes(MAX_CHUNK_REMESHES_PER_FRAME);
        if !remesh_chunks.is_empty() {
            self.rebuild_chunk_meshes(&remesh_chunks);
        }
        self.update_target_outline();
        let uniform = CameraUniform::new(
            self.camera
                .view_projection(self.config.width, self.config.height),
        );
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub(super) fn apply_block_interaction(&mut self, button: MouseButton) -> Vec<ChunkPosition> {
        let Some(hit) = self
            .world
            .as_ref()
            .and_then(|world| world.raycast(self.camera.position, self.camera.forward()))
        else {
            return Vec::new();
        };

        match button {
            MouseButton::Right => {
                if self
                    .world
                    .as_ref()
                    .and_then(|world| {
                        world
                            .block(hit.block)
                            .and_then(|block| world.blocks.get(block))
                    })
                    .is_some_and(|definition| definition.has_tag("crafting_table"))
                {
                    self.open_crafting_table();
                    return Vec::new();
                }
                self.world
                    .as_mut()
                    .map(|world| {
                        world.place_selected_hotbar_block_for_player(
                            hit.previous,
                            self.selected_hotbar_slot,
                            self.camera.position,
                        )
                    })
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn continue_block_break(&mut self, delta_seconds: f32) -> Vec<ChunkPosition> {
        let Some(world) = self.world.as_mut() else {
            return Vec::new();
        };
        let Some(hit) = world.raycast(self.camera.position, self.camera.forward()) else {
            world.clear_block_break_progress();
            return Vec::new();
        };
        world.continue_breaking_block(hit.block, delta_seconds)
    }

    pub(super) fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let aspect = self.config.width.max(1) as f32 / self.config.height.max(1) as f32;
        let ui_mesh = if self.mode == AppMode::InGame && !self.paused {
            self.world.as_ref().map(|world| {
                build_gameplay_ui_mesh(
                    world,
                    self.inventory_open,
                    self.crafting_ui_kind(),
                    self.active_crafting_grid(),
                    self.crafting_result,
                    aspect,
                    self.selected_hotbar_slot,
                    self.inventory_cursor,
                    cursor_to_ui_point(self.cursor_position, self.size),
                )
            })
        } else if self.mode != AppMode::InGame || self.paused {
            Some(build_menu_mesh(self))
        } else {
            None
        };
        let textured_ui_mesh = if self.mode == AppMode::InGame && !self.paused {
            self.world.as_ref().map(|world| {
                build_inventory_icon_mesh(
                    world,
                    &self.texture_atlas,
                    self.inventory_open,
                    self.crafting_ui_kind(),
                    self.active_crafting_grid(),
                    self.crafting_result,
                    aspect,
                    self.selected_hotbar_slot,
                    self.inventory_cursor,
                    cursor_to_ui_point(self.cursor_position, self.size),
                )
            })
        } else {
            None
        };
        let loot_mesh = if self.mode == AppMode::InGame {
            self.world
                .as_ref()
                .map(|world| build_loot_mesh(world, &self.texture_atlas, &self.camera))
        } else {
            None
        };
        let ui_buffers = ui_mesh.as_ref().and_then(|(vertices, indices)| {
            if vertices.is_empty() || indices.is_empty() {
                return None;
            }
            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Menu Vertex Buffer"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Menu Index Buffer"),
                    contents: bytemuck::cast_slice(indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
            Some((vertex_buffer, index_buffer, indices.len() as u32))
        });
        let textured_ui_buffers = textured_ui_mesh.as_ref().and_then(|(vertices, indices)| {
            if vertices.is_empty() || indices.is_empty() {
                return None;
            }
            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Textured UI Vertex Buffer"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Textured UI Index Buffer"),
                    contents: bytemuck::cast_slice(indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
            Some((vertex_buffer, index_buffer, indices.len() as u32))
        });
        let loot_buffers = loot_mesh.as_ref().and_then(|(vertices, indices)| {
            if vertices.is_empty() || indices.is_empty() {
                return None;
            }
            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Loot Vertex Buffer"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Loot Index Buffer"),
                    contents: bytemuck::cast_slice(indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
            Some((vertex_buffer, index_buffer, indices.len() as u32))
        });

        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.53,
                            g: 0.75,
                            b: 0.95,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if self.mode == AppMode::InGame {
                pass.set_pipeline(&self.render_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(1, &self.texture_bind_group, &[]);
                for chunk_buffer in self.chunk_buffers.values() {
                    if chunk_buffer.index_count == 0 {
                        continue;
                    }
                    pass.set_vertex_buffer(0, chunk_buffer.vertex_buffer.slice(..));
                    pass.set_index_buffer(
                        chunk_buffer.index_buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    pass.draw_indexed(0..chunk_buffer.index_count, 0, 0..1);
                }

                if let Some((vertex_buffer, index_buffer, index_count)) = &loot_buffers {
                    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..*index_count, 0, 0..1);
                }

                if self.outline_vertex_count > 0 {
                    pass.set_pipeline(&self.line_pipeline);
                    pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.outline_vertex_buffer.slice(..));
                    pass.draw(0..self.outline_vertex_count, 0..1);
                }

                if self.block_break_index_count > 0 {
                    pass.set_pipeline(&self.solid_world_overlay_pipeline);
                    pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.block_break_vertex_buffer.slice(..));
                    pass.set_index_buffer(
                        self.block_break_index_buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    pass.draw_indexed(0..self.block_break_index_count, 0, 0..1);
                }
            }

            pass.set_pipeline(&self.ui_pipeline);
            if self.mode == AppMode::InGame && !self.paused {
                pass.set_vertex_buffer(0, self.crosshair_vertex_buffer.slice(..));
                pass.set_index_buffer(
                    self.crosshair_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..self.crosshair_index_count, 0, 0..1);
            }

            if let Some((vertex_buffer, index_buffer, index_count)) = &ui_buffers {
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..*index_count, 0, 0..1);
            }

            if let Some((vertex_buffer, index_buffer, index_count)) = &textured_ui_buffers {
                pass.set_pipeline(&self.textured_ui_pipeline);
                pass.set_bind_group(0, &self.texture_bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..*index_count, 0, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub(super) fn rebuild_chunk_meshes(&mut self, dirty_chunks: &[ChunkPosition]) {
        let Some(world) = &self.world else {
            return;
        };
        for chunk_position in unique_loaded_chunk_positions(dirty_chunks, world) {
            let Some((vertices, indices)) =
                world.build_chunk_render_mesh(chunk_position, &self.texture_atlas)
            else {
                self.chunk_buffers.remove(&chunk_position);
                continue;
            };
            self.chunk_buffers.insert(
                chunk_position,
                ChunkRenderBuffer::new(&self.device, chunk_position, &vertices, &indices),
            );
        }
    }

    pub(super) fn queue_chunk_remeshes(&mut self, dirty_chunks: &[ChunkPosition]) {
        let Some(world) = &self.world else {
            return;
        };
        for chunk_position in unique_loaded_chunk_positions(dirty_chunks, world) {
            self.pending_chunk_remeshes.insert(chunk_position);
        }
    }

    pub(super) fn take_pending_chunk_remeshes(&mut self, limit: usize) -> Vec<ChunkPosition> {
        let mut chunks: Vec<_> = self.pending_chunk_remeshes.iter().copied().collect();
        chunks.sort_by_key(|chunk| {
            let camera_chunk = chunk_position_for_render_position(self.camera.position);
            let dx = (chunk.x - camera_chunk.x).abs();
            let dz = (chunk.z - camera_chunk.z).abs();
            (dx.max(dz), dx + dz, chunk.z, chunk.x)
        });
        chunks.truncate(limit);

        for chunk in &chunks {
            self.pending_chunk_remeshes.remove(chunk);
        }

        chunks
    }

    pub(super) fn update_target_outline(&mut self) {
        self.targeted_block = if self.mode != AppMode::InGame || self.paused {
            None
        } else {
            self.world
                .as_ref()
                .and_then(|world| world.raycast(self.camera.position, self.camera.forward()))
                .map(|hit| hit.block)
        };

        if let Some(block) = self.targeted_block {
            let vertices = build_outline_vertices(block);
            self.queue.write_buffer(
                &self.outline_vertex_buffer,
                0,
                bytemuck::cast_slice(&vertices),
            );
            self.outline_vertex_count = vertices.len() as u32;
        } else {
            self.outline_vertex_count = 0;
        }

        let break_progress = self
            .world
            .as_ref()
            .and_then(|world| world.block_break_progress());
        if let Some(progress) = break_progress {
            let mesh = build_block_break_overlay_mesh(progress.target, progress.ratio);
            self.queue.write_buffer(
                &self.block_break_vertex_buffer,
                0,
                bytemuck::cast_slice(&mesh.vertices),
            );
            self.queue.write_buffer(
                &self.block_break_index_buffer,
                0,
                bytemuck::cast_slice(&mesh.indices),
            );
            self.block_break_index_count = mesh.indices.len() as u32;
        } else {
            self.block_break_index_count = 0;
        }
    }

    pub(super) fn update_crosshair_mesh(&self) {
        let (vertices, _) = build_crosshair_mesh(self.config.width, self.config.height);
        self.queue.write_buffer(
            &self.crosshair_vertex_buffer,
            0,
            bytemuck::cast_slice(&vertices),
        );
    }
}
