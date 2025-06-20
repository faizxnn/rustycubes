use std::{io::{self, Read}, thread, time::Duration, f32::consts::PI, sync::{Arc, Mutex}};
use std::sync::mpsc;

const WIDTH: usize = 80;
const HEIGHT: usize = 24;
const CUBE_SIZE: f32 = 1.0;
const DISTANCE: f32 = 3.0;
const SCALE: f32 = 20.0;

#[derive(Clone, Copy, Debug)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

// Rotation around X, Y, Z axes
fn rotate(v: Vec3, ax: f32, ay: f32, az: f32) -> Vec3 {
    // Rotate around X
    let sinx = ax.sin();
    let cosx = ax.cos();
    let mut v = Vec3 {
        x: v.x,
        y: v.y * cosx - v.z * sinx,
        z: v.y * sinx + v.z * cosx,
    };
    // Rotate around Y
    let siny = ay.sin();
    let cosy = ay.cos();
    v = Vec3 {
        x: v.x * cosy + v.z * siny,
        y: v.y,
        z: -v.x * siny + v.z * cosy,
    };
    // Rotate around Z
    let sinz = az.sin();
    let cosz = az.cos();
    Vec3 {
        x: v.x * cosz - v.y * sinz,
        y: v.x * sinz + v.y * cosz,
        z: v.z,
    }
}

// Simple perspective projection
fn project(v: Vec3) -> (usize, usize) {
    let factor = SCALE / (v.z + DISTANCE);
    let x = (v.x * factor + (WIDTH as f32) / 2.0) as isize;
    let y = (v.y * factor + (HEIGHT as f32) / 2.0) as isize;
    (
        x.clamp(0, (WIDTH - 1) as isize) as usize,
        y.clamp(0, (HEIGHT - 1) as isize) as usize,
    )
}

// Bresenham's line algorithm
fn draw_line((x0, y0): (usize, usize), (x1, y1): (usize, usize), screen: &mut [Vec<char>]) {
    let (mut x0, mut y0, x1, y1) = (x0 as isize, y0 as isize, x1 as isize, y1 as isize);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x0 >= 0 && x0 < WIDTH as isize && y0 >= 0 && y0 < HEIGHT as isize {
            screen[y0 as usize][x0 as usize] = '#';
        }
        if x0 == x1 && y0 == y1 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; x0 += sx; }
        if e2 <= dx { err += dx; y0 += sy; }
    }
}

// Non-blocking input reader for arrow keys
fn spawn_input_thread() -> mpsc::Receiver<(f32, f32)> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let stdin = io::stdin();
        let mut stdin = stdin.lock();
        let mut buf = [0u8; 3];
        loop {
            if let Ok(n) = stdin.read(&mut buf) {
                if n == 3 && buf[0] == 27 && buf[1] == 91 {
                    // Arrow keys
                    match buf[2] {
                        65 => { tx.send((0.1, 0.0)).ok(); } // Up
                        66 => { tx.send((-0.1, 0.0)).ok(); } // Down
                        67 => { tx.send((0.0, 0.1)).ok(); } // Right
                        68 => { tx.send((0.0, -0.1)).ok(); } // Left
                        _ => {}
                    }
                }
            }
        }
    });
    rx
}

fn main() {
    // Set terminal to raw mode (Unix only)
    #[cfg(unix)]
    let _ = std::process::Command::new("stty").arg("raw").arg("-echo").status();

    // Vertices of a cube
    let cube = [
        Vec3 { x: -CUBE_SIZE, y: -CUBE_SIZE, z: -CUBE_SIZE },
        Vec3 { x:  CUBE_SIZE, y: -CUBE_SIZE, z: -CUBE_SIZE },
        Vec3 { x:  CUBE_SIZE, y:  CUBE_SIZE, z: -CUBE_SIZE },
        Vec3 { x: -CUBE_SIZE, y:  CUBE_SIZE, z: -CUBE_SIZE },
        Vec3 { x: -CUBE_SIZE, y: -CUBE_SIZE, z:  CUBE_SIZE },
        Vec3 { x:  CUBE_SIZE, y: -CUBE_SIZE, z:  CUBE_SIZE },
        Vec3 { x:  CUBE_SIZE, y:  CUBE_SIZE, z:  CUBE_SIZE },
        Vec3 { x: -CUBE_SIZE, y:  CUBE_SIZE, z:  CUBE_SIZE },
    ];
    // Edges between vertices
    let edges = [
        (0,1),(1,2),(2,3),(3,0), // back face
        (4,5),(5,6),(6,7),(7,4), // front face
        (0,4),(1,5),(2,6),(3,7), // connections
    ];

    // Shared rotation angles
    let angle_x = Arc::new(Mutex::new(0.0f32));
    let angle_y = Arc::new(Mutex::new(0.0f32));
    let rx = angle_x.clone();
    let ry = angle_y.clone();

    // Input thread
    let input_rx = spawn_input_thread();

    // Animation loop
    loop {
        // Handle input
        while let Ok((dx, dy)) = input_rx.try_recv() {
            *rx.lock().unwrap() += dx;
            *ry.lock().unwrap() += dy;
        }

        // Clear screen
        print!("\x1B[2J\x1B[1;1H");

        let ax = *rx.lock().unwrap();
        let ay = *ry.lock().unwrap();
        let az = ax * 0.5 + ay * 0.5; // for a bit of extra spin

        // Rotate and project
        let projected: Vec<(usize, usize)> = cube.iter()
            .map(|&v| {
                let rotated = rotate(v, ax, ay, az);
                project(rotated)
            })
            .collect();

        // Draw edges
        let mut screen = vec![vec![' '; WIDTH]; HEIGHT];
        for &(a, b) in &edges {
            draw_line(projected[a], projected[b], &mut screen);
        }

        // Print screen
        for row in screen {
            println!("{}", row.iter().collect::<String>());
        }
        println!("Use arrow keys to rotate. Ctrl+C to exit.");

        thread::sleep(Duration::from_millis(30));
    }

    // Restore terminal mode (Unix only)
    #[cfg(unix)]
    let _ = std::process::Command::new("stty").arg("-raw").arg("echo").status();
} 