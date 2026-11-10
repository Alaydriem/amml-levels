use crate::config::LevelsConfig;
use crate::services::{
    DrawListBuilder, LabelGrid, LabelMeshBuilder, LightClassifier, ScanPlanner, Telemetry, WorldMarkers,
};
use crate::structs::frame::{ColumnRequest, FrameStatus};
use crate::structs::light::SurfaceSample;
use crate::structs::manager::ManagerSignal;
use crate::structs::marker::WorldDrawPlan;
use crate::structs::telemetry::TelemetryReport;
use crate::structs::world::{BlockPos, ColumnPos};

// longest frame step fed to the scan planner, so a hitch or a paused game does not flood one frame
const MAX_FRAME_SECONDS: f64 = 0.1;
const BYTES_PER_MB: u64 = 1024 * 1024;

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
        let (label_origin, label_quads) =
            if label_version == 0 { (BlockPos::default(), &[][..]) } else { (self.mesh.origin(), self.mesh.quads()) };
        let plan = match markers {
            Some(plan) => WorldDrawPlan { label_origin, label_version, label_quads, ..plan },
            None => WorldDrawPlan {
                label_origin,
                label_version,
                label_quads,
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

    pub fn world_timing(&mut self, build_us: f32, draw_us: f32, meshes_built: u32) {
        self.telemetry.world_timing(build_us, draw_us, meshes_built);
    }

    // applies the settings that take effect at once; scan and memory budget size buffers in new() and keep
    // their values until the next start
    pub fn apply_config(&mut self, config: LevelsConfig) {
        let config = config.sanitized();
        self.builder.restyle(config.label, LightClassifier::new(config.light));
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

    // whether the last frame status had a world loaded
    pub fn is_in_world(&self) -> bool {
        self.last_status.is_some_and(|status| status.in_world)
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
            return;
        };
        let scale = self.builder.scale();
        let labels = self.builder.build(&self.grid, status.feet, status.sky_darken);
        self.telemetry.labels(labels.len());
        self.label_version = self.mesh.build(labels, status.yaw, scale);
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
