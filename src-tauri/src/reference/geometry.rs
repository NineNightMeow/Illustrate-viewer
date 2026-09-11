use super::persistence::{MonitorIdentity, ReferenceGeometry};

const MIN_WIDTH_LOGICAL: f64 = 160.0;
const MIN_HEIGHT_LOGICAL: f64 = 120.0;
const MIN_VISIBLE_WIDTH_LOGICAL: f64 = 96.0;
const MIN_VISIBLE_HEIGHT_LOGICAL: f64 = 48.0;
const INITIAL_MAX_WORK_AREA_RATIO: f64 = 0.70;

#[derive(Clone, Debug, PartialEq)]
pub struct MonitorWorkArea {
    pub name: Option<String>,
    pub position_x: i32,
    pub position_y: i32,
    pub width: u32,
    pub height: u32,
    pub work_x: i32,
    pub work_y: i32,
    pub work_width: u32,
    pub work_height: u32,
    pub scale_factor: f64,
}

/// Calculates a first-open window size in logical pixels while preserving the
/// image aspect ratio. Persisted geometry is intentionally handled by the
/// caller before this helper is used.
pub fn initial_window_size(
    image_width: u32,
    image_height: u32,
    monitor: &MonitorWorkArea,
) -> (f64, f64) {
    if image_width == 0 || image_height == 0 || !monitor.is_valid() {
        return (MIN_WIDTH_LOGICAL, MIN_HEIGHT_LOGICAL);
    }

    let work_width = f64::from(monitor.work_width) / monitor.scale_factor;
    let work_height = f64::from(monitor.work_height) / monitor.scale_factor;
    let max_width = work_width * INITIAL_MAX_WORK_AREA_RATIO;
    let max_height = work_height * INITIAL_MAX_WORK_AREA_RATIO;
    if !max_width.is_finite() || !max_height.is_finite() || max_width <= 0.0 || max_height <= 0.0 {
        return (MIN_WIDTH_LOGICAL, MIN_HEIGHT_LOGICAL);
    }

    let aspect_ratio = f64::from(image_width) / f64::from(image_height);
    let mut width = f64::from(image_width) / monitor.scale_factor;
    let mut height = f64::from(image_height) / monitor.scale_factor;
    let downscale = (max_width / width).min(max_height / height).min(1.0);
    width *= downscale;
    height *= downscale;

    // Small images get only the minimum size needed for a usable window.
    // Keep the scale uniform and never grow beyond the work-area cap.
    let min_scale = (MIN_WIDTH_LOGICAL / width).max(MIN_HEIGHT_LOGICAL / height);
    let up_scale = min_scale.min(max_width / width).min(max_height / height);
    if up_scale.is_finite() && up_scale > 1.0 {
        width *= up_scale;
        height *= up_scale;
    }

    // Guard against numerical drift while retaining the source ratio.
    width = width.min(max_width);
    height = height.min(max_height);
    if aspect_ratio.is_finite() && aspect_ratio > 0.0 {
        height = width / aspect_ratio;
        if height > max_height {
            height = max_height;
            width = height * aspect_ratio;
        }
    }

    (width.max(1.0), height.max(1.0))
}

impl MonitorWorkArea {
    pub fn identity(&self) -> MonitorIdentity {
        MonitorIdentity {
            name: self.name.clone().filter(|name| !name.is_empty()),
            position_x: self.position_x,
            position_y: self.position_y,
            width: self.width,
            height: self.height,
            scale_factor: self.scale_factor,
        }
    }

    fn is_valid(&self) -> bool {
        self.width > 0
            && self.height > 0
            && self.work_width > 0
            && self.work_height > 0
            && self.scale_factor.is_finite()
            && self.scale_factor > 0.0
            && self.scale_factor <= 8.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalWindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub fn capture_geometry(
    monitor: &MonitorWorkArea,
    position_x: i32,
    position_y: i32,
    width: u32,
    height: u32,
) -> Option<ReferenceGeometry> {
    if !monitor.is_valid() {
        return None;
    }

    let scale_factor = monitor.scale_factor;
    let geometry = ReferenceGeometry {
        // These values are logical pixels relative to this monitor's current work area.
        // They are never virtual-desktop coordinates.
        x: (f64::from(position_x) - f64::from(monitor.work_x)) / scale_factor,
        y: (f64::from(position_y) - f64::from(monitor.work_y)) / scale_factor,
        width: f64::from(width) / scale_factor,
        height: f64::from(height) / scale_factor,
        scale_factor,
        monitor: Some(monitor.identity()),
    };
    super::persistence::geometry_is_valid(&geometry).then_some(geometry)
}

pub fn restore_geometry(
    geometry: &ReferenceGeometry,
    monitors: &[MonitorWorkArea],
    primary_monitor: Option<&MonitorWorkArea>,
) -> Option<PhysicalWindowGeometry> {
    if !super::persistence::geometry_is_valid(geometry) {
        return None;
    }

    let monitor = select_monitor(geometry.monitor.as_ref()?, monitors, primary_monitor)?;
    let scale_factor = monitor.scale_factor;
    let width = clamp_size(
        geometry.width,
        MIN_WIDTH_LOGICAL,
        monitor.work_width,
        scale_factor,
    );
    let height = clamp_size(
        geometry.height,
        MIN_HEIGHT_LOGICAL,
        monitor.work_height,
        scale_factor,
    );
    let visible_width = logical_to_physical(MIN_VISIBLE_WIDTH_LOGICAL, scale_factor)
        .min(width)
        .min(monitor.work_width);
    let visible_height = logical_to_physical(MIN_VISIBLE_HEIGHT_LOGICAL, scale_factor)
        .min(height)
        .min(monitor.work_height);

    Some(PhysicalWindowGeometry {
        x: clamp_axis(
            i64::from(monitor.work_x) + logical_to_physical_signed(geometry.x, scale_factor),
            i64::from(monitor.work_x),
            i64::from(monitor.work_width),
            i64::from(width),
            i64::from(visible_width),
        ),
        y: clamp_axis(
            i64::from(monitor.work_y) + logical_to_physical_signed(geometry.y, scale_factor),
            i64::from(monitor.work_y),
            i64::from(monitor.work_height),
            i64::from(height),
            i64::from(visible_height),
        ),
        width,
        height,
    })
}

fn select_monitor<'a>(
    identity: &MonitorIdentity,
    monitors: &'a [MonitorWorkArea],
    primary_monitor: Option<&'a MonitorWorkArea>,
) -> Option<&'a MonitorWorkArea> {
    let named = identity.name.as_deref().filter(|name| !name.is_empty());
    let exact_signature = |monitor: &&MonitorWorkArea| {
        monitor.position_x == identity.position_x
            && monitor.position_y == identity.position_y
            && monitor.width == identity.width
            && monitor.height == identity.height
    };

    if let Some(name) = named {
        if let Some(monitor) = monitors
            .iter()
            .find(|monitor| monitor.name.as_deref() == Some(name) && exact_signature(monitor))
        {
            return Some(monitor);
        }
        if let Some(monitor) = monitors
            .iter()
            .filter(|monitor| monitor.name.as_deref() == Some(name))
            .min_by_key(|monitor| monitor_signature_distance(monitor, identity))
        {
            return Some(monitor);
        }
    }
    if let Some(monitor) = monitors.iter().find(exact_signature) {
        return Some(monitor);
    }

    primary_monitor.or_else(|| monitors.first())
}

fn monitor_signature_distance(monitor: &MonitorWorkArea, identity: &MonitorIdentity) -> u64 {
    u64::from(monitor.position_x.abs_diff(identity.position_x))
        + u64::from(monitor.position_y.abs_diff(identity.position_y))
        + u64::from(monitor.width.abs_diff(identity.width))
        + u64::from(monitor.height.abs_diff(identity.height))
        + ((monitor.scale_factor - identity.scale_factor).abs() * 1_000.0).round() as u64
}

fn clamp_size(logical_size: f64, minimum_logical: f64, work_size: u32, scale_factor: f64) -> u32 {
    logical_to_physical(logical_size.max(minimum_logical), scale_factor)
        .min(work_size)
        .max(1)
}

fn logical_to_physical(value: f64, scale_factor: f64) -> u32 {
    (value * scale_factor)
        .round()
        .clamp(1.0, f64::from(u32::MAX)) as u32
}

fn logical_to_physical_signed(value: f64, scale_factor: f64) -> i64 {
    (value * scale_factor).round() as i64
}

fn clamp_axis(
    requested: i64,
    work_start: i64,
    work_size: i64,
    window_size: i64,
    visible_size: i64,
) -> i32 {
    let lower = work_start + visible_size - window_size;
    let upper = work_start + work_size - visible_size;
    requested
        .clamp(lower, upper)
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests {
    use super::{capture_geometry, initial_window_size, restore_geometry, MonitorWorkArea};

    fn monitor(name: &str, x: i32, scale_factor: f64) -> MonitorWorkArea {
        MonitorWorkArea {
            name: Some(name.into()),
            position_x: x,
            position_y: 0,
            width: 1_920,
            height: 1_080,
            work_x: x,
            work_y: 0,
            work_width: 1_920,
            work_height: 1_040,
            scale_factor,
        }
    }

    #[test]
    fn round_trips_logical_geometry_at_all_supported_dpi_scales() {
        for scale_factor in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let display = MonitorWorkArea {
                width: 3_840,
                height: 2_160,
                work_width: 3_840,
                work_height: 2_120,
                ..monitor("Display A", 0, scale_factor)
            };
            let geometry = capture_geometry(
                &display,
                (120.0 * scale_factor).round() as i32,
                (80.0 * scale_factor).round() as i32,
                (720.0 * scale_factor).round() as u32,
                (540.0 * scale_factor).round() as u32,
            )
            .expect("captured geometry should be valid");
            let restored =
                restore_geometry(&geometry, std::slice::from_ref(&display), Some(&display))
                    .expect("geometry should restore");

            assert_eq!(restored.width, (720.0 * scale_factor).round() as u32);
            assert_eq!(restored.height, (540.0 * scale_factor).round() as u32);
        }
    }

    #[test]
    fn follows_the_same_named_monitor_after_layout_changes() {
        let original = monitor("Display B", 1_920, 1.25);
        let geometry = capture_geometry(&original, 2_120, 100, 900, 675).unwrap();
        let moved = monitor("Display B", -2_560, 1.5);

        let restored =
            restore_geometry(&geometry, std::slice::from_ref(&moved), Some(&moved)).unwrap();

        assert_eq!(restored.x, -2_320);
        assert_eq!(restored.width, 1_080);
    }

    #[test]
    fn disambiguates_same_named_monitors_with_their_position_signature() {
        let original = monitor("Generic monitor", 1_920, 1.0);
        let geometry = capture_geometry(&original, 2_120, 100, 720, 540).unwrap();
        let other = monitor("Generic monitor", 0, 1.0);

        let restored = restore_geometry(&geometry, &[other, original], None).unwrap();

        assert_eq!(restored.x, 2_120);
    }

    #[test]
    fn missing_monitor_falls_back_to_primary_and_keeps_a_grab_area_visible() {
        let removed = monitor("Removed display", 1_920, 1.0);
        let mut geometry = capture_geometry(&removed, 20_000, 20_000, 720, 540).unwrap();
        geometry.x = 20_000.0;
        geometry.y = 20_000.0;
        let primary = monitor("Primary", 0, 1.0);

        let restored =
            restore_geometry(&geometry, std::slice::from_ref(&primary), Some(&primary)).unwrap();

        assert_eq!(restored.x, 1_824);
        assert_eq!(restored.y, 992);
    }

    #[test]
    fn clamps_size_when_resolution_or_work_area_shrinks() {
        let original = monitor("Display A", 0, 1.0);
        let mut geometry = capture_geometry(&original, 100, 100, 1_800, 1_000).unwrap();
        geometry.width = 1_800.0;
        geometry.height = 1_000.0;
        let smaller = MonitorWorkArea {
            work_width: 900,
            work_height: 600,
            ..monitor("Display A", 0, 1.0)
        };

        let restored =
            restore_geometry(&geometry, std::slice::from_ref(&smaller), Some(&smaller)).unwrap();

        assert_eq!((restored.width, restored.height), (900, 600));
    }

    #[test]
    fn initial_size_preserves_common_image_aspects_within_work_area() {
        let display = monitor("Display A", 0, 1.0);
        for (image_width, image_height) in [(1200, 2400), (4000, 2000), (1600, 1600)] {
            let (width, height) = initial_window_size(image_width, image_height, &display);
            assert!(width <= f64::from(display.work_width) * 0.70 + 0.01);
            assert!(height <= f64::from(display.work_height) * 0.70 + 0.01);
            assert!(
                (width / height - f64::from(image_width) / f64::from(image_height)).abs() < 0.001
            );
        }
    }

    #[test]
    fn initial_size_handles_extreme_aspects_and_small_images() {
        let display = monitor("Display A", 0, 1.5);
        let (wide_width, wide_height) = initial_window_size(8_000, 500, &display);
        let (tall_width, tall_height) = initial_window_size(500, 8_000, &display);
        let (small_width, small_height) = initial_window_size(80, 60, &display);

        for (width, height) in [(wide_width, wide_height), (tall_width, tall_height)] {
            assert!(width <= f64::from(display.work_width) / display.scale_factor * 0.70 + 0.01);
            assert!(height <= f64::from(display.work_height) / display.scale_factor * 0.70 + 0.01);
        }
        assert!(small_width < 400.0 && small_height < 400.0);
        assert!(small_width >= 160.0 || small_height >= 120.0);
    }

    #[test]
    fn initial_size_converts_physical_work_area_to_logical_pixels() {
        let display = monitor("Display A", 0, 2.0);
        let (width, height) = initial_window_size(1920, 1080, &display);
        assert!((width - (364.0 * 16.0 / 9.0)).abs() < 0.01);
        assert!((height - 364.0).abs() < 0.01);
    }
}
