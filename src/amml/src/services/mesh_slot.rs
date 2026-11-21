use amml_api::{ApiError, ColorQuad, MeshId, MeshStyle, WorldCanvas};
use levels_core::structs::draw::ColorQuad as CoreQuad;

use crate::services::{LevelsLog, QuadConverter};
use crate::types::{WorldDrawStats, WorldSlot};

/// One slot of the world draw: the meshes of one quad list, split into parts of at most `MAX_QUADS` quads, the most
/// the loader builds in one mesh. Bound: `capacity` meshes, fixed at construction; quads past `capacity * MAX_QUADS`
/// are not drawn. Each plan version is built once. A failed build is tried `MAX_FAILURES` times, then the slot stays
/// empty until the version changes. The loader owns the meshes; the slot keeps their ids and frees them through the
/// canvas of the frame
pub struct MeshSlot {
    slot: WorldSlot,
    capacity: usize,
    meshes: Vec<MeshId>,
    // the version the meshes belong to, built in full or in part
    building: Option<u64>,
    // the version built in full, or given up on
    settled: Option<u64>,
    failures: u32,
    // the first build of the slot was logged and the slot has not been emptied since
    announced: bool,
    // the loader freed the meshes (a graphics mode switch, a world leave): the next build is a rebuild
    stale: bool,
    log: LevelsLog,
}

impl MeshSlot {
    // quads per mesh: the loader's limit
    pub const MAX_QUADS: usize = WorldCanvas::MAX_QUADS;
    // the failed builds of one version before the slot gives up on it
    pub const MAX_FAILURES: u32 = 3;
    // two windings of each quad are written outside Vibrant Visuals
    pub const STYLE: MeshStyle = MeshStyle { two_sided: true, ..MeshStyle::OVERLAY };

    // the meshes a list of `quads` quads takes
    pub const fn meshes_for(quads: usize) -> usize {
        let meshes = quads.div_ceil(Self::MAX_QUADS);
        if meshes == 0 { 1 } else { meshes }
    }

    pub fn new(slot: WorldSlot, capacity: usize, log: LevelsLog) -> Self {
        Self {
            slot,
            capacity,
            meshes: Vec::with_capacity(capacity),
            building: None,
            settled: None,
            failures: 0,
            announced: false,
            stale: false,
            log,
        }
    }

    pub fn slot(&self) -> WorldSlot {
        self.slot
    }

    pub fn meshes(&self) -> &[MeshId] {
        &self.meshes
    }

    /// Builds the meshes of `version` from `quads` when they are not built yet; an empty list empties the slot
    pub fn sync(
        &mut self,
        canvas: &WorldCanvas<'_>,
        version: u64,
        quads: &[CoreQuad],
        scratch: &mut Vec<ColorQuad>,
        stats: &mut WorldDrawStats,
    ) {
        if self.settled == Some(version) {
            return;
        }
        if self.building != Some(version) {
            self.free_all(canvas);
            self.building = Some(version);
            self.failures = 0;
        }
        if quads.is_empty() {
            self.settled = Some(version);
            self.announced = false;
            self.stale = false;
            return;
        }
        let built = self.meshes.len();
        for part in quads.chunks(Self::MAX_QUADS).take(self.capacity).skip(built) {
            QuadConverter::fill(part, scratch);
            match canvas.build(scratch, Self::STYLE) {
                Ok(mesh) => {
                    self.meshes.push(mesh);
                    stats.meshes_built += 1;
                }
                Err(error) => {
                    self.fail(canvas, error);
                    return;
                }
            }
        }
        self.settled = Some(version);
        self.announce(quads.len());
    }

    /// A draw of a mesh of the slot gave `Stale`: the loader freed it. The other meshes of the slot are freed and the
    /// next `sync` builds the slot again
    pub fn mark_stale(&mut self, canvas: &WorldCanvas<'_>) {
        self.free_all(canvas);
        self.settled = None;
        self.building = None;
        self.stale = true;
    }

    /// Frees every mesh and forgets the versions; the slot is logged again at its next build
    pub fn clear(&mut self, canvas: &WorldCanvas<'_>) {
        self.free_all(canvas);
        self.forget();
    }

    /// Forgets every id without a call: the loader freed them at a world leave
    pub fn forget(&mut self) {
        self.meshes.clear();
        self.building = None;
        self.settled = None;
        self.failures = 0;
        self.announced = false;
        self.stale = false;
    }

    // an id the loader already freed answers `Stale`, which is fine here
    fn free_all(&mut self, canvas: &WorldCanvas<'_>) {
        for mesh in self.meshes.drain(..) {
            let _ = canvas.free(mesh);
        }
    }

    fn announce(&mut self, quads: usize) {
        if self.stale {
            self.stale = false;
            self.log.meshes_stale(self.slot);
        }
        if !self.announced {
            self.announced = true;
            self.log.mesh_built(self.slot, quads);
        }
    }

    fn fail(&mut self, canvas: &WorldCanvas<'_>, error: ApiError) {
        self.failures += 1;
        if self.failures == 1 {
            self.log.build_failed(self.slot, &error.to_string(), self.failures);
        }
        if self.failures >= Self::MAX_FAILURES {
            self.free_all(canvas);
            self.settled = self.building;
            self.log.build_given_up(self.slot);
        }
    }
}
