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
    // 직전 프레임에 측정된 컨텐츠 사이즈. None이면 첫 프레임.
    measured: Option<egui::Vec2>,
}

impl Default for EntryCApp {
    fn default() -> Self {
        Self {
            name: "EntryC GUI 판".to_string(),
            measured: None,
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
                // 메뉴바 제외한 패널 영역의 중앙 좌표를 기준으로 동적 중앙 잡기.
                // Area는 측정된 컨텐츠 사이즈만큼 fixed_pos해서 매 프레임 중앙 유지.
                // 첫 프레임은 사이즈 미측정이므로 중앙 anchor로 그리고, 다음 프레임부터
                // fixed_pos로 정확히 패널 중앙에 둔다.
                let panel = ui.max_rect();

                // 컨텐츠를 그릴 때 부모 폭을 헤딩/가로줄 중 더 넓은 쪽(=measured.x)으로 강제한다.
                // 그래야 Layout의 Align::Center가 모든 자식을 같은 폭으로 보고 가운데로 배치한다.
                // 헤딩의 자식폭이 가로줄보다 넓어서 헤딩 박스만 화면 가운데, 가로줄은 자기 폭
                // 가운데로 가는 어긋남을 막는다.
                let available_w = self.measured.map(|sz| sz.x).unwrap_or(panel.width());

                let draw = |ui: &mut egui::Ui| {
                    ui.allocate_ui(egui::vec2(available_w, ui.available_height()), |ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                            ui.label(RichText::new("엔트리 파일을 Rust 로 변환합니다.").size(20.0));
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                boxed(ui, |ui| {
                                    ui.add_sized(Vec2::new(50.0, 50.0), egui::Button::new("From"));
                                });
                                ui.label("->");
                                boxed(ui, |ui| {
                                    ui.add_sized(Vec2::new(50.0, 50.0), egui::Button::new("To"));
                                });
                            });
                        });
                    });
                };

                let area = egui::Area::new(egui::Id::new("centered_block"))
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO);

                let response = if let Some(sz) = self.measured {
                    let pos =
                        egui::pos2(panel.center().x - sz.x * 0.5, panel.center().y - sz.y * 0.5);
                    area.fixed_pos(pos).show(ui.ctx(), draw)
                } else {
                    area.show(ui.ctx(), draw)
                };
                self.measured = Some(response.response.rect.size());
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
