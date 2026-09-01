#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::{RichText, Vec2};
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
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
}

impl Default for EntryCApp {
    fn default() -> Self {
        Self {
            name: "EntryC GUI 판".to_string(),
        }
    }
}

impl eframe::App for EntryCApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("menubar").show(ui, |ui| {
            egui::menu::MenuBar::new().ui(ui, |ui| {
                ui.label(&self.name);
                ui.menu_button("도움말", |ui| {
                    if ui.button("정보").clicked() {
                        ui.close();
                    }
                })
            })
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_hex("#fff").unwrap()))
            .show(ui, |ui| {
                let rect = ui.available_rect_before_wrap();

                let row_width = 220.0;
                let row_height = 100.0;

                let row_rect = egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, rect.center().y + 25.0),
                    egui::vec2(row_width, row_height),
                );

                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(row_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        boxed(ui, |ui| {
                            ui.add_sized(Vec2::new(50.0, 50.0), egui::Button::new("From"));
                        });

                        ui.add_space(10.0);

                        ui.label("->");

                        ui.add_space(10.0);

                        boxed(ui, |ui| {
                            ui.add_sized(Vec2::new(50.0, 50.0), egui::Button::new("To"));
                        });
                    },
                );

                // 제목은 별도로 화면 정중앙에 배치
                let title_rect = egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, rect.center().y - 50.0),
                    egui::vec2(rect.width(), 30.0),
                );

                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(title_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.label(RichText::new("엔트리 파일을 Rust 로 변환합니다.").size(20.0));
                    },
                );
            });
    }
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
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
