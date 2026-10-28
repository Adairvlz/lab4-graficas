use raylib::prelude::*;
use std::{env, error::Error, fmt, fs, path::Path};

const SCREEN_WIDTH: i32 = 900;
const SCREEN_HEIGHT: i32 = 700;
const MARGIN: f32 = 70.0;
const DEFAULT_MODEL: &str = "assets/model.obj";

#[derive(Clone, Copy, Debug, PartialEq)]
struct Vertex3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug)]
struct ObjModel {
    vertices: Vec<Vertex3>,
    triangles: Vec<[usize; 3]>,
}

#[derive(Debug)]
struct ObjError {
    line: usize,
    message: String,
}

impl fmt::Display for ObjError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "línea {}: {}", self.line, self.message)
        }
    }
}

impl Error for ObjError {}

impl ObjModel {
    fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let source = fs::read_to_string(path)?;
        Ok(Self::parse(&source)?)
    }

    fn parse(source: &str) -> Result<Self, ObjError> {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();

        for (line_index, raw_line) in source.lines().enumerate() {
            let line_number = line_index + 1;
            let line = raw_line.split('#').next().unwrap_or("").trim();
            let mut parts = line.split_whitespace();

            match parts.next() {
                Some("v") => {
                    let coordinates: Result<Vec<f32>, _> =
                        parts.take(3).map(str::parse::<f32>).collect();
                    let coordinates = coordinates.map_err(|_| ObjError {
                        line: line_number,
                        message: "coordenada de vértice inválida".into(),
                    })?;
                    if coordinates.len() != 3 {
                        return Err(ObjError {
                            line: line_number,
                            message: "un vértice necesita x, y, z".into(),
                        });
                    }
                    vertices.push(Vertex3 {
                        x: coordinates[0],
                        y: coordinates[1],
                        z: coordinates[2],
                    });
                }
                Some("f") => {
                    let face: Result<Vec<usize>, ObjError> = parts
                        .map(|token| parse_vertex_index(token, vertices.len(), line_number))
                        .collect();
                    let face = face?;
                    if face.len() < 3 {
                        return Err(ObjError {
                            line: line_number,
                            message: "una cara necesita al menos tres índices".into(),
                        });
                    }

                    // Convierte también caras de 4+ vértices en triángulos.
                    for i in 1..face.len() - 1 {
                        triangles.push([face[0], face[i], face[i + 1]]);
                    }
                }
                _ => {} // vt, vn, o, g, mtllib y demás campos no son necesarios aún.
            }
        }

        if vertices.is_empty() || triangles.is_empty() {
            return Err(ObjError {
                line: 0,
                message: "el OBJ debe contener vértices y caras".into(),
            });
        }

        Ok(Self {
            vertices,
            triangles,
        })
    }
}

fn parse_vertex_index(token: &str, vertex_count: usize, line: usize) -> Result<usize, ObjError> {
    let value = token
        .split('/')
        .next()
        .unwrap_or("")
        .parse::<isize>()
        .map_err(|_| ObjError {
            line,
            message: format!("índice de cara inválido: {token}"),
        })?;

    // OBJ numera desde 1; los índices negativos cuentan desde el final.
    let index = if value > 0 {
        value - 1
    } else if value < 0 {
        vertex_count as isize + value
    } else {
        -1
    };

    if index < 0 || index >= vertex_count as isize {
        return Err(ObjError {
            line,
            message: format!("índice fuera de rango: {value}"),
        });
    }
    Ok(index as usize)
}

fn project_vertices(vertices: &[Vertex3], width: i32, height: i32, margin: f32) -> Vec<Vector2> {
    let min_x = vertices.iter().map(|v| v.x).fold(f32::INFINITY, f32::min);
    let max_x = vertices
        .iter()
        .map(|v| v.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = vertices.iter().map(|v| v.y).fold(f32::INFINITY, f32::min);
    let max_y = vertices
        .iter()
        .map(|v| v.y)
        .fold(f32::NEG_INFINITY, f32::max);

    let model_width = (max_x - min_x).max(f32::EPSILON);
    let model_height = (max_y - min_y).max(f32::EPSILON);
    let usable_width = (width as f32 - 2.0 * margin).max(1.0);
    let usable_height = (height as f32 - 2.0 * margin).max(1.0);
    let scale = (usable_width / model_width).min(usable_height / model_height);
    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;

    vertices
        .iter()
        .map(|vertex| Vector2 {
            x: (vertex.x - center_x) * scale + width as f32 / 2.0,
            // La pantalla crece hacia abajo, por eso se invierte el eje Y.
            y: height as f32 / 2.0 - (vertex.y - center_y) * scale,
        })
        .collect()
}

fn line<D: RaylibDraw>(draw: &mut D, start: Vector2, end: Vector2, color: Color) {
    let (mut x0, mut y0) = (start.x.round() as i32, start.y.round() as i32);
    let (x1, y1) = (end.x.round() as i32, end.y.round() as i32);
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;

    loop {
        draw.draw_pixel(x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let doubled_error = 2 * error;
        if doubled_error >= dy {
            error += dy;
            x0 += sx;
        }
        if doubled_error <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

fn triangle<D: RaylibDraw>(draw: &mut D, a: Vector2, b: Vector2, c: Vector2, color: Color) {
    line(draw, a, b, color);
    line(draw, b, c, color);
    line(draw, c, a, color);
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let screenshot_mode = arguments.iter().any(|argument| argument == "--screenshot");
    let model_path = arguments
        .iter()
        .find(|argument| argument.as_str() != "--screenshot")
        .map(String::as_str)
        .unwrap_or(DEFAULT_MODEL);

    let model = ObjModel::load(model_path)?;
    let projected = project_vertices(&model.vertices, SCREEN_WIDTH, SCREEN_HEIGHT, MARGIN);

    let (mut rl, thread) = raylib::init()
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("OBJ wireframe — Rust + Raylib")
        .build();
    rl.set_target_fps(60);

    let mut screenshot_saved = false;
    while !rl.window_should_close() {
        {
            let mut draw = rl.begin_drawing(&thread);
            draw.clear_background(Color::new(15, 20, 30, 255));

            for [a, b, c] in &model.triangles {
                triangle(
                    &mut draw,
                    projected[*a],
                    projected[*b],
                    projected[*c],
                    Color::new(82, 210, 255, 255),
                );
            }

            draw.draw_text(
                "OBJ dibujado con triangle + Bresenham",
                20,
                18,
                20,
                Color::RAYWHITE,
            );
            draw.draw_text(
                &format!(
                    "{} vértices | {} triángulos | z se conserva, pero aún no se proyecta",
                    model.vertices.len(),
                    model.triangles.len()
                ),
                20,
                45,
                18,
                Color::GRAY,
            );

            if screenshot_mode && !screenshot_saved {
                // Vacía el lote antes de leer el framebuffer. Esto evita capturas negras
                // en algunos drivers que intercambian el búfer al finalizar el frame.
                unsafe {
                    raylib::ffi::rlDrawRenderBatchActive();
                    raylib::ffi::TakeScreenshot(c"screenshot.png".as_ptr());
                }
                screenshot_saved = true;
            }
        }

        if screenshot_mode && screenshot_saved {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_slashes_negative_indices_and_triangulates_quads() {
        let obj = "\
v -1 -1 0
v  1 -1 0
v  1  1 0
v -1  1 0
f -4/1/1 -3/2/1 -2/3/1 -1/4/1
";
        let model = ObjModel::parse(obj).unwrap();
        assert_eq!(model.vertices.len(), 4);
        assert_eq!(model.triangles, vec![[0, 1, 2], [0, 2, 3]]);
    }

    #[test]
    fn projection_fits_and_centers_the_model() {
        let points = [
            Vertex3 {
                x: -2.0,
                y: -1.0,
                z: 5.0,
            },
            Vertex3 {
                x: 2.0,
                y: 1.0,
                z: -5.0,
            },
        ];
        let result = project_vertices(&points, 100, 100, 10.0);
        assert_eq!(result[0], Vector2::new(10.0, 70.0));
        assert_eq!(result[1], Vector2::new(90.0, 30.0));
    }
}
