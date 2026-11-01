use levels_core::structs::draw::ColorQuad;

use crate::services::MeshFactory;
use crate::types::{BuildStall, MeshAttempt, WorldDrawStats, WorldFrame, WorldMesh, WorldSlot};

/// One slot of the world draw: the meshes of one quad list, split into parts of at most `MAX_QUADS` quads, so no
/// mesh needs an index past 16 bits whatever index width the game picks. Bound: `capacity` meshes, fixed at
/// construction; quads past `capacity * MAX_QUADS` are not drawn. Each plan version is built once. A busy build is
/// tried again next frame without a limit; a failed build is tried `MAX_FAILURES` times, then the slot stays empty
/// until the version changes. The meshes are dropped on the render thread only
pub struct MeshSlot {
    slot: WorldSlot,
    capacity: usize,
    meshes: Vec<WorldMesh>,
    // the version the meshes belong to, built in full or in part
    building: Option<u64>,
    // the version built in full, or given up on
    settled: Option<u64>,
    failures: u32,
    busy_frames: u32,
}

impl MeshSlot {
    // quads per mesh: each quad writes 8 vertices, so 8191 quads are 65528 vertices
    pub const MAX_QUADS: usize = 8191;
    // the failed builds of one version before the slot gives up on it
    pub const MAX_FAILURES: u32 = 3;
    // the busy frames of one version before one log line names the cause; about 10 s at 60 frames a second
    pub const STALL_FRAMES: u32 = 600;

    // the meshes a list of `quads` quads takes
    pub const fn meshes_for(quads: usize) -> usize {
        let meshes = quads.div_ceil(Self::MAX_QUADS);
        if meshes == 0 { 1 } else { meshes }
    }

    pub fn new(slot: WorldSlot, capacity: usize) -> Self {
        Self {
            slot,
            capacity,
            meshes: Vec::with_capacity(capacity),
            building: None,
            settled: None,
            failures: 0,
            busy_frames: 0,
        }
    }

    pub fn meshes(&self) -> &[WorldMesh] {
        &self.meshes
    }

    /// Builds the meshes of `version` from `quads` when they are not built yet; an empty list empties the slot
    ///
    /// # Safety
    /// As for `MeshFactory::build`
    #[allow(unsafe_code)]
    pub unsafe fn sync(
        &mut self,
        factory: &mut MeshFactory,
        frame: &WorldFrame<'_>,
        version: u64,
        quads: &[ColorQuad],
        stats: &mut WorldDrawStats,
    ) {
        if self.settled == Some(version) {
            return;
        }
        if self.building != Some(version) {
            self.meshes.clear();
            self.building = Some(version);
            self.failures = 0;
            self.busy_frames = 0;
        }
        let built = self.meshes.len();
        for part in quads.chunks(Self::MAX_QUADS).take(self.capacity).skip(built) {
            // SAFETY: forwarded to the caller
            match unsafe { factory.build(frame, part, self.slot) } {
                MeshAttempt::Built(mesh) => {
                    self.meshes.push(mesh);
                    stats.meshes_built += 1;
                }
                MeshAttempt::Busy(stall) => {
                    self.wait(stall);
                    return;
                }
                MeshAttempt::Failed { vertices } => {
                    self.fail(vertices);
                    return;
                }
            }
        }
        self.settled = Some(version);
    }

    // drops every mesh and forgets the versions
    pub fn reset(&mut self) {
        self.meshes.clear();
        self.building = None;
        self.settled = None;
        self.failures = 0;
        self.busy_frames = 0;
    }

    pub fn resident_bytes(&self) -> u64 {
        self.meshes.iter().map(WorldMesh::resident_bytes).sum()
    }

    fn wait(&mut self, stall: BuildStall) {
        self.busy_frames = self.busy_frames.saturating_add(1);
        if self.busy_frames == Self::STALL_FRAMES {
            curia::warn!("a world mesh build keeps waiting on the game; it is still tried each frame", {
                slot: self.slot.as_str(),
                cause: stall.as_str(),
                frames: self.busy_frames
            });
        }
    }

    fn fail(&mut self, vertices: usize) {
        self.failures += 1;
        if self.failures == 1 {
            curia::warn!("a world mesh build failed; it is tried again next frame", {
                slot: self.slot.as_str(),
                vertices: vertices
            });
        }
        if self.failures >= Self::MAX_FAILURES {
            self.meshes.clear();
            self.settled = self.building;
            curia::error!("a world mesh build failed again; the slot stays empty until its quads change", {
                slot: self.slot.as_str(),
                vertices: vertices,
                tries: self.failures
            });
        }
    }
}
