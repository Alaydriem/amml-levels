use crate::config::LevelsConfig;
use crate::services::{
    DrawListBuilder, LabelGrid, LabelMeshBuilder, LightClassifier, ScanPlanner, Telemetry, WorldMarkers,
};
use crate::structs::draw::LabelStamp;
use crate::structs::frame::{ColumnRequest, FrameStatus};
use crate::structs::light::SurfaceSample;
use crate::structs::manager::ManagerSignal;
use crate::structs::marker::WorldDrawPlan;
use crate::structs::telemetry::TelemetryReport;
use crate::structs::world::{BlockPos, ColumnPos};

// longest frame step fed to the scan planner, so a hitch or a paused game does not flood one frame
const MAX_FRAME_SECONDS: f64 = 0.1;
const BYTES_PER_MB: u64 = 1024 * 1024;
// the label mesh is relative to the player's block rounded down to a multiple of this
const ORIGIN_SNAP: i32 = 16;
// the label mesh keeps its origin until the player is farther than this from it on an axis, so the origin (and with it
// every part's quads) changes once in many blocks of walking
const ORIGIN_REACH: i32 = 48;

// the on/off state, the world lifecycle and the per-frame flow from scan requests to the draw list
pub struct LevelsController {
    max_columns_per_frame: usize,
    is_on: bool,
    last_status: Option<FrameStatus>,
    planner: ScanPlanner,
    grid: LabelGrid,
    builder: DrawListBuilder,
    mesh: LabelMeshBuilder,
    markers: WorldMarkers,
    is_lease_hidden: bool,
    telemetry: Telemetry,
    // the buffers core allocates once in new()
    core_bytes: u64,
    last_report: Option<TelemetryReport>,
    // 0 when the labels are not drawn this frame
    label_version: u64,
    // label draw list and mesh builds since the controller was made
    label_builds: u64,
    // what the last label build depended on; None when the next frame has to build
    label_stamp: Option<LabelStamp>,
    // bumped when the label look or the light bands change
    style_revision: u64,
    label_origin: BlockPos,
}

impl LevelsController {
    pub fn new(config: LevelsConfig) -> Self {
        let config = config.sanitized();
        let planner = ScanPlanner::new(&config.scan);
        let grid = LabelGrid::new(config.scan.radius);
        let builder = DrawListBuilder::new(&config.scan, config.label, LightClassifier::new(config.light));
        let mesh = LabelMeshBuilder::new(builder.max_labels());
        let markers = WorldMarkers::new(config.borders.clone(), config.spheres.clone());
        let core_bytes = planner.resident_bytes()
            + grid.resident_bytes()
            + builder.resident_bytes()
            + mesh.resident_bytes()
            + markers.resident_bytes();
        let mut telemetry = Telemetry::new(config.telemetry, config.memory_budget_mb as u64 * BYTES_PER_MB);
        telemetry.memory(core_bytes);
        Self {
            max_columns_per_frame: config.scan.max_columns_per_frame as usize,
            is_on: false,
            last_status: None,
            planner,
            grid,
            builder,
            mesh,
            markers,
            is_lease_hidden: false,
            telemetry,
            core_bytes,
            last_report: None,
            label_version: 0,
            label_builds: 0,
            label_stamp: None,
            style_revision: 0,
            label_origin: BlockPos::default(),
        }
    }

    pub fn signal(&mut self, signal: ManagerSignal) {
        match signal {
            ManagerSignal::TogglePressed => self.toggle(),
            ManagerSignal::LeaseLost | ManagerSignal::LeaseSuspended => {
                if !self.is_lease_hidden {
                    curia::info!("overlay hidden by the mod manager", { reason: format!("{signal:?}") });
                }
                self.is_lease_hidden = true;
            }
            ManagerSignal::LeaseRestored => self.is_lease_hidden = false,
            ManagerSignal::Disabled => {
                self.is_lease_hidden = false;
                if self.is_on {
                    curia::info!("overlay stopped by the mod manager", { reason: format!("{signal:?}") });
                    self.set_on(false);
                }
            }
            ManagerSignal::BordersPressed
            | ManagerSignal::SphereDropPressed
            | ManagerSignal::SphereRemovePressed
            | ManagerSignal::SphereClearPressed => {}
        }
        self.markers.signal(signal, self.last_status);
    }

    // The labels, the borders and the spheres for one frame. Some when the labels are on and shown, or
    // when the markers have a plan. A hidden HUD leaves the label quads empty.
    pub fn world_plan(&mut self) -> Option<WorldDrawPlan<'_>> {
        let is_labels_shown = self.is_on && !self.is_lease_hidden;
        let mut label_version = 0;
        if is_labels_shown {
            self.build_labels();
            label_version = self.label_version;
        } else {
            self.telemetry.labels(0);
        }
        let markers = self.markers.plan();
        if !is_labels_shown && markers.is_none() {
            return None;
        }
        let (label_origin, label_quads, label_parts) = if label_version == 0 {
            (BlockPos::default(), &[][..], &[][..])
        } else {
            (self.mesh.origin(), self.mesh.quads(), self.mesh.parts())
        };
        let plan = match markers {
            Some(plan) => WorldDrawPlan { label_origin, label_version, label_quads, label_parts, ..plan },
            None => WorldDrawPlan {
                label_origin,
                label_version,
                label_quads,
                label_parts,
                border_origin: BlockPos::default(),
                border_version: 0,
                border_quads: &[],
                shell_version: 0,
                shell_quads: &[],
                instances: &[],
            },
        };
        Some(plan)
    }

    pub fn is_lease_hidden(&self) -> bool {
        self.is_lease_hidden
    }

    pub fn wants_world_draw(&self) -> bool {
        self.is_on || self.markers.is_active()
    }

    pub fn world_timing(&mut self, build_us: f32, draw_us: f32, meshes_built: u32, quads_converted: u32) {
        self.telemetry.world_timing(build_us, draw_us, meshes_built, quads_converted);
    }

    // applies the settings that take effect at once; scan and memory budget size buffers in new() and keep
    // their values until the next start
    pub fn apply_config(&mut self, config: LevelsConfig) {
        let config = config.sanitized();
        self.builder.restyle(config.label, LightClassifier::new(config.light));
        self.style_revision += 1;
        self.telemetry.set_enabled(config.telemetry);
        self.markers.apply_config(config.borders, config.spheres);
    }

    // the graphics mode the host renders with. Under Vibrant Visuals translucent area builds up over frames, so the
    // spawn shells are drawn as grid lines only and the border lines thinner and fainter
    pub fn set_vibrant(&mut self, vibrant: bool) {
        self.markers.set_vibrant(vibrant);
    }

    pub fn frame(&mut self, status: FrameStatus) -> &[ColumnRequest] {
        let requests: &[ColumnRequest] = if status.in_world && !status.has_player {
            &[]
        } else {
            let dt = self
                .last_status
                .map_or(0.0, |last| (status.time_seconds - last.time_seconds).clamp(0.0, MAX_FRAME_SECONDS));
            self.track_world(&status);
            self.last_status = Some(status);
            // a hidden HUD pauses the scan; the stored labels stay for when it comes back
            if self.is_on && !status.hud_hidden && !self.is_lease_hidden {
                self.planner.plan(status.feet, dt as f32)
            } else {
                &[]
            }
        };
        self.markers.frame(&status);
        Self::record_world(&self.markers, &mut self.telemetry);
        self.telemetry.frame(status.time_seconds, requests.len());
        if let Some(report) = self.telemetry.take_report(status.time_seconds) {
            Self::log_report(&report);
            self.last_report = Some(report);
        }
        requests
    }

    pub fn submit_column(&mut self, column: ColumnPos, samples: &[SurfaceSample]) {
        if self.is_on {
            self.grid.store(column, samples);
        }
    }

    // outside_bytes is the memory the host holds for the overlay
    pub fn timing(&mut self, sample_us: f32, draw_us: f32, outside_bytes: u64) {
        self.telemetry.timing(sample_us, draw_us);
        self.telemetry.memory(self.core_bytes + outside_bytes);
    }

    pub fn last_report(&self) -> Option<TelemetryReport> {
        self.last_report
    }

    pub fn is_on(&self) -> bool {
        self.is_on
    }

    // whether telemetry windows are being kept (the setting `telemetry`)
    pub fn is_telemetry_on(&self) -> bool {
        self.telemetry.is_enabled()
    }

    // the spawn spheres the player has dropped
    pub fn marker_count(&self) -> usize {
        self.markers.marker_count()
    }

    // whether the last frame status had a world loaded
    pub fn is_in_world(&self) -> bool {
        self.last_status.is_some_and(|status| status.in_world)
    }

    // how many times the label draw list and mesh were built
    pub fn label_builds(&self) -> u64 {
        self.label_builds
    }

    pub fn max_labels(&self) -> usize {
        self.builder.max_labels()
    }

    pub fn max_columns_per_frame(&self) -> usize {
        self.max_columns_per_frame
    }

    // builds the label mesh for the last frame status; a hidden HUD draws nothing, as the game would not
    // show the labels
    fn build_labels(&mut self) {
        let Some(status) = self.last_status.filter(|status| !status.hud_hidden) else {
            self.telemetry.labels(0);
            self.label_version = 0;
            self.label_stamp = None;
            return;
        };
        let stamp = LabelStamp {
            grid_revision: self.grid.revision(),
            style_revision: self.style_revision,
            feet: status.feet,
            sky_darken: status.sky_darken,
            quarter: LabelMeshBuilder::quarter_for_yaw(status.yaw),
        };
        if self.label_stamp == Some(stamp) && self.label_version != 0 {
            self.telemetry.labels(self.builder.labels().len());
            return;
        }
        self.label_stamp = Some(stamp);
        let scale = self.builder.scale();
        self.label_builds += 1;
        self.telemetry.label_build();
        self.label_origin = Self::origin_near(self.label_origin, status.feet);
        let labels = self.builder.build(&self.grid, status.feet, status.sky_darken);
        self.telemetry.labels(labels.len());
        self.label_version = self.mesh.build(labels, self.label_origin, status.yaw, scale);
    }

    // `current` while the player is within ORIGIN_REACH of it, else the player's block snapped to ORIGIN_SNAP
    fn origin_near(current: BlockPos, feet: BlockPos) -> BlockPos {
        let near = |a: i32, b: i32| (a - b).abs() <= ORIGIN_REACH;
        if near(feet.x, current.x) && near(feet.y, current.y) && near(feet.z, current.z) {
            return current;
        }
        let snap = |value: i32| value.div_euclid(ORIGIN_SNAP) * ORIGIN_SNAP;
        BlockPos { x: snap(feet.x), y: snap(feet.y), z: snap(feet.z) }
    }

    fn track_world(&mut self, status: &FrameStatus) {
        let Some(last) = self.last_status else {
            return;
        };
        if !status.in_world {
            self.is_lease_hidden = false;
            if self.is_on {
                curia::info!("left the world, overlay off");
                self.set_on(false);
            }
            return;
        }
        if last.in_world && last.dimension != status.dimension {
            curia::info!("dimension changed, rescanning", { dimension: status.dimension });
            self.drop_labels();
        }
    }

    fn toggle(&mut self) {
        let in_world = self.last_status.is_some_and(|status| status.in_world);
        if !in_world {
            curia::info!("toggle ignored: no world loaded");
            return;
        }
        if self.last_status.is_some_and(|status| status.menu_open) {
            return;
        }
        self.set_on(!self.is_on);
    }

    fn set_on(&mut self, is_on: bool) {
        self.is_on = is_on;
        self.drop_labels();
        curia::info!("overlay toggled", { on: is_on });
    }

    fn record_world(markers: &WorldMarkers, telemetry: &mut Telemetry) {
        let (count, quads) = match markers.plan() {
            Some(plan) => {
                (plan.instances.len(), plan.border_quads.len() + plan.instances.len() * plan.shell_quads.len())
            }
            None => (0, 0),
        };
        telemetry.world(count, quads);
    }

    fn drop_labels(&mut self) {
        self.grid.clear();
        self.planner.reset();
    }

    fn log_report(report: &TelemetryReport) {
        let memory_mb = report.memory_bytes as f64 / BYTES_PER_MB as f64;
        let budget_mb = report.budget_bytes as f64 / BYTES_PER_MB as f64;
        curia::info!("telemetry", {
            fps: report.fps,
            max_frame_ms: report.max_frame_ms,
            labels: report.labels,
            columns_per_frame: report.columns_per_frame,
            sample_us: report.sample_us_avg,
            draw_us: report.draw_us_avg,
            markers: report.markers,
            quads_drawn: report.quads_drawn,
            meshes_built: report.meshes_built,
            label_builds_per_s: report.label_builds_per_s,
            quads_converted_per_s: report.quads_converted_per_s,
            world_build_us: report.world_build_us_avg,
            world_draw_us: report.world_draw_us_avg,
            memory_mb: memory_mb,
            budget_mb: budget_mb,
        });
        if report.is_over_budget() {
            curia::warn!("memory over budget", { memory_mb: memory_mb, budget_mb: budget_mb });
        }
    }
}
