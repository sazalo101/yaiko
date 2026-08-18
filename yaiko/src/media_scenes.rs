//! Scene-boundary validation for deterministic editor timeline markers.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
    Empty,
    NotOrdered,
    TooShort,
    Overlap,
    InvalidLabel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneBoundary {
    pub start_ms: u64,
    pub end_ms: u64,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneMarker {
    pub at_ms: u64,
    pub label: String,
}

pub fn validate_scenes(
    mut scenes: Vec<SceneBoundary>,
    minimum_duration_ms: u64,
) -> Result<Vec<SceneMarker>, SceneError> {
    if scenes.is_empty() || minimum_duration_ms == 0 {
        return Err(SceneError::Empty);
    }
    scenes.sort_by_key(|scene| scene.start_ms);
    let mut previous_end = 0;
    let mut markers = Vec::with_capacity(scenes.len());
    for (index, scene) in scenes.iter().enumerate() {
        if scene.end_ms <= scene.start_ms || scene.end_ms - scene.start_ms < minimum_duration_ms {
            return Err(SceneError::TooShort);
        }
        if index > 0 && scene.start_ms < previous_end {
            return Err(SceneError::Overlap);
        }
        if scene.label.as_ref().is_some_and(|label| {
            label.is_empty() || label.len() > 128 || label.chars().any(|c| c.is_control())
        }) {
            return Err(SceneError::InvalidLabel);
        }
        markers.push(SceneMarker {
            at_ms: scene.start_ms,
            label: scene
                .label
                .clone()
                .unwrap_or_else(|| format!("scene-{}", index + 1)),
        });
        previous_end = scene.end_ms;
    }
    Ok(markers)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn produces_deterministic_ordered_markers() {
        let markers = validate_scenes(
            vec![
                SceneBoundary {
                    start_ms: 100,
                    end_ms: 300,
                    label: Some("second".into()),
                },
                SceneBoundary {
                    start_ms: 0,
                    end_ms: 100,
                    label: None,
                },
            ],
            50,
        )
        .unwrap();
        assert_eq!(
            markers,
            vec![
                SceneMarker {
                    at_ms: 0,
                    label: "scene-1".into()
                },
                SceneMarker {
                    at_ms: 100,
                    label: "second".into()
                }
            ]
        );
    }
    #[test]
    fn rejects_short_overlapping_empty_and_invalid_scenes() {
        assert_eq!(validate_scenes(Vec::new(), 1), Err(SceneError::Empty));
        assert_eq!(
            validate_scenes(
                vec![SceneBoundary {
                    start_ms: 0,
                    end_ms: 5,
                    label: None
                }],
                10
            ),
            Err(SceneError::TooShort)
        );
        assert_eq!(
            validate_scenes(
                vec![
                    SceneBoundary {
                        start_ms: 0,
                        end_ms: 100,
                        label: None
                    },
                    SceneBoundary {
                        start_ms: 50,
                        end_ms: 150,
                        label: None
                    }
                ],
                10
            ),
            Err(SceneError::Overlap)
        );
        assert_eq!(
            validate_scenes(
                vec![SceneBoundary {
                    start_ms: 0,
                    end_ms: 100,
                    label: Some("bad\nlabel".into())
                }],
                10
            ),
            Err(SceneError::InvalidLabel)
        );
    }
}
