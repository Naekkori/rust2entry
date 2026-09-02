#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use egui::{Button, Color32, Pos2, Rect, RichText, Sense, Shape, TextureHandle, Vec2, emath};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_resizable(false)
            .with_maximize_button(false),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "EntryC",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(EntryCApp::default()))
        }),
    )
}
#[derive(Default)]
struct Arrow {
    current: f32,
    target: f32,
}

#[derive(Default)]
struct EntryCApp {
    name: String,
    entry_description: String,
    entry_toggle_mode: bool,
    group_width: f32,
    group_height: f32,
    arrow: Arrow,
    arrow_texture: Option<TextureHandle>,
}

impl eframe::App for EntryCApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.name = format!(
            "{0} v{1}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        );
        // ─────────────────────────────────────
        // 애니메이션
        // ─────────────────────────────────────
        let dt = ui.ctx().input(|i| i.stable_dt);
        let k = 1.0 - (-8.0 * dt).exp();
        self.arrow.current += (self.arrow.target - self.arrow.current) * k;
        ui.ctx().request_repaint();

        // ─────────────────────────────────────
        // 메뉴바
        // ─────────────────────────────────────
        egui::Panel::top("menubar")
            .show_separator_line(true)
            .show(ui, |ui| {
                egui::menu::MenuBar::new().ui(ui, |ui| {
                    ui.label(&self.name);
                    ui.menu_button("도움말", |ui| {
                        if ui.button("정보").clicked() {
                            ui.close();
                        }
                    });
                });
            });

        // ─────────────────────────────────────
        // 메인 화면
        // ─────────────────────────────────────
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new().fill(egui::Color32::from_hex("#ffffff").expect("failed color")),
            )
            .show(ui, |ui| {
                let available = ui.available_rect_before_wrap();

                // 실제 중앙 좌표
                let center = available.center();

                // 전체 콘텐츠 그룹
                //
                // 제목 + From -> To 를 하나의 그룹으로 묶는다.
                // 이렇게 해야 내부 크기가 바뀌어도 그룹 전체의
                // 중앙을 기준으로 배치할 수 있다.
                let group_width = available.width().min(self.group_width);
                let group_height = self.group_height;

                let group_rect =
                    egui::Rect::from_center_size(center, egui::vec2(group_width, group_height));

                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(group_rect)
                        .layout(egui::Layout::top_down(egui::Align::Center)),
                    |ui| {
                        // 제목
                        ui.label(RichText::new(&self.entry_description).size(20.0));

                        ui.add_space(20.0);

                        // From -> To
                        //
                        // horizontal() 자체가 반환하는
                        // response.rect에 실제 사용 영역이 들어간다.
                        let row = ui.horizontal(|ui| {
                            boxed(ui, |ui| {
                                let img = egui::Image::new(egui::ImageSource::Bytes {
                                    uri: "bytes://rust.svg".into(),
                                    bytes: include_bytes!("../assets/image/rust.svg").into(),
                                });
                                ui.add_sized(Vec2::new(80.0, 80.0), img);
                            });

                            ui.add_space(5.0);

                            // 회전 화살표
                            boxed(ui, |ui| {
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::new(80.0, 80.0), Sense::hover());

                                let texture = self
                                    .arrow_texture
                                    .get_or_insert_with(|| load_arrow_texture(ui.ctx()))
                                    .clone();

                                let mut mesh = egui::Mesh::with_texture(texture.id());
                                mesh.add_rect_with_uv(
                                    rect,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                                mesh.rotate(
                                    emath::Rot2::from_angle(self.arrow.current),
                                    rect.center(),
                                );

                                ui.painter().add(Shape::mesh(Arc::new(mesh)));
                            });

                            ui.add_space(5.0);

                            boxed(ui, |ui| {
                                let img = egui::Image::new(egui::ImageSource::Bytes {
                                    uri: "bytes://entry.png".into(),
                                    bytes: include_bytes!("../assets/image/entry.png").into(),
                                });
                                ui.add_sized(Vec2::new(80.0, 80.0), img);
                            });
                        });

                        ui.add_space(20.0);

                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(
                                [ui.available_width(), 50.0].into(),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    let half = ui.available_width() * 0.5;
                                    ui.add_sized([half, 50.0], Button::new("Rust 소스폴더 열기"));
                                    ui.add_sized(
                                        [half, 50.0],
                                        Button::new(".Ent 엔트리프로젝트 열기"),
                                    );
                                },
                            );
                            /*
                                self.arrow.target += std::f32::consts::PI;
                                self.entry_toggle_mode = !self.entry_toggle_mode;
                            */
                            // 모드토글
                            if self.entry_toggle_mode {
                                self.entry_description =
                                    "엔트리 파일을 Rust 로 컴파일 합니다.".to_owned();
                            } else {
                                self.entry_description =
                                    "Rust 파일을 엔트리 로 컴파일 합니다.".to_owned();
                            }
                        });
                        ui.vertical(|ui| {
                            ui.allocate_ui_with_layout(
                                [ui.available_width(), 50.0].into(),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.add_sized(
                                        [ui.available_width(), 50.0],
                                        Button::new("Rust 소스 열기"),
                                    );
                                },
                            );
                            // 모드토글
                            if self.entry_toggle_mode {
                                self.entry_description =
                                    "엔트리 파일을 Rust 로 컴파일 합니다.".to_owned();
                            } else {
                                self.entry_description =
                                    "Rust 파일을 엔트리 로 컴파일 합니다.".to_owned();
                            }
                        });
                        let actual_width = row.response.rect.width();
                        if (actual_width - self.group_width).abs() > 0.5 {
                            self.group_width = actual_width
                        }

                        let actual_height = ui.min_rect().height();
                        if (actual_height - self.group_height).abs() > 0.5 {
                            self.group_height = actual_height
                        }
                    },
                );
            });
    }
}
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_extras::install_image_loaders(ctx);
    fonts.font_data.insert(
        "nanum".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/font/NanumGothic.ttf")).into(),
    );

    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .expect("Proportional family missing")
        .insert(0, "nanum".to_owned());

    ctx.set_fonts(fonts);
}
fn load_arrow_texture(ctx: &egui::Context) -> TextureHandle {
    let svg_bytes = include_bytes!("../assets/image/arrow.svg").as_ref();
    let color_image = svg_to_color_image(svg_bytes, 80, 80);
    ctx.load_texture("arrow_tex", color_image, egui::TextureOptions::default())
}

fn svg_to_color_image(svg_bytes: &[u8], width: u32, height: u32) -> egui::ColorImage {
    let tree = resvg::usvg::Tree::from_data(svg_bytes, &resvg::usvg::Options::default())
        .expect("failed parse svg");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).expect("pixmap alloc");
    let scale = resvg::tiny_skia::Transform::from_scale(
        width as f32 / tree.size().width(),
        height as f32 / tree.size().height(),
    );
    resvg::render(&tree, scale, &mut pixmap.as_mut());
    egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], pixmap.data())
}
fn boxed(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .inner_margin(egui::Margin::same(20))
        .show(ui, content);
}
