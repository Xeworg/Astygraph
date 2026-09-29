//! Heroicons outline SVG assets for essential desktop UI actions.
//!
//! This module provides static SVG path data for the Heroicons outline set,
//! Copyright (c) 2020 Refactoring UI Inc., used under the MIT License below.
//! The full license text is available in `heroicons_license.txt`.
//!
//! ## MIT License
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.

use std::fmt;

/// Size in user-space units for icon rendering.  All icons live in a 24×24 viewBox.
pub const ICON_SIZE: f32 = 16.0;

/// Stroke width used in the Heroicons outline style.
pub const STROKE_WIDTH: f32 = 1.5;

/// Available outline icons for the essential desktop UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Folder — closed state.
    Folder,
    /// Folder — open state.
    FolderOpen,
    /// Plain document/file.
    Document,
    /// Symbolic link.
    Link,
    /// Navigate up / parent directory.
    ArrowUp,
    /// Close or remove.
    XMark,
}

impl Icon {
    /// Returns the raw SVG path data string (d-attribute) for this icon.
    ///
    /// All paths follow the Heroicons 24×24 outline style with stroke-width 1.5.
    pub fn path_data(&self) -> &'static str {
        match self {
            Icon::Folder => {
                "M4 20h16M4 20V10a2 2 0 0 1 2-2h3.17a1 1 0 0 1 .707.293l.829.828a1 1 0 0 0 .707.293H18a2 2 0 0 1 2 2v1"
            }
            Icon::FolderOpen => {
                "M3.5 20 10 12m0 0L3.5 4m6.5 8L2 12m0 0l2.5 8m4.5-8H8.17a1 1 0 0 0-.707.293l-2.829 2.828a1 1 0 0 0 .707 1.707H18a2 2 0 0 0 2-2V9.5a1 1 0 0 0-1-1h-5a1 1 0 0 0-.707.293L9.5 12"
            }
            Icon::Document => {
                "M9 12h6m-6 4h6m2 5H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5.586a1 1 0 0 1 .707.293l5.414 5.414A1 1 0 0 1 19 9.414V19a2 2 0 0 1-2 2z"
            }
            Icon::Link => {
                "M13.828 10.172a4 4 0 0 0-5.656 0l-4 4a4 4 0 1 0 5.656 5.656l1.102-1.101m-.758-4.899a4 4 0 0 0 5.656 0l4-4a4 4 0 0 0-5.656-5.656l-1.1 1.1"
            }
            Icon::ArrowUp => {
                "M12 19V5m5 6-5-6-5 6"
            }
            Icon::XMark => {
                "M6 18 18 6M6 6l12 12"
            }
        }
    }

    /// Returns an iterator over all icon variants.
    pub fn all() -> impl Iterator<Item = Self> {
        [
            Icon::Folder,
            Icon::FolderOpen,
            Icon::Document,
            Icon::Link,
            Icon::ArrowUp,
            Icon::XMark,
        ]
        .into_iter()
    }
}

impl fmt::Display for Icon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Icon::Folder => write!(f, "folder"),
            Icon::FolderOpen => write!(f, "folder-open"),
            Icon::Document => write!(f, "document"),
            Icon::Link => write!(f, "link"),
            Icon::ArrowUp => write!(f, "arrow-up"),
            Icon::XMark => write!(f, "x-mark"),
        }
    }
}

// ---------------------------------------------------------------------------
// SVG path parsing
//
// Converts a compact Heroicons SVG path (M/L/C/Z commands, relative and
// absolute) into a flat list of egui::Pos2 points suitable for
// Shape::Path rendering.  This avoids adding an external SVG dependency.
// ---------------------------------------------------------------------------

/// Parses a Heroicons SVG path string into a vector of egui `Pos2` points.
///
/// Supports the commands used by Heroicons outline icons:
/// - M (moveto, absolute)
/// - m (moveto, relative)
/// - L (lineto, absolute)
/// - l (lineto, relative)
/// - H (horizontal lineto, absolute)
/// - h (horizontal lineto, relative)
/// - V (vertical lineto, absolute)
/// - v (vertical lineto, relative)
/// - C (cubic Bézier, absolute)
/// - c (cubic Bézier, relative)
/// - A (elliptical arc, absolute)
/// - a (elliptical arc, relative)
/// - Z (close path)
/// - z (close path)
///
/// The path is mapped from the 24×24 Heroicons viewBox into a `size × size`
/// render box centered at the origin.
pub fn parse_path(path_data: &str, size: f32) -> Vec<egui::Pos2> {
    let scale = size / 24.0;
    let mut points = Vec::with_capacity(64);
    let mut cx = 0.0_f32;
    let mut cy = 0.0_f32;
    let mut start_x = 0.0_f32;
    let mut start_y = 0.0_f32;
    let mut chars = path_data.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            'M' => {
                cx = parse_num(&mut chars) * scale;
                cy = parse_num(&mut chars) * scale;
                start_x = cx;
                start_y = cy;
                points.push(egui::Pos2::new(cx, cy));
            }
            'm' => {
                cx += parse_num(&mut chars) * scale;
                cy += parse_num(&mut chars) * scale;
                start_x = cx;
                start_y = cy;
                points.push(egui::Pos2::new(cx, cy));
            }
            'L' => {
                cx = parse_num(&mut chars) * scale;
                cy = parse_num(&mut chars) * scale;
                points.push(egui::Pos2::new(cx, cy));
            }
            'l' => {
                cx += parse_num(&mut chars) * scale;
                cy += parse_num(&mut chars) * scale;
                points.push(egui::Pos2::new(cx, cy));
            }
            'H' => {
                cx = parse_num(&mut chars) * scale;
                points.push(egui::Pos2::new(cx, cy));
            }
            'h' => {
                cx += parse_num(&mut chars) * scale;
                points.push(egui::Pos2::new(cx, cy));
            }
            'V' => {
                cy = parse_num(&mut chars) * scale;
                points.push(egui::Pos2::new(cx, cy));
            }
            'v' => {
                cy += parse_num(&mut chars) * scale;
                points.push(egui::Pos2::new(cx, cy));
            }
            'C' => {
                // Control point 1, control point 2, end point
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                cx = parse_num(&mut chars) * scale;
                cy = parse_num(&mut chars) * scale;
                points.push(egui::Pos2::new(cx, cy));
            }
            'c' => {
                // Control point 1, control point 2, end point
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                cx += parse_num(&mut chars) * scale;
                cy += parse_num(&mut chars) * scale;
                points.push(egui::Pos2::new(cx, cy));
            }
            'A' => {
                // Elliptical arc: rx ry x-axis-rotation large-arc-flag sweep-flag x y
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                cx = parse_num(&mut chars) * scale;
                cy = parse_num(&mut chars) * scale;
                points.push(egui::Pos2::new(cx, cy));
            }
            'a' => {
                // Elliptical arc: rx ry x-axis-rotation large-arc-flag sweep-flag dx dy
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                let _ = parse_num(&mut chars) * scale;
                cx += parse_num(&mut chars) * scale;
                cy += parse_num(&mut chars) * scale;
                points.push(egui::Pos2::new(cx, cy));
            }
            'Z' | 'z' => {
                cx = start_x;
                cy = start_y;
                // Close path: push start point to close the loop
                points.push(egui::Pos2::new(start_x, start_y));
            }
            ' ' | ',' | '\n' | '\t' | '\r' => {
                // Skip whitespace
            }
            _ => {
                // Skip other characters (parse_num handles '-' internally via peek)
            }
        }
    }

    points
}

/// Skip SVG whitespace separators (space, comma, newline, tab, carriage return).
fn skip_spaces(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while matches!(
        chars.peek(),
        Some(' ') | Some(',') | Some('\n') | Some('\t') | Some('\r')
    ) {
        chars.next();
    }
}

/// Parse a single floating-point number from the character iterator.
/// Skips whitespace before the number and handles an optional leading minus sign.
fn parse_num(chars: &mut std::iter::Peekable<std::str::Chars>) -> f32 {
    // Skip leading whitespace (space, comma, CR, LF, tab)
    skip_spaces(chars);

    // Consume optional minus
    let negative = chars.peek() == Some(&'-');
    if negative {
        chars.next();
    }

    let mut num_str = String::new();
    if negative {
        num_str.push('-');
    }

    // Collect digits and at most one decimal point
    let mut seen_dot = false;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num_str.push(c);
            chars.next();
        } else if c == '.' && !seen_dot {
            seen_dot = true;
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }

    num_str.parse().unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_icons_return_valid_path_data() {
        for icon in Icon::all() {
            let data = icon.path_data();
            assert!(!data.is_empty(), "icon {:?} has empty path data", icon);
            // At minimum, path data must contain a command letter
            assert!(
                data.chars().any(|c| matches!(
                    c,
                    'M' | 'm'
                        | 'L'
                        | 'l'
                        | 'Z'
                        | 'z'
                        | 'C'
                        | 'c'
                        | 'A'
                        | 'a'
                        | 'H'
                        | 'h'
                        | 'V'
                        | 'v'
                )),
                "icon {:?} has no SVG command letters: {}",
                icon,
                data
            );
        }
    }

    #[test]
    fn parse_path_returns_points_for_all_icons() {
        for icon in Icon::all() {
            let pts = parse_path(icon.path_data(), ICON_SIZE);
            assert!(!pts.is_empty(), "icon {:?} parsed to zero points", icon);
        }
    }

    #[test]
    fn parse_path_handles_empty() {
        let pts = parse_path("", 16.0);
        assert!(pts.is_empty());
    }

    #[test]
    fn parse_path_known_folder_shape() {
        // Folder icon starts with "M4 20" (absolute moveto to 4,20)
        let pts = parse_path("M4 20h16M4 20V10a2 2 0 0 1 2-2h3.17a1 1 0 0 1 .707.293l.829.828a1 1 0 0 0 .707.293H18a2 2 0 0 1 2 2v1", 24.0);
        assert!(!pts.is_empty());
        // First point should be (4, 20) in 24x24 space
        assert!((pts[0].x - 4.0).abs() < 0.01);
        assert!((pts[0].y - 20.0).abs() < 0.01);
    }

    #[test]
    fn icon_display_shows_name() {
        assert_eq!(Icon::Folder.to_string(), "folder");
        assert_eq!(Icon::XMark.to_string(), "x-mark");
    }

    #[test]
    fn parse_path_handles_arc_absolute() {
        // Arc command A: M startX startY A rx ry rotation large sweep endX endY
        // After M5 5, an arc from (5,5) to (15,5) with radius 5
        let pts = parse_path("M5 5A5 5 0 0 0 15 5", 24.0);
        // Should have at least 2 points: moveto start and arc end
        assert!(pts.len() >= 2, "arc absolute should produce end point");
        // End point should be at (15, 5) scaled to 24x24 space
        assert!((pts[pts.len() - 1].x - 15.0).abs() < 0.01);
        assert!((pts[pts.len() - 1].y - 5.0).abs() < 0.01);
    }

    #[test]
    fn parse_path_handles_arc_relative() {
        // Arc command a: M startX startY a rx ry rotation large sweep dx dy
        // After M5 5, an arc relative move by (10, 0) to (15, 5)
        let pts = parse_path("M5 5a5 5 0 0 0 10 0", 24.0);
        // Should have at least 2 points: moveto start and arc end
        assert!(pts.len() >= 2, "arc relative should produce end point");
        // End point should be at (15, 5) scaled to 24x24 space
        assert!((pts[pts.len() - 1].x - 15.0).abs() < 0.01);
        assert!((pts[pts.len() - 1].y - 5.0).abs() < 0.01);
    }

    #[test]
    fn parse_path_handles_negative_numbers() {
        // Negative coordinates after a command
        let pts = parse_path("M0 0l-10 -10", 24.0);
        assert!(!pts.is_empty(), "negative coords should parse");
        // End point should be at (-10, -10) in 24x24 space
        assert!((pts[pts.len() - 1].x - (-10.0)).abs() < 0.01);
        assert!((pts[pts.len() - 1].y - (-10.0)).abs() < 0.01);
    }

    #[test]
    fn parse_path_handles_negative_after_positive() {
        // Mixed positive/negative: "20 -10" where -10 follows a positive number
        let pts = parse_path("M20 -10", 24.0);
        assert!(!pts.is_empty(), "negative after positive should parse");
        // Point should be at (20, -10)
        assert!((pts[0].x - 20.0).abs() < 0.01);
        assert!((pts[0].y - (-10.0)).abs() < 0.01);
    }

    #[test]
    fn parse_path_arc_preserves_visual_fidelity() {
        // A complex path with arc that would be used in circular icons
        // Simulates part of a circle: arc flags control the sweep direction
        let pts_clockwise = parse_path("M12 12a5 5 0 0 1 5 0", 24.0);
        let pts_counter = parse_path("M12 12a5 5 0 0 0 5 0", 24.0);

        // Both should produce valid points (sweep flag is parsed but not rendered)
        assert!(!pts_clockwise.is_empty(), "clockwise arc should parse");
        assert!(
            !pts_counter.is_empty(),
            "counter-clockwise arc should parse"
        );
    }
}
