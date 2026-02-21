use super::ship::{Ship, SENSOR_RANGE};
use super::projectile::Projectile;

/// Returns true if `target` is within sensor range of `observer`.
pub fn is_visible(observer: &Ship, target: &Ship) -> bool {
    observer.position.distance_to(target.position) <= SENSOR_RANGE
}

/// Filter projectiles to only those within sensor range of the observer.
pub fn visible_projectiles(observer: &Ship, projectiles: &[Projectile]) -> Vec<Projectile> {
    projectiles
        .iter()
        .filter(|p| observer.position.distance_to(p.position) <= SENSOR_RANGE)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Vec2;

    #[test]
    fn ship_visible_within_range() {
        let a = Ship::new(Vec2::new(100.0, 100.0), 0.0);
        let b = Ship::new(Vec2::new(200.0, 100.0), 0.0);
        assert!(is_visible(&a, &b));
    }

    #[test]
    fn ship_not_visible_out_of_range() {
        let a = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        let b = Ship::new(Vec2::new(500.0, 500.0), 0.0);
        assert!(!is_visible(&a, &b));
    }
}
