use levels_core::config::MAX_COLUMN_SURFACES;
use levels_core::services::LabelGrid;
use levels_core::structs::light::SurfaceSample;
use levels_core::structs::world::{BlockPos, ColumnPos};

fn samples(column: ColumnPos, count: usize) -> Vec<SurfaceSample> {
    (0..count)
        .map(|i| SurfaceSample {
            pos: BlockPos { x: column.x, y: 40 + i as i32, z: column.z },
            block_light: 0,
            sky_light: 0,
        })
        .collect()
}

#[test]
fn store_replaces_the_column() {
    let mut grid = LabelGrid::new(24);
    let column = ColumnPos { x: 0, z: 0 };

    grid.store(column, &samples(column, 3));
    grid.store(column, &samples(column, 1));

    assert_eq!(grid.len(), 1);
}

#[test]
fn empty_store_clears_the_column() {
    let mut grid = LabelGrid::new(24);
    let column = ColumnPos { x: 0, z: 0 };

    grid.store(column, &samples(column, 3));
    grid.store(column, &[]);

    assert_eq!(grid.len(), 0);
}

#[test]
fn column_is_capped() {
    let mut grid = LabelGrid::new(24);
    let column = ColumnPos { x: 0, z: 0 };
    let all = samples(column, MAX_COLUMN_SURFACES + 5);

    grid.store(column, &all);

    assert_eq!(grid.len(), MAX_COLUMN_SURFACES);
    let kept: Vec<SurfaceSample> = grid.samples_near(BlockPos { x: 0, y: 40, z: 0 }, 24).copied().collect();
    assert_eq!(kept, all[..MAX_COLUMN_SURFACES]);
}

#[test]
fn same_slot_far_column_evicts() {
    let mut grid = LabelGrid::new(24);
    let near = ColumnPos { x: 0, z: 0 };
    let far = ColumnPos { x: 49, z: 0 };

    grid.store(near, &samples(near, 2));
    grid.store(far, &samples(far, 3));

    let around_far: Vec<SurfaceSample> = grid.samples_near(BlockPos { x: 49, y: 40, z: 0 }, 24).copied().collect();
    assert_eq!(around_far, samples(far, 3));
    assert_eq!(grid.samples_near(BlockPos { x: 0, y: 40, z: 0 }, 24).count(), 0);
    assert_eq!(grid.len(), 3);
}

#[test]
fn teleport_hides_old_area() {
    let mut grid = LabelGrid::new(24);
    for x in -3..=3 {
        for z in -3..=3 {
            let column = ColumnPos { x, z };
            grid.store(column, &samples(column, 2));
        }
    }

    assert_eq!(grid.samples_near(BlockPos { x: 1000, y: 40, z: 1000 }, 24).count(), 0);
}

#[test]
fn clear_drops_every_column() {
    let mut grid = LabelGrid::new(24);
    let column = ColumnPos { x: 5, z: -5 };
    grid.store(column, &samples(column, 4));

    grid.clear();

    assert_eq!(grid.len(), 0);
    assert_eq!(grid.samples_near(BlockPos { x: 5, y: 40, z: -5 }, 24).count(), 0);
}
