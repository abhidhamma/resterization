// h:\coding\rustWorkspace\graphics\resterization\src\app.rs

use eframe::egui;
use egui::{Color32, ColorImage, Pos2, Resize, ScrollArea, TextureHandle, Ui, vec2};
use glam::{Mat3, Vec3, vec2 as glam_vec2, vec3};

/**
 * C++의 Mesh 클래스에 해당
 * 원과 같은 기하학적 도형의 원본 데이터를 보관
 */
struct Mesh {
    vertices: Vec<Vec3>,
    colors: Vec<Vec3>,
    indices: Vec<usize>,
}

impl Mesh {
    /**
     * 원의 기하학적 정보를 생성
     * C++의 Mesh::InitCircle 함수와 동일한 로직
     */
    fn new_circle(center: Vec3, radius: f32, num_triangles: usize) -> Self {
        let mut vertices = Vec::new();
        let mut colors = Vec::new();
        let mut indices = Vec::new();

        /* 필요한 메모리 미리 할당 */
        vertices.reserve(num_triangles + 1);
        colors.reserve(num_triangles + 1);
        indices.reserve(num_triangles * 3);

        /* 중심점 추가 */
        vertices.push(center);
        colors.push(vec3(1.0, 0.0, 0.0)); // Red

        let two_pi = 2.0 * std::f32::consts::PI;
        let delta_theta = two_pi / num_triangles as f32;

        /* 가장자리 정점 추가 */
        for i in 0..num_triangles {
            let theta = i as f32 * delta_theta;
            vertices.push(center + vec3(theta.cos() * radius, theta.sin() * radius, 0.0));
            colors.push(vec3(0.0, 0.0, 1.0)); // Blue
        }

        /* 인덱스 정의 (반시계 방향, CCW) - C++ 코드와 동일하게 수정 */
        for i in 0..num_triangles {
            indices.push(0); // 중심점
            indices.push(if i == num_triangles - 1 {
                1 // 마지막 삼각형은 첫 번째 가장자리 정점과 연결
            } else {
                i + 2
            });
            indices.push(i + 1);
        }

        Self {
            vertices,
            colors,
            indices,
        }
    }
}

/**
 * C++의 Rasterization 클래스에 해당
 * 변환, 래스터화 등 핵심 로직 담당
 */
struct Rasterization {
    width: usize,
    height: usize,
    circle: Mesh, // 원본 도형 데이터

    /* 변환(Transform)을 위한 데이터 */
    translation1: Vec3,
    translation2: Vec3,
    rotation1: f32,
    rotation2: f32,
    scale_x: f32,
    scale_y: f32,

    /* 변환이 적용된 후의 정점, 색상, 인덱스 버퍼 */
    vertex_buffer: Vec<Vec3>,
    color_buffer: Vec<Vec3>,
    index_buffer: Vec<usize>,
}

impl Rasterization {
    fn new(width: usize, height: usize) -> Self {
        let circle = Mesh::new_circle(vec3(0.0, 0.0, 1.0), 0.3, 5); // 오각형으로 수정

        /*
         * C++ 코드처럼 원본 데이터를 버퍼에 복사
         * Update()에서 원본(circle)을 사용해 계산한 결과를 이 버퍼들에 저장
         */
        let vertex_buffer = circle.vertices.clone();
        let color_buffer = circle.colors.clone();
        let index_buffer = circle.indices.clone();

        Self {
            width,
            height,
            circle,
            translation1: Vec3::ZERO,
            translation2: Vec3::ZERO,
            rotation1: 0.0,
            rotation2: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            vertex_buffer,
            color_buffer,
            index_buffer,
        }
    }

    // 2차원 변환: GPU의 버텍스 셰이더가 하는 일과 매우 유사함
    fn update(&mut self) {
        for i in 0..self.circle.vertices.len() {
            // 1. 오브젝트 중심 회전
            let mut temp = rotate_about_z(self.circle.vertices[i], self.rotation1);
            // 2. 크기 조절
            temp *= vec3(self.scale_x, self.scale_y, 1.0);
            // 3. 오브젝트 이동
            temp += self.translation1;
            // 4. 원점 중심 회전
            temp = rotate_about_z(temp, self.rotation2);
            // 5. 축 전체 이동
            temp += self.translation2;

            self.vertex_buffer[i] = temp;
        }
    }

    /**
     * C++의 Rasterization::Render() 함수에 해당
     * 인덱스 버퍼를 순회하며 모든 삼각형을 그림
     * 이 부분이 이번 학습의 핵심입니다.
     */
    fn render(&self, pixels: &mut [Color32]) {
        /*
         * 인덱스 버퍼(index_buffer)는 삼각형을 구성하는 정점들의 '주소' 목록
         * 3개씩 묶어서 하나의 삼각형을 정의 (예: [0, 1, 2, 0, 2, 3, ...])
         * step_by(3)을 사용해 3칸씩 건너뛰며 각 삼각형의 시작 인덱스(i)를 가져옴
         */
        for i in (0..self.index_buffer.len()).step_by(3) {
            self.draw_indexed_triangle(i, pixels);
        }
    }

    /**
     * 3차원 월드 좌표를 2차원 래스터(화면) 좌표로 변환
     * C++ 버전의 `ProjectWorldToRaster`와 동일한 로직
     */
    fn project_world_to_raster(&self, point: Vec3) -> Pos2 {
        let aspect = self.width as f32 / self.height as f32;
        let point_ndc = glam_vec2(point.x / aspect, point.y);

        let x_scale = 2.0 / self.width as f32;
        let y_scale = 2.0 / self.height as f32;

        Pos2::new(
            (point_ndc.x + 1.0) / x_scale - 0.5,
            (1.0 - point_ndc.y) / y_scale - 0.5,
        )
    }

    /**
     * Edge Function: 한 점이 특정 변(edge)에 대해 어느 쪽에 있는지 판별
     * C++ 버전의 `EdgeFunction`과 동일
     */
    fn edge_function(v0: Pos2, v1: Pos2, point: Pos2) -> f32 {
        let a = v1 - v0;
        let b = point - v0;
        a.x * b.y - a.y * b.x
    }

    /**
     * 인덱스를 사용하여 삼각형 하나를 그림
     * C++ 버전의 `DrawIndexedTriangle`과 동일한 로직
     */
    fn draw_indexed_triangle(&self, start_index: usize, pixels: &mut [Color32]) {
        /* 1. 인덱스 버퍼에서 현재 삼각형을 구성하는 정점 인덱스 3개를 가져옴 */
        let i0 = self.index_buffer[start_index];
        let i1 = self.index_buffer[start_index + 1];
        let i2 = self.index_buffer[start_index + 2];

        /* 2. 정점 버퍼와 색상 버퍼에서 해당 인덱스의 실제 데이터(위치, 색상)를 가져옴 */
        let v0_raster = self.project_world_to_raster(self.vertex_buffer[i0]);
        let v1_raster = self.project_world_to_raster(self.vertex_buffer[i1]);
        let v2_raster = self.project_world_to_raster(self.vertex_buffer[i2]);

        let c0 = self.color_buffer[i0];
        let c1 = self.color_buffer[i1];
        let c2 = self.color_buffer[i2];

        /* 3. 삼각형을 감싸는 최소 사각형(Bounding Box) 계산 */
        let x_min = (v0_raster.x.min(v1_raster.x.min(v2_raster.x)))
            .floor()
            .max(0.0) as usize;
        let x_max = (v0_raster.x.max(v1_raster.x.max(v2_raster.x)))
            .ceil()
            .min(self.width as f32 - 1.0) as usize;
        let y_min = (v0_raster.y.min(v1_raster.y.min(v2_raster.y)))
            .floor()
            .max(0.0) as usize;
        let y_max = (v0_raster.y.max(v1_raster.y.max(v2_raster.y)))
            .ceil()
            .min(self.height as f32 - 1.0) as usize;

        /* 4. 바운딩 박스 내 모든 픽셀을 순회하며 삼각형 내부에 있는지 검사 */
        for j in y_min..=y_max {
            for i in x_min..=x_max {
                let point = Pos2::new(i as f32, j as f32);

                /* Edge Function을 이용해 픽셀이 삼각형 내부에 있는지 판별 */
                let alpha0 = Self::edge_function(v1_raster, v2_raster, point);
                let alpha1 = Self::edge_function(v2_raster, v0_raster, point);
                let alpha2 = Self::edge_function(v0_raster, v1_raster, point);

                if alpha0 >= 0.0 && alpha1 >= 0.0 && alpha2 >= 0.0 {
                    let area = alpha0 + alpha1 + alpha2;
                    if area == 0.0 {
                        continue;
                    }

                    /* 5. 무게중심 좌표를 이용해 색상 보간 및 픽셀 채색 */
                    let color_vec = (alpha0 * c0 + alpha1 * c1 + alpha2 * c2) / area;

                    let r = (color_vec.x * 255.0) as u8;
                    let g = (color_vec.y * 255.0) as u8;
                    let b = (color_vec.z * 255.0) as u8;

                    let index = i + j * self.width;
                    if index < pixels.len() {
                        pixels[index] = Color32::from_rgb(r, g, b);
                    }
                }
            }
        }
    }
}

/*
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
 * C++의 Example 클래스와 main() 함수의 역할을 합친 구조체
 * eframe::App 트레이트를 구현하여 GUI 애플리케이션의 상태를 관리하고 렌더링 로직을 실행
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
     * 요청하신 순서대로 UI 컨트롤을 표시하는 함수
     */
    fn show_controls(&mut self, ui: &mut Ui) {
        ui.label("object translation");
        ui.add(
            egui::Slider::new(&mut self.rasterization.translation1.x, -1.0..=1.0)
                .text("Translation X"),
        );
        ui.add(
            egui::Slider::new(&mut self.rasterization.translation1.y, -1.0..=1.0)
                .text("Translation Y"),
        );
        ui.add(egui::Slider::new(&mut self.rasterization.scale_x, -2.0..=2.0).text("Scale X"));
        ui.add(egui::Slider::new(&mut self.rasterization.scale_y, -2.0..=2.0).text("Scale Y"));
        ui.add(
            egui::Slider::new(&mut self.rasterization.rotation1, -3.141592..=3.141592)
                .text("Rotation (Object)"),
        );

        ui.separator();

        ui.label("axis translation");
        ui.add(
            egui::Slider::new(&mut self.rasterization.rotation2, -3.141592..=3.141592)
                .text("Rotation (Axis)"),
        );
        ui.add(
            egui::Slider::new(&mut self.rasterization.translation2.x, -1.0..=1.0)
                .text("Axis Translate X"),
        );
        ui.add(
            egui::Slider::new(&mut self.rasterization.translation2.y, -1.0..=1.0)
                .text("Axis Translate Y"),
        );
    }
}

impl eframe::App for RasterizationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        /* 1. 변환 값에 따라 정점 위치 업데이트 */
        self.rasterization.update();

        /* 2. 픽셀 버퍼 생성 및 래스터화 수행 */
        let mut image = ColorImage::filled(
            [self.rasterization.width, self.rasterization.height],
            Color32::BLACK,
        );
        self.rasterization.render(&mut image.pixels);

        /* 3. 픽셀 버퍼를 텍스처로 변환하여 화면에 표시 준비 */
        self.texture.set(image, Default::default());

        /* 4. ImGui에 해당하는 egui UI 컨트롤 생성 */
        egui::Window::new("Scene Control").show(ctx, |ui| {
            self.show_controls(ui);
        });

        /* 5. 텍스처를 화면 중앙에 그림 (반응형 UI 적용) */
        egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::both().show(ui, |ui| {
                Resize::default()
                    .default_size(vec2(
                        self.rasterization.width as f32,
                        self.rasterization.height as f32,
                    ))
                    .min_size(vec2(320.0, 240.0))
                    .show(ui, |ui| {
                        ui.image(&self.texture);
                    });
            });
        });

        /* 6. 다음 프레임을 위해 다시 그리도록 요청 (애니메이션) */
        ctx.request_repaint();
    }
}
