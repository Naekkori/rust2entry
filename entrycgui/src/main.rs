#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::{RichText, Vec2};

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

struct EntryCApp {
    name: String,
    group_width: f32,
}

impl Default for EntryCApp {
    fn default() -> Self {
        Self {
            name: "EntryC GUI 판".to_string(),
            group_width: 250.0,
        }
    }
}

impl eframe::App for EntryCApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // ─────────────────────────────────────
        // 메뉴바
        // ─────────────────────────────────────
        egui::Panel::top("menubar").show(ui, |ui| {
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
                let group_height = 160.0;

                let group_rect =
                    egui::Rect::from_center_size(center, egui::vec2(group_width, group_height));

                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(group_rect)
                        .layout(egui::Layout::top_down(egui::Align::Center)),
                    |ui| {
                        // 제목
                        ui.label(RichText::new("엔트리 파일을 Rust 로 변환합니다.").size(20.0));

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
                                ui.add_sized(Vec2::new(50.0, 50.0), img);
                            });

                            ui.add_space(10.0);

                            ui.label("->");

                            ui.add_space(10.0);

                            boxed(ui, |ui| {
                                let img = egui::Image::new(egui::ImageSource::Bytes {
                                    uri: "bytes://entry.png".into(),
                                    bytes: include_bytes!("../assets/image/entry.png").into(),
                                });
                                ui.add_sized(Vec2::new(50.0, 50.0), img);
                            });
                        });

                        let actual_width = row.response.rect.width();
                        if (actual_width - self.group_width).abs() > 0.5 {
                            self.group_width = actual_width
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

fn boxed(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .inner_margin(egui::Margin::same(20))
        .show(ui, content);
}
