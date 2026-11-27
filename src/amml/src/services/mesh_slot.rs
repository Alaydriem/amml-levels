use amml_api::{ApiError, ColorQuad, MeshId, MeshStyle, WorldCanvas};
use levels_core::structs::draw::{ColorQuad as CoreQuad, LabelPart};

use crate::services::{LevelsLog, QuadConverter};
use crate::types::{WorldDrawStats, WorldSlot};

/// One slot of the world draw: the meshes of one quad list, split into parts of at most `MAX_QUADS` quads, the most
/// the loader builds in one mesh. Each mesh is kept with the key of the quads it was built from, so a new plan version
/// builds only the parts whose key is new and frees the meshes whose key is gone. Bound: `capacity` meshes, fixed at
/// construction; parts past `capacity` are not drawn. Each plan version is built once. A failed build is tried
/// `MAX_FAILURES` times, then the slot stays empty until the version changes. The loader owns the meshes; the slot
/// keeps their ids and frees them through the canvas of the frame
pub struct MeshSlot {
    slot: WorldSlot,
    capacity: usize,
    meshes: Vec<MeshId>,
    // the key of the quads of each mesh, in the order of `meshes`
    keys: Vec<u64>,
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
            keys: Vec::with_capacity(capacity),
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

    /// Builds the meshes of `version` from `quads`, cut into parts of `MAX_QUADS`, when they are not built yet; an empty
    /// list empties the slot. A new version replaces every mesh
    pub fn sync(
        &mut self,
        canvas: &WorldCanvas<'_>,
        version: u64,
        quads: &[CoreQuad],
        scratch: &mut Vec<ColorQuad>,
        stats: &mut WorldDrawStats,
    ) {
        let part = |index: usize| {
            let start = (index * Self::MAX_QUADS).min(quads.len());
            (Self::chunk_key(version, index), &quads[start..(start + Self::MAX_QUADS).min(quads.len())])
        };
        self.apply(canvas, version, quads.len(), quads.len().div_ceil(Self::MAX_QUADS), part, scratch, stats);
    }

    /// Builds the meshes of `version` from `quads` cut into `parts`; a part whose key a mesh of the slot already has
    /// keeps that mesh, and the meshes whose key is not in `parts` are freed
    pub fn sync_parts(
        &mut self,
        canvas: &WorldCanvas<'_>,
        version: u64,
        quads: &[CoreQuad],
        parts: &[LabelPart],
        scratch: &mut Vec<ColorQuad>,
        stats: &mut WorldDrawStats,
    ) {
        let part = |index: usize| {
            let part = parts[index];
            (part.key, quads.get(part.start..part.end).unwrap_or(&[]))
        };
        self.apply(canvas, version, quads.len(), parts.len(), part, scratch, stats);
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
        self.keys.clear();
        self.building = None;
        self.settled = None;
        self.failures = 0;
        self.announced = false;
        self.stale = false;
    }

    // the key of a part of a plain split: unique for a version below 2^44 and a part index below 2^20
    fn chunk_key(version: u64, index: usize) -> u64 {
        version.wrapping_shl(20) | index as u64
    }

    #[allow(clippy::too_many_arguments)]
    fn apply<'q>(
        &mut self,
        canvas: &WorldCanvas<'_>,
        version: u64,
        total_quads: usize,
        parts: usize,
        part: impl Fn(usize) -> (u64, &'q [CoreQuad]),
        scratch: &mut Vec<ColorQuad>,
        stats: &mut WorldDrawStats,
    ) {
        if self.settled == Some(version) {
            return;
        }
        let parts = parts.min(self.capacity);
        if self.building != Some(version) {
            self.free_unused(canvas, parts, &part);
            self.building = Some(version);
            self.failures = 0;
        }
        if total_quads == 0 {
            self.settled = Some(version);
            self.announced = false;
            self.stale = false;
            return;
        }
        for index in 0..parts {
            let (key, quads) = part(index);
            if self.keys.contains(&key) {
                continue;
            }
            QuadConverter::fill(quads, scratch);
            stats.quads_converted += quads.len() as u32;
            match canvas.build(scratch, Self::STYLE) {
                Ok(mesh) => {
                    self.meshes.push(mesh);
                    self.keys.push(key);
                    stats.meshes_built += 1;
                }
                Err(error) => {
                    self.fail(canvas, error);
                    return;
                }
            }
        }
        self.settled = Some(version);
        self.announce(total_quads);
    }

    // frees the meshes whose key none of the first `parts` parts has
    fn free_unused<'q>(
        &mut self,
        canvas: &WorldCanvas<'_>,
        parts: usize,
        part: &impl Fn(usize) -> (u64, &'q [CoreQuad]),
    ) {
        let mut kept = 0;
        for index in 0..self.keys.len() {
            let key = self.keys[index];
            if (0..parts).any(|wanted| part(wanted).0 == key) {
                self.keys.swap(kept, index);
                self.meshes.swap(kept, index);
                kept += 1;
            }
        }
        self.keys.truncate(kept);
        for mesh in self.meshes.drain(kept..) {
            let _ = canvas.free(mesh);
        }
    }

    // an id the loader already freed answers `Stale`, which is fine here
    fn free_all(&mut self, canvas: &WorldCanvas<'_>) {
        self.keys.clear();
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
