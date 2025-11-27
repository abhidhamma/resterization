// h:\coding\rustWorkspace\graphics\resterization\src\app.rs

use eframe::egui;
use egui::{Color32, ColorImage, Pos2, TextureHandle, Ui};
use glam::{Vec3, vec3};

/**
 * 각 천체(태양, 지구, 달)의 원본 데이터를 보관
 */
struct Mesh {
    vertices: Vec<Vec3>,
    colors: Vec<Vec3>,
    indices: Vec<usize>,
}

impl Mesh {
    /**
     * 원의 기하학적 정보 생성 및 색상 지정
     */
    fn new_circle(radius: f32, num_triangles: usize, color: Vec3) -> Self {
        let mut vertices = Vec::with_capacity(num_triangles + 1);
        let mut colors = Vec::with_capacity(num_triangles + 1);
        let mut indices = Vec::with_capacity(num_triangles * 3);

        // 중심점은 원점(0,0,0)으로 고정 (인덱스 0)
        vertices.push(Vec3::ZERO);
        colors.push(color);

        let two_pi = 2.0 * std::f32::consts::PI;
        let delta_theta = two_pi / num_triangles as f32;

        // 가장자리 정점 추가 (인덱스 1 ~ num_triangles)
        for i in 0..num_triangles {
            let theta = i as f32 * delta_theta;
            vertices.push(vec3(theta.cos() * radius, theta.sin() * radius, 0.0));
            colors.push(color);
        }

        // 인덱스 정의 (반시계 방향, CCW)
        for i in 0..num_triangles {
            indices.push(0); // 중심점

            if i == num_triangles - 1 {
                indices.push(1); // 마지막 삼각형은 첫 번째 가장자리 정점과 연결
            } else {
                indices.push(i + 2); // 다음 가장자리 정점
            }

            indices.push(i + 1); // 현재 가장자리 정점
        }

        Self {
            vertices,
            colors,
            indices,
        }
    }
}

/**
 * 태양계 애니메이션과 래스터화 로직 담당
 */
struct Rasterization {
    width: usize,
    height: usize,

    // 각 천체의 원본 데이터
    sun: Mesh,
    earth: Mesh,
    moon: Mesh,

    // 애니메이션 상태 변수
    earth_angle: f32,
    earth_angular_velocity: f32,
    moon_angle: f32,
    moon_angular_velocity: f32,

    // 천체 간 거리
    dist_sun_to_earth: f32,
    dist_earth_to_moon: f32,

    /**
     * 변환된 정점을 담기 위한 임시 버퍼.
     * 매 프레임 재사용하여 불필요한 메모리 할당을 줄임.
     */
    transformed_vertices: Vec<Vec3>,
}

impl Rasterization {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            sun: Mesh::new_circle(0.1, 20, vec3(1.0, 1.0, 0.0)), // Yellow
            earth: Mesh::new_circle(0.05, 20, vec3(0.0, 0.0, 1.0)), // Blue
            moon: Mesh::new_circle(0.02, 20, vec3(1.0, 1.0, 1.0)), // White
            earth_angle: 0.0,
            earth_angular_velocity: 0.3,
            moon_angle: 0.0,
            moon_angular_velocity: 1.0,
            dist_sun_to_earth: 0.5,
            dist_earth_to_moon: 0.2,
            transformed_vertices: Vec::new(),
        }
    }

    /**
     * 애니메이션 상태 업데이트
     */
    fn update_animation(&mut self, dt: f32) {
        self.earth_angle += self.earth_angular_velocity * dt;
        self.moon_angle += self.moon_angular_velocity * dt;
    }

    /*
     * 각 천체를 순서대로 변환하고 그림
     */
    fn render(&mut self, pixels: &mut [Color32]) {
        // 1. 태양 그리기: 월드좌표계 원점
        self.draw_mesh(&self.sun.vertices, &self.sun, pixels);

        // 2. 지구 그리기: x축이동 + 원점(태양)중심 회전
        self.transformed_vertices.clear();
        for v in &self.earth.vertices {
            let earth_pos = *v + vec3(self.dist_sun_to_earth, 0.0, 0.0);
            self.transformed_vertices
                .push(rotate_about_z(earth_pos, self.earth_angle));
        }
        // 변환된 정점 데이터와 원본 지구 메쉬(&self.earth)를 함께 넘기기
        self.draw_mesh(&self.transformed_vertices, &self.earth, pixels);

        // 3. 달 그리기: 지구그리기(x축이동 + 원점중심 회전) + 객체(지구) 중심 회전
        self.transformed_vertices.clear();
        for v in &self.moon.vertices {
            let moon_relative_to_earth = *v + vec3(self.dist_earth_to_moon, 0.0, 0.0);
            let moon_rotated_around_earth = rotate_about_z(moon_relative_to_earth, self.moon_angle);
            let moon_pos_in_solar_system =
                moon_rotated_around_earth + vec3(self.dist_sun_to_earth, 0.0, 0.0);
            self.transformed_vertices
                .push(rotate_about_z(moon_pos_in_solar_system, self.earth_angle));
        }
        // 변환된 정점 데이터와 원본 달 메쉬(&self.moon)를 함께 넘겨 그림
        self.draw_mesh(&self.transformed_vertices, &self.moon, pixels);
    }

    /**
     * 중복 로직을 처리하기 위해 추가된 헬퍼(Helper) 함수.
     * 주어진 정점들과 메쉬 정보를 이용해 삼각형들을 그림.
     */
    fn draw_mesh(&self, vertices: &[Vec3], mesh: &Mesh, pixels: &mut [Color32]) {
        // 메쉬가 가진 인덱스 버퍼의 길이만큼 3칸씩(삼각형 하나) 건너뛰며 반복
        for i in (0..mesh.indices.len()).step_by(3) {
            // `draw_indexed_triangle` 함수에 필요한 모든 정보를 '참조'로 넘김
            self.draw_indexed_triangle(vertices, &mesh.indices, &mesh.colors, i, pixels);
        }
    }

    /**
     * 인덱스를 사용하여 삼각형 하나를 그림
     */
    fn draw_indexed_triangle(
        &self,
        vertices: &[Vec3],      // 그릴 삼각형의 정점 데이터 (변환 완료)
        indices: &[usize],      // 정점을 연결하는 인덱스 데이터
        colors: &[Vec3],        // 각 정점의 색상 데이터
        start_index: usize,     // 인덱스 버퍼에서 현재 삼각형이 시작되는 위치
        pixels: &mut [Color32], // 최종 픽셀 색상을 기록할 버퍼
    ) {
        // 1. 현재 삼각형을 구성하는 세 정점의 인덱스를 가져옴
        let i0 = indices[start_index];
        let i1 = indices[start_index + 1];
        let i2 = indices[start_index + 2];

        // 2. 인덱스를 사용해 3D 월드 좌표계의 정점 위치를 가져온 후, 2D 화면 좌표로 변환
        let v0 = self.project_world_to_raster(vertices[i0]);
        let v1 = self.project_world_to_raster(vertices[i1]);
        let v2 = self.project_world_to_raster(vertices[i2]);

        // 3. 각 정점에 해당하는 색상 정보를 가져옴
        let c0 = colors[i0];
        let c1 = colors[i1];
        let c2 = colors[i2];

        // 4. 삼각형을 감싸는 최소한의 사각형(Bounding Box)을 계산
        let x_min = (v0.x.min(v1.x.min(v2.x)))
            .floor()
            .max(0.0)
            .min((self.width - 1) as f32) as usize;
        let y_min = (v0.y.min(v1.y.min(v2.y)))
            .floor()
            .max(0.0)
            .min((self.height - 1) as f32) as usize;
        let x_max = (v0.x.max(v1.x.max(v2.x)))
            .ceil()
            .max(0.0)
            .min((self.width - 1) as f32) as usize;
        let y_max = (v0.y.max(v1.y.max(v2.y)))
            .ceil()
            .max(0.0)
            .min((self.height - 1) as f32) as usize;

        // 5. Bounding Box 내의 모든 픽셀을 순회
        for j in y_min..=y_max {
            for i in x_min..=x_max {
                // 현재 검사할 픽셀의 중심 좌표
                let point = Pos2::new(i as f32 + 0.5, j as f32 + 0.5);

                // 6. Edge Function을 사용해 현재 픽셀이 삼각형의 각 변(edge)의 '안쪽'에 있는지 검사
                let alpha0 = Self::edge_function(v1, v2, point);
                let alpha1 = Self::edge_function(v2, v0, point);
                let alpha2 = Self::edge_function(v0, v1, point);

                // 7. 세 개의 Edge Function 결과가 모두 0 이상이면, 픽셀은 삼각형 내부에 존재
                if alpha0 >= 0.0 && alpha1 >= 0.0 && alpha2 >= 0.0 {
                    // 8. 무게 중심 좌표(Barycentric Coordinates)를 계산
                    let area = alpha0 + alpha1 + alpha2;
                    if area == 0.0 {
                        continue; // 면적이 0이면 계산을 건너뜀
                    }

                    // 각 정점의 가중치(alpha, beta, gamma)를 계산
                    let alpha = alpha0 / area;
                    let beta = alpha1 / area;
                    let gamma = alpha2 / area;

                    // 9. 계산된 가중치를 이용해 세 정점의 색상을 보간하여 현재 픽셀의 최종 색상을 결정
                    let color = alpha * c0 + beta * c1 + gamma * c2;

                    // 10. 계산된 색상(f32)을 Color32(u8)로 변환하여 픽셀 버퍼에 기록
                    let r = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
                    let g = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
                    let b = (color.z.clamp(0.0, 1.0) * 255.0) as u8;

                    // 픽셀 버퍼의 1차원 인덱스 계산 및 색상 쓰기
                    let index = i + j * self.width;
                    if index < pixels.len() {
                        pixels[index] = Color32::from_rgb(r, g, b);
                    }
                }
            }
        }
    }

    /**
     * 3차원 월드 좌표를 2차원 래스터(화면) 좌표로 변환
     */
    fn project_world_to_raster(&self, point: Vec3) -> Pos2 {
        let aspect = self.width as f32 / self.height as f32;
        let point_ndc = glam::vec2(point.x / aspect, point.y);

        let x_scale = 2.0 / self.width as f32;
        let y_scale = 2.0 / self.height as f32;

        Pos2::new(
            (point_ndc.x + 1.0) / x_scale - 0.5,
            (1.0 - point_ndc.y) / y_scale - 0.5,
        )
    }

    /**
     * Edge Function
     */
    fn edge_function(v0: Pos2, v1: Pos2, p: Pos2) -> f32 {
        (v1.x - v0.x) * (p.y - v0.y) - (v1.y - v0.y) * (p.x - v0.x)
    }
}

/**
 * 원점(z축)을 기준으로 2차원 회전
 */
fn rotate_about_z(v: Vec3, theta: f32) -> Vec3 {
    vec3(
        v.x * theta.cos() - v.y * theta.sin(),
        v.x * theta.sin() + v.y * theta.cos(),
        v.z,
    )
}

/**
 * eframe::App 구현체
 */
pub struct RasterizationApp {
    rasterization: Rasterization,
    texture: TextureHandle,
}

impl RasterizationApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let width = 1280;
        let height = 960;

        let texture = cc.egui_ctx.load_texture(
            "rasterization_texture",
            ColorImage::filled([width, height], Color32::BLACK),
            Default::default(),
        );

        Self {
            rasterization: Rasterization::new(width, height),
            texture,
        }
    }

    /**
     * UI 컨트롤
     */
    fn show_controls(&mut self, ui: &mut Ui) {
        ui.add(
            egui::Slider::new(&mut self.rasterization.earth_angular_velocity, 0.0..=2.0)
                .text("Earth Angular Velocity"),
        );
        ui.add(
            egui::Slider::new(&mut self.rasterization.moon_angular_velocity, 0.0..=5.0)
                .text("Moon Angular Velocity"),
        );
    }
}

impl eframe::App for RasterizationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Window::new("Scene Control").show(ctx, |ui| {
            self.show_controls(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let available_size = ui.available_size();
            let (width, height) = (available_size.x as usize, available_size.y as usize);

            if width == 0 || height == 0 {
                return;
            }

            // 래스터라이저의 크기를 현재 UI 크기에 맞게 업데이트
            self.rasterization.width = width;
            self.rasterization.height = height;

            // 애니메이션 상태 업데이트
            self.rasterization
                .update_animation(ctx.input(|i| i.stable_dt));

            let mut image = ColorImage::filled([width, height], Color32::BLACK);
            self.rasterization.render(&mut image.pixels);

            self.texture.set(image, Default::default());

            ui.image((self.texture.id(), available_size));
        });

        ctx.request_repaint();
    }
}
