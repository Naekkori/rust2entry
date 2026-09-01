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
            .frame(egui::Frame::new().fill(egui::Color32::from_hex("#fff").expect("failed color")))
            .show(ui, |ui| {
                // 표준 패턴: 패널 폭 안에서 Layout::top_down(Align::Center)로 자식 가운데.
                // 가로줄은 ui.horizontal로 잡으면 packed 결과의 폭 = max_width 가 되어서
                // 부모 Layout::Center의 자식 packed가 가운데 정렬되지만 자식 자체가 좌측
                // 정렬된 packed rect를 가지므로 첫 자식이 좌측에 보임. 이걸 막으려면
                // 가로줄에서 ui.horizontal 대신 ui.with_layout(left_to_right(Center))
                // + 직전 프레임 측정 가로 폭으로 Ui를 잡아 packed_width == 자식 union 으로.
                let panel = ui.max_rect();
                let panel_w = panel.width();

                ui.allocate_ui(egui::vec2(panel_w, panel.height()), |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.label(RichText::new("엔트리 파일을 Rust 로 변환합니다.").size(20.0));
                        ui.add_space(5.0);
                        // 가로줄: max_width를 자식 packed보다 약간 크게(320) 잡아서
                        // Layout::left_to_right(Align::Center)에서 자식 union이
                        // 가운데 정렬되게 한다.
                        ui.allocate_ui(egui::vec2(320.0, 100.0), |ui| {
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    boxed(ui, |ui| {
                                        ui.add_sized(
                                            Vec2::new(50.0, 50.0),
                                            egui::Button::new("From"),
                                        );
                                    });
                                    ui.add_space(4.0);
                                    ui.label("->");
                                    ui.add_space(4.0);
                                    boxed(ui, |ui| {
                                        ui.add_sized(
                                            Vec2::new(50.0, 50.0),
                                            egui::Button::new("To"),
                                        );
                                    });
                                },
                            );
                        });
                    });
                });
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
