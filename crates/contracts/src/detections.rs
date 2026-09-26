//! Club/ball detection contracts. `contracts/detections.py`. [M22 P2]
//!
//! Reachable from [`crate::swing::SwingResult`] and from nothing else the engine does: M1.5 found
//! club-head detection a no-go at this repo's shutter speed, so the list is empty on every swing on
//! disk and on every committed vector. It is ported anyway because the *field* is on the result and
//! a shape that silently drops a key is the failure the round-trip gate exists to catch.

use serde::{Deserialize, Serialize};

use crate::{each, ge, le, nested, ContractError, Validate};

/// The objects YOLOv8 is trained to detect (ADR-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectClass {
    ClubHead,
    Ball,
}

/// Axis-aligned box in normalized image coordinates (xyxy, each in `[0, 1]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct BoundingBox {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

crate::validated!(BoundingBox);

impl BoundingBox {
    /// Box center — the point strung together into the club-path arc.
    pub fn center(&self) -> (f64, f64) {
        ((self.x1 + self.x2) / 2.0, (self.y1 + self.y2) / 2.0)
    }
}

impl Validate for BoundingBox {
    fn validate(&self) -> Result<(), ContractError> {
        for (name, value) in [
            ("BoundingBox.x1", self.x1),
            ("BoundingBox.y1", self.y1),
            ("BoundingBox.x2", self.x2),
            ("BoundingBox.y2", self.y2),
        ] {
            ge(name, value, 0.0)?;
            le(name, value, 1.0)?;
        }
        Ok(())
    }
}

/// A single detected object in a frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct Detection {
    pub object_class: ObjectClass,
    pub bbox: BoundingBox,
    pub confidence: f64,
    /// Assigned by the tracker (ByteTrack); links the same object across frames so club-head
    /// centers can be connected into a path. `None` before tracking is applied.
    #[serde(default)]
    pub track_id: Option<i64>,
}

crate::validated!(Detection);

impl Validate for Detection {
    fn validate(&self) -> Result<(), ContractError> {
        ge("Detection.confidence", self.confidence, 0.0)?;
        le("Detection.confidence", self.confidence, 1.0)?;
        nested("Detection.bbox", Some(&self.bbox))
    }
}

/// All detections for one video frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct FrameDetections {
    pub frame_index: i64,
    pub timestamp_ms: f64,
    #[serde(default)]
    pub detections: Vec<Detection>,
}

crate::validated!(FrameDetections);

impl FrameDetections {
    /// All detections of a given class in this frame.
    pub fn of(&self, object_class: ObjectClass) -> Vec<&Detection> {
        self.detections
            .iter()
            .filter(|d| d.object_class == object_class)
            .collect()
    }
}

impl Validate for FrameDetections {
    fn validate(&self) -> Result<(), ContractError> {
        ge("FrameDetections.frame_index", self.frame_index, 0)?;
        ge("FrameDetections.timestamp_ms", self.timestamp_ms, 0.0)?;
        each("FrameDetections.detections", &self.detections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_outside_the_frame_is_refused() {
        let json = r#"{"x1":0.0,"y1":0.0,"x2":1.5,"y2":1.0}"#;
        let err = serde_json::from_str::<BoundingBox>(json).unwrap_err();
        assert!(err.to_string().contains("BoundingBox.x2"), "{err}");
    }

    #[test]
    fn the_class_wire_names_are_the_python_values() {
        assert_eq!(
            serde_json::to_string(&ObjectClass::ClubHead).unwrap(),
            "\"club_head\""
        );
        assert_eq!(
            serde_json::to_string(&ObjectClass::Ball).unwrap(),
            "\"ball\""
        );
    }

    #[test]
    fn center_is_the_midpoint() {
        let bbox = BoundingBox {
            x1: 0.25,
            y1: 0.5,
            x2: 0.75,
            y2: 1.0,
        };
        assert_eq!(bbox.center(), (0.5, 0.75));
    }
}
