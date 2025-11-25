// h:\coding\rustWorkspace\graphics\resterization\src\app.rs

use eframe::egui;
use egui::{Color32, ColorImage, Pos2, TextureHandle};

/**
 * C++의 glm 라이브러리 대신 glam 크레이트를 사용합니다.
 * Cargo.toml 파일의 [dependencies] 섹션에 glam = "0.27.0" 을 추가해야 합니다.
 */
use glam::{Vec2, Vec3, vec2, vec3};

/**
 * C++ 코드의 Rasterization 클래스에 해당합니다.
 * 렌더링에 필요한 데이터와 로직을 가집니다.
 * 이번 예제에서는 원을 그리기 위한 정점, 색상, 인덱스 데이터를 가집니다.
 */
struct Rasterization {
    width: usize,
    height: usize,
    num_triangles: usize, // 원을 구성하는 삼각형의 개수
    vertices: Vec<Vec3>,
    colors: Vec<Vec3>,
    indices: Vec<usize>,
}

impl Rasterization {
    /**
     * Rasterization 구조체의 생성자(constructor) 역할을 하는 함수입니다.
     * C++ 생성자 코드의 로직을 그대로 따릅니다.
     */
    fn new(width: usize, height: usize, num_triangles: usize) -> Self {
        let mut rasterizer = Self {
            width,
            height,
            num_triangles,
            vertices: Vec::new(),
            colors: Vec::new(),
            indices: Vec::new(),
        };
        rasterizer.setup_circle_geometry();
        rasterizer
    }

    /*
     원의 속성(정점, 색상, 인덱스)을 설정
     num_triangles가 변경될 때마다 이 함수를 호출하여 원을 다시 계산
    */
    fn setup_circle_geometry(&mut self) {
        // 1. 기존 데이터 초기화
        self.vertices.clear();
        self.colors.clear();
        self.indices.clear();

        // 2. 원의 반지름과 중심위치 정의
        let radius = 0.5;
        let center = vec3(0.0, 0.0, 1.0);

        // 3. 정점, 색상, 인덱스 Vec에 필요한 메모리를 미리 할당(최적화)
        // 정점: 가장자리점은 삼각형 개수 + 중심점 1개
        self.vertices.reserve(self.num_triangles + 1);
        // 색상: 모든 정점은 색상을 가짐
        self.colors.reserve(self.num_triangles + 1);
        // 인덱스: 삼각형 1개당 3개의 인덱스가 필요
        self.indices.reserve(self.num_triangles * 3);

        // 4. 중심이 될 0번인덱스의 정점을 추가하고 색상을 빨간색으로 설정
        self.vertices.push(center);
        self.colors.push(vec3(1.0, 0.0, 0.0));

        // 5. 원을 몇 개의 조각으로 나눌지 각도를 계산
        let two_pi = 2.0 * std::f32::consts::PI;
        let delta_theta = two_pi / self.num_triangles as f32;

        /*
            6. for 루프를 돌며 원의 가장자리를 구성하는 정점들을 추가
            원 위의 점 중에서
            x축에서 theta만큼 회전한 위치의 좌표는 (cos(theta), sin(theta))로 계산할 수 있음
            cos(theta): 회전한 점의 x좌표
            sin(theta): 회전한 점의 y좌표
            이 좌표에 반지름(radius)을 곱해서 원의 실제 정점 위치 구하기
        */
        for i in 0..self.num_triangles {
            // 현재 정점이 위치할 각도 theta를 계산
            let theta = i as f32 * delta_theta;
            // (cos, sin)으로 방향을 구하고 radius를 곱해 거리 조절 후 center를 더해 최종 위치 계산
            self.vertices
                .push(center + vec3(theta.cos() * radius, theta.sin() * radius, 0.0));
            // 가장자리 정점들의 색상은 파란색으로 설정
            self.colors.push(vec3(0.0, 0.0, 1.0)); // Blue
        }

        // 7. 위에서 만든 정점들을 이용해 삼각형을 구성하도록 인덱스를 반시계방향으로 정의
        for i in 0..self.num_triangles {
            // 모든 삼각형은 0번 정점(중심점)을 포함하고 있음
            self.indices.push(0);
            // 삼각형의 두 번째 꼭지점 마지막 삼각형은 처음(1번) 가장자리 점과 연결
            self.indices.push(if i == self.num_triangles - 1 {
                1 // 마지막 삼각형은 첫 번째 가장자리 정점과 연결
            } else {
                i + 2
            });
            // 삼각형의 세 번째 꼭지점
            self.indices.push(i + 1);
        }
    }

    /**
     * 3차원 월드 좌표를 2차원 래스터(화면) 좌표로 변환합니다.
     * C++ 버전의 `ProjectWorldToRaster`와 동일한 로직입니다.
     */
    fn project_world_to_raster(&self, point: Vec3) -> Pos2 {
        let aspect = self.width as f32 / self.height as f32;
        let point_ndc = vec2(point.x / aspect, point.y);

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
     * 인덱스를 사용하여 삼각형 하나를 그립니다.
     * C++ 버전의 `DrawIndexedTriangle`과 동일한 로직입니다.
     */
    fn draw_indexed_triangle(&self, start_index: usize, pixels: &mut [Color32]) {
        let i0 = self.indices[start_index];
        let i1 = self.indices[start_index + 1];
        let i2 = self.indices[start_index + 2];

        let v0_raster = self.project_world_to_raster(self.vertices[i0]);
        let v1_raster = self.project_world_to_raster(self.vertices[i1]);
        let v2_raster = self.project_world_to_raster(self.vertices[i2]);

        let c0 = self.colors[i0];
        let c1 = self.colors[i1];
        let c2 = self.colors[i2];

        /* 삼각형을 감싸는 최소 사각형(Bounding Box) 계산 */
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

        /* 바운딩 박스 내 모든 픽셀 순회 */
        for j in y_min..=y_max {
            for i in x_min..=x_max {
                let point = Pos2::new(i as f32, j as f32);

                let alpha0 = Self::edge_function(v1_raster, v2_raster, point);
                let alpha1 = Self::edge_function(v2_raster, v0_raster, point);
                let alpha2 = Self::edge_function(v0_raster, v1_raster, point);

                if alpha0 >= 0.0 && alpha1 >= 0.0 && alpha2 >= 0.0 {
                    let area = alpha0 + alpha1 + alpha2;
                    if area == 0.0 {
                        continue;
                    }

                    /* 무게중심 좌표를 이용해 색상 보간 */
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

    /**
     * 래스터라이제이션을 수행하여 픽셀 버퍼를 채웁니다.
     * C++ 버전의 `Render` 함수와 동일한 역할을 합니다.
     */
    fn render(&self, pixels: &mut [Color32]) {
        /* 인덱스 버퍼를 순회하며 모든 삼각형을 그립니다. */
        for i in (0..self.indices.len()).step_by(3) {
            self.draw_indexed_triangle(i, pixels);
        }
    }

    fn update(&mut self) {
        /* 애니메이션 구현 공간 */
    }
}

/**
 * C++의 Example 클래스와 main() 함수의 역할을 합친 구조체입니다.
 * eframe::App 트레이트를 구현하여 GUI 애플리케이션의 상태를 관리하고 렌더링 로직을 실행합니다.
 */
pub struct RasterizationApp {
    rasterization: Rasterization,
    texture: TextureHandle,
}

impl RasterizationApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let width = 1280;
        let height = 960;
        let initial_triangles = 3; // 시작 삼각형 개수

        let texture = cc.egui_ctx.load_texture(
            "rasterization_texture",
            ColorImage::filled([width, height], Color32::BLACK),
            Default::default(),
        );

        Self {
            rasterization: Rasterization::new(width, height, initial_triangles),
            texture,
        }
    }
}

impl eframe::App for RasterizationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.rasterization.update();

        let mut image = ColorImage::filled(
            [self.rasterization.width, self.rasterization.height],
            Color32::BLACK,
        );

        self.rasterization.render(&mut image.pixels);

        self.texture.set(image, Default::default());

        /* UI 컨트롤을 위한 패널 추가 */
        egui::Window::new("Controls").show(ctx, |ui| {
            /* 삼각형 개수를 조절하는 슬라이더 추가 */
            let mut num_triangles = self.rasterization.num_triangles;
            if ui
                .add(egui::Slider::new(&mut num_triangles, 3..=100).text("Num Triangles"))
                .changed()
            {
                self.rasterization.num_triangles = num_triangles;
                self.rasterization.setup_circle_geometry(); // 개수가 바뀌면 원을 다시 계산
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.image(&self.texture);
        });

        ctx.request_repaint();
    }
}
