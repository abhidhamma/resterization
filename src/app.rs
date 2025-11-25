use eframe::egui;
use egui::{Color32, ColorImage, Pos2, TextureHandle};

/**
 * C++의 glm 라이브러리 대신 glam 크레이트를 사용합니다.
 * Cargo.toml 파일의 [dependencies] 섹션에 glam = "0.27.0" 을 추가해야 합니다.
 */
use glam::{Vec3, vec2, vec3};

/**
 * C++ 코드의 MyVertex 구조체에 해당합니다.
 * Rust에서는 struct를 사용하여 데이터 구조를 정의합니다.
 * Copy, Clone 트레이트를 derive하면 값 타입처럼 쉽게 복사해서 사용할 수 있습니다.
 */
#[derive(Copy, Clone)]
struct MyVertex {
    pos: Vec3,
    color: Color32,
}

/**
 * C++ 코드의 MyTriangle 구조체에 해당합니다.
 */
#[derive(Copy, Clone)]
struct MyTriangle {
    v0: MyVertex,
    v1: MyVertex,
    v2: MyVertex,
}

/**
 * C++의 Rasterization 클래스에 해당합니다.
 * 렌더링에 필요한 데이터와 로직을 가집니다.
 */
struct Rasterization {
    width: usize,
    height: usize,
    triangle: MyTriangle,
}

impl Rasterization {
    /**
     * Rasterization 구조체의 생성자(constructor) 역할을 하는 함수입니다.
     * Rust에서는 보통 new라는 이름의 연관 함수(associated function)를 만들어 생성자처럼 사용합니다.
     */
    fn new(width: usize, height: usize) -> Self {
        /* 삼각형을 구성하는 3개 정점들의 위치와 색 초기화 */
        let triangle = MyTriangle {
            v0: MyVertex {
                pos: vec3(0.0, 0.5, 1.0),
                color: Color32::from_rgb(255, 0, 0), // Red
            },
            v1: MyVertex {
                pos: vec3(1.0, -0.5, 1.0),
                color: Color32::from_rgb(0, 255, 0), // Green
            },
            v2: MyVertex {
                pos: vec3(-1.0, -0.5, 1.0),
                color: Color32::from_rgb(0, 0, 255), // Blue
            },
        };

        Self {
            width,
            height,
            triangle,
        }
    }

    /**
     * 3차원 월드 좌표를 2차원 래스터(화면) 좌표로 변환합니다.
     * 이번 예제에서는 정투영(Orthographic projection)을 사용합니다.
     */
    fn project_world_to_raster(&self, point: Vec3) -> Pos2 {
        /*
         * NDC(Normalized Device Coordinates)로 변환: [-1, 1] x [-1, 1]
         * 모니터 해상도와 무관하게 정규화된 좌표계입니다.
         * 여기서는 너비가 높이보다 긴 경우만 고려합니다.
         */
        let aspect = self.width as f32 / self.height as f32;
        let point_ndc = vec2(point.x / aspect, point.y);

        /*
         * NDC -> 래스터 화면 좌표계로 변환
         * C++ 원본의 로직을 그대로 따릅니다.
         * y좌표는 아래 방향이 양수이므로 상하를 반전시킵니다.
         */
        let x_scale = 2.0 / self.width as f32;
        let y_scale = 2.0 / self.height as f32;

        Pos2::new(
            (point_ndc.x + 1.0) / x_scale - 0.5,
            (1.0 - point_ndc.y) / y_scale - 0.5,
        )
    }

    /*
     * Edge Function: 한 점이 특정 변(edge)에 대해 어느 쪽에 있는지 판별
     * 결과 > 0: 점이 반시계 방향으로 감은 변의 안쪽에 있음
     * 결과 < 0: 점이 변의 바깥쪽에 있음
     * 결과 = 0: 점이 변 위에 있음
     */
    fn edge_function(v0: Pos2, v1: Pos2, point: Pos2) -> f32 {
        let a = v1 - v0;
        let b = point - v0;
        a.x * b.y - a.y * b.x
    }

    // 래스터라이제이션: 가상의 삼각형으로 구성된 3차원 대상을 모니터에 투영
    fn render(&self, pixels: &mut [Color32]) {
        //월드좌표계의 정점들을 화면좌표계로 변환
        let v0_raster = self.project_world_to_raster(self.triangle.v0.pos);
        let v1_raster = self.project_world_to_raster(self.triangle.v1.pos);
        let v2_raster = self.project_world_to_raster(self.triangle.v2.pos);

        // 삼각형을 감싸는 최소 사각형을 계산하여 탐색 범위 줄이기
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

        // 바운딩박스 내부의 모든 픽셀 순회
        for j in y_min..=y_max {
            for i in x_min..=x_max {
                let point = Pos2::new(i as f32, j as f32);

                // 삼각형 내부에 있는지 판별
                let alpha0 = Self::edge_function(v1_raster, v2_raster, point);
                let alpha1 = Self::edge_function(v2_raster, v0_raster, point);
                let alpha2 = Self::edge_function(v0_raster, v1_raster, point);

                // 세 개의 edge function 결과가 모두 양수(또는 0)이면 픽셀은 삼각형 내부에 있음
                if alpha0 >= 0.0 && alpha1 >= 0.0 && alpha2 >= 0.0 {
                    let area = alpha0 + alpha1 + alpha2;
                    if area == 0.0 {
                        continue;
                    } // 삼각형이 직선으로 찌그러진 경우 분모가 0이 되는 것을 방지

                    let w0 = alpha0 / area;
                    let w1 = alpha1 / area;
                    let w2 = alpha2 / area;

                    // 무게중심 좌표를 이용해 각 정점의 색상을 선형보간
                    let r = (w0 * self.triangle.v0.color.r() as f32
                        + w1 * self.triangle.v1.color.r() as f32
                        + w2 * self.triangle.v2.color.r() as f32) as u8;
                    let g = (w0 * self.triangle.v0.color.g() as f32
                        + w1 * self.triangle.v1.color.g() as f32
                        + w2 * self.triangle.v2.color.g() as f32) as u8;
                    let b = (w0 * self.triangle.v0.color.b() as f32
                        + w1 * self.triangle.v1.color.b() as f32
                        + w2 * self.triangle.v2.color.b() as f32) as u8;

                    // 계산된 색상으로 픽셀 색칠하기
                    let index = i + j * self.width;
                    if index < pixels.len() {
                        pixels[index] = Color32::from_rgb(r, g, b);
                    }
                }
            }
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

        /*
         * egui에서 이미지를 표시하기 위한 TextureHandle을 생성합니다.
         * 처음에는 검은색으로 채워진 빈 이미지를 만듭니다.
         */
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
}

impl eframe::App for RasterizationApp {
    /**
     * eframe의 update 함수는 매 프레임 호출됩니다. C++ 예제의 메인 루프와 같습니다.
     */
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        /* 애니메이션 로직 호출 */
        self.rasterization.update();

        /*
         * 픽셀 버퍼를 가져와서 검은색으로 초기화합니다.
         * C++의 std::vector<vec4> pixels와 동일한 역할을 합니다.
         */
        let mut image = ColorImage::filled(
            [self.rasterization.width, self.rasterization.height],
            Color32::BLACK,
        );

        /* 소프트웨어 래스터라이저를 호출하여 image의 픽셀 데이터를 채웁니다. */
        self.rasterization.render(&mut image.pixels);

        /* 변경된 픽셀 데이터로 텍스처를 업데이트합니다. */
        self.texture.set(image, Default::default());

        /* egui 패널에 텍스처를 이미지로 표시합니다. */
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.image(&self.texture);
        });

        /* UI가 계속해서 다시 그려지도록 요청합니다. */
        ctx.request_repaint();
    }
}
