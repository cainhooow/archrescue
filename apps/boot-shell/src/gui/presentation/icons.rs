//! Line icons drawn at the reference's 24-unit grid.

use tiny_skia::{PathBuilder, Transform};

use super::*;

#[derive(Clone, Copy)]
pub(super) enum Icon {
    Brand,
    Disk,
    Pulse,
    Shield,
    Wrench,
    Check,
    Report,
    Windows,
    History,
    Sliders,
    Refresh,
    Terminal,
    Stop,
    Left,
    Right,
    Down,
    Info,
}

impl Icon {
    pub fn page(section: usize) -> Self {
        match section {
            DASHBOARD | DISKS => Self::Disk,
            DIAGNOSIS => Self::Pulse,
            INTEGRITY_PAGE | BACKUP => Self::Shield,
            REPAIR_PAGE => Self::Wrench,
            VERIFY_PAGE => Self::Check,
            REPORTS => Self::Report,
            deployment::DEPLOY => Self::Windows,
            deployment::RESTORE => Self::History,
            _ => Self::Info,
        }
    }

    pub fn draw(self, scene: &mut Scene<'_>, x: i32, y: i32, size: i32, color: COLORREF) {
        let mut path = PathBuilder::new();
        let lines: &[&[(f32, f32)]] = match self {
            Self::Brand | Self::Shield => {
                path.move_to(12.0, 2.8);
                path.line_to(20.2, 6.0);
                path.line_to(20.2, 11.1);
                path.cubic_to(20.2, 15.6, 16.9, 18.8, 12.0, 21.2);
                path.cubic_to(7.1, 18.8, 3.8, 15.6, 3.8, 11.1);
                path.line_to(3.8, 6.0);
                path.close();
                if matches!(self, Self::Brand) {
                    &[&[(8., 11.5), (16., 11.5)], &[(12., 7.5), (12., 15.5)]]
                } else {
                    &[&[(8., 12.), (11., 15.), (16., 9.)]]
                }
            }
            Self::Disk => {
                path.move_to(4.5, 4.0);
                path.line_to(19.5, 4.0);
                path.quad_to(22.0, 4.0, 22.0, 6.5);
                path.line_to(22.0, 17.5);
                path.quad_to(22.0, 20.0, 19.5, 20.0);
                path.line_to(4.5, 20.0);
                path.quad_to(2.0, 20.0, 2.0, 17.5);
                path.line_to(2.0, 6.5);
                path.quad_to(2.0, 4.0, 4.5, 4.0);
                path.close();
                &[
                    &[(2., 13.), (22., 13.)],
                    &[(6., 16.5), (6.6, 16.5)],
                    &[(10., 16.5), (10.6, 16.5)],
                ]
            }
            Self::Pulse => &[&[
                (2., 12.),
                (7., 12.),
                (10., 4.),
                (14., 20.),
                (17., 12.),
                (22., 12.),
            ]],
            Self::Wrench => {
                path.move_to(14.5, 6.5);
                path.cubic_to(15.0, 3.2, 18.0, 1.5, 21.1, 2.7);
                path.line_to(17.6, 6.5);
                path.line_to(18.2, 8.8);
                path.line_to(20.5, 9.5);
                path.line_to(23.0, 6.9);
                path.cubic_to(23.1, 10.7, 20.1, 13.2, 16.2, 12.1);
                path.line_to(6.3, 22.0);
                path.cubic_to(3.5, 24.3, 0.3, 20.6, 2.9, 18.0);
                path.line_to(13.4, 8.7);
                path.close();
                &[]
            }
            Self::Check => {
                path.push_circle(12.0, 12.0, 9.5);
                &[&[(7.7, 12.3), (10.7, 15.3), (16.2, 9.1)]]
            }
            Self::Info => {
                path.push_circle(12.0, 12.0, 9.5);
                &[&[(12., 11.), (12., 16.)], &[(12., 7.5), (12., 7.7)]]
            }
            Self::Report => &[
                &[
                    (5., 2.),
                    (14., 2.),
                    (19., 7.),
                    (19., 22.),
                    (5., 22.),
                    (5., 2.),
                ],
                &[(14., 2.), (14., 7.), (19., 7.)],
                &[(8., 12.), (16., 12.)],
                &[(8., 16.), (16., 16.)],
            ],
            Self::Windows => &[
                &[(2., 4.), (10., 2.5), (10., 10.8), (2., 10.8), (2., 4.)],
                &[
                    (12.5, 2.1),
                    (22., 0.5),
                    (22., 10.8),
                    (12.5, 10.8),
                    (12.5, 2.1),
                ],
                &[(2., 13.2), (10., 13.2), (10., 21.5), (2., 20.), (2., 13.2)],
                &[
                    (12.5, 13.2),
                    (22., 13.2),
                    (22., 23.5),
                    (12.5, 21.9),
                    (12.5, 13.2),
                ],
            ],
            Self::Refresh | Self::History => {
                path.move_to(3.2, 10.0);
                path.cubic_to(4.8, 0.6, 18.9, 0.7, 20.7, 9.2);
                path.move_to(20.8, 14.0);
                path.cubic_to(19.2, 23.4, 5.1, 23.3, 3.3, 14.8);
                if matches!(self, Self::History) {
                    &[
                        &[(3., 5.), (3., 10.), (8., 10.)],
                        &[(12., 7.), (12., 12.), (16., 14.)],
                    ]
                } else {
                    &[
                        &[(21., 4.), (21., 9.), (16., 9.)],
                        &[(3., 20.), (3., 15.), (8., 15.)],
                    ]
                }
            }
            Self::Sliders => {
                for (x, y) in [(7., 5.), (17., 12.), (10., 19.)] {
                    path.push_circle(x, y, 2.0);
                    path.move_to(2.0, y);
                    path.line_to(x - 2.0, y);
                    path.move_to(x + 2.0, y);
                    path.line_to(22.0, y);
                }
                &[]
            }
            Self::Terminal => &[
                &[(2., 4.), (22., 4.), (22., 20.), (2., 20.), (2., 4.)],
                &[(6., 8.), (10., 12.), (6., 16.)],
                &[(13., 16.), (18., 16.)],
            ],
            Self::Stop => &[&[(4., 4.), (20., 4.), (20., 20.), (4., 20.), (4., 4.)]],
            Self::Left => &[
                &[(12., 6.), (6., 12.), (12., 18.)],
                &[(6., 12.), (20., 12.)],
            ],
            Self::Right => &[
                &[(12., 6.), (18., 12.), (12., 18.)],
                &[(4., 12.), (18., 12.)],
            ],
            Self::Down => &[&[(6., 9.), (12., 15.), (18., 9.)]],
        };

        for line in lines {
            let [(start_x, start_y), rest @ ..] = *line else {
                continue;
            };
            path.move_to(*start_x, *start_y);
            for &(point_x, point_y) in rest {
                path.line_to(point_x, point_y);
            }
        }

        let scale = size as f32 / 24.0;
        let transform = Transform::from_scale(scale, scale).post_translate(x as f32, y as f32);
        if let Some(path) = path.finish().and_then(|path| path.transform(transform)) {
            scene.stroke(&path, color, (1.65 * scale).max(1.0));
        }
    }
}
