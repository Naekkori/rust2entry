#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use egui::{
    Button, Color32, Id, Image, ImageSource, Modal, Pos2, Rect, RichText, Sense, Shape,
    TextureHandle, Vec2, emath,
};
use rfd::FileDialog;

use crate::entryc::RunOutput;

mod entryc;

/// 디자인 통일용 상수.
mod style {
    use egui::Color32;

    /// 성공 (녹).
    pub const SUCCESS: Color32 = Color32::from_rgb(40, 160, 80);
    /// 실패 (적).
    pub const DANGER: Color32 = Color32::from_rgb(220, 60, 60);
    /// 진행 중 (청).
    pub const PROGRESS: Color32 = Color32::from_rgb(80, 130, 220);
    /// 보조 텍스트 (회색).
    pub const MUTED: Color32 = Color32::from_rgb(120, 120, 120);
    /// 카드 배경 (연회색).
    pub const CARD_BG: Color32 = Color32::from_rgb(248, 248, 250);
    /// 구분선 (옅은 회색).
    pub const DIVIDER: Color32 = Color32::from_rgb(220, 220, 224);

    /// 헤더 위/아래 표준 여백.
    pub const HEADER_PAD_TOP: f32 = 24.0;
    pub const HEADER_PAD_BOTTOM: f32 = 16.0;
    /// 본문 좌우 여백.
    pub const CONTENT_PAD_X: f32 = 24.0;
    /// 하단 액션 바 높이.
    pub const ACTION_BAR_HEIGHT: f32 = 50.0;
}

/// 통일된 헤더 — 큰 타이틀 + 서브 라벨 + (옵션) 우측 위젯.
/// 서브 라벨과 trailing 은 옵션.
fn draw_header(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: Option<&str>,
    title_color: egui::Color32,
    trailing: Option<Box<dyn FnOnce(&mut egui::Ui)>>,
) {
    use style::*;
    ui.add_space(HEADER_PAD_TOP);
    let bar_height = 40.0;
    let (_id, bar_rect) = ui.allocate_space(egui::vec2(ui.available_width(), bar_height));
    let inner =
        egui::Rect::from_min_max(bar_rect.min + egui::vec2(CONTENT_PAD_X, 0.0), bar_rect.max);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.vertical(|ui| {
        ui.label(
            egui::RichText::new(title)
                .size(24.0)
                .strong()
                .color(title_color),
        );
        if let Some(sub) = subtitle {
            ui.label(egui::RichText::new(sub).size(13.0).color(MUTED));
        }
    });
    if let Some(t) = trailing {
        child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            t(ui);
        });
    }
    ui.add_space(HEADER_PAD_BOTTOM);
    ui.painter().rect_filled(
        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 1.0)),
        0.0,
        DIVIDER,
    );
    ui.add_space(12.0);
}

/// 하단 액션 바 — 좌측 status / 우측 buttons.
/// trailing 은 status 와 buttons 사이 중앙에 배치 (예: 스피너).
fn draw_action_bar<F: FnOnce(&mut egui::Ui), T: FnOnce(&mut egui::Ui)>(
    ui: &mut egui::Ui,
    status: Option<&str>,
    trailing: Option<T>,
    right_buttons: F,
) {
    use style::*;
    let bar_height = ACTION_BAR_HEIGHT;
    let (_id, rect) = ui.allocate_space(egui::vec2(ui.available_width(), bar_height));
    ui.painter().rect_filled(
        egui::Rect::from_min_max(rect.min, rect.min + egui::vec2(rect.width(), 1.0)),
        0.0,
        DIVIDER,
    );
    let inner = egui::Rect::from_min_max(
        rect.min + egui::vec2(CONTENT_PAD_X, 8.0),
        rect.max - egui::vec2(CONTENT_PAD_X, 8.0),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    if let Some(s) = status {
        child.label(egui::RichText::new(s).size(13.0).color(MUTED));
    }
    if let Some(t) = trailing {
        child.add_space(12.0);
        t(&mut child);
    }
    child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        right_buttons(ui);
    });
}

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
    is_enabled: bool,
    current: f32,
    target: f32,
}

/// 현재 화면.
#[derive(Default)]
enum View {
    #[default]
    Home,
    /// 컴파일/추출 진행 중. 백그라운드 스레드가 끝나면 Result 로 전환.
    Compiling,
    /// 완료된 결과 표시.
    Result,
    /// egui 모달 — 에러 알림 + 사용자 결정.
    Error,
    /// 정보
    About,
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
    view: View,
    /// 컴파일 진행 중 누적 메시지 (백그라운드 스레드 → UI).
    progress: Arc<Mutex<Vec<String>>>,
    /// 백그라운드 컴파일 스레드 핸들. 끝나면 join 해서 last_output 으로 이동.
    join: Option<JoinHandle<RunOutput>>,
    /// 완료된 컴파일 결과.
    last_output: Option<entryc::RunOutput>,
    /// 모달에 표시할 메시지. Some 일 때만 View::Error 표시.
    error_message: Option<String>,
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
                            self.view = View::About;
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
            .show(ui, |ui| match self.view {
                View::Home => self.show_home(ui),
                View::Compiling => self.show_compiling(ui),
                View::Result => self.show_result(ui),
                View::Error => self.show_error(ui),
                View::About => self.show_about(ui),
            });
    }
}

impl EntryCApp {
    /// 메인 화면 — 입력 버튼 + 화살표 애니메이션.
    fn show_home(&mut self, ui: &mut egui::Ui) {
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
                        ui.add_enabled_ui(self.arrow.is_enabled, |ui|{
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
                            ui.spacing_mut().item_spacing.x = 3.0;
                            let half = ui.available_width() * 0.5;
                            // Rust 소스 폴더 열기 버튼
                            if ui
                                .add_sized([half, 50.0], Button::new("Rust 소스 폴더 열기"))
                                .on_hover_text("Rust 소스 폴더를 선택합니다. 여러 개의 소스를 컴파일할 때 사용합니다.")
                                .clicked()
                            {
                                self.on_click_open_rust_source_folder();
                            }
                            // 엔트리 프로젝트 열기 버튼
                            if ui
                                .add_sized(
                                    [half, 50.0],
                                    Button::new("엔트리 프로젝트 열기"),
                                )
                                .on_hover_text("엔트리 프로젝트를 선택합니다.")
                                .clicked()
                            {
                                self.on_click_open_entry_project();
                            }
                        },
                    );
                    /*
                        self.arrow.target += std::f32::consts::PI;
                        self.entry_toggle_mode = !self.entry_toggle_mode;
                    */
                });
                let bottom_w = ui.available_width();
                // Rust 소스 열기 버튼
                if ui
                    .add_sized([bottom_w, 50.0], Button::new("Rust 소스 열기"))
                    .on_hover_text("Rust 개별 소스를 선택합니다. 하나만 선택할 때 사용합니다.")
                    .clicked()
                {
                    self.on_click_open_rust_source();
                }
                let actual_width = row.response.rect.width();
                if (actual_width - self.group_width).abs() > 0.5 {
                    self.group_width = actual_width
                }

                let actual_height = ui.min_rect().height();
                if (actual_height - self.group_height).abs() > 0.5 {
                    self.group_height = actual_height
                }

                // 모드토글 및 화살표 활성 상태에 따라 설명 갱신
                self.entry_description = if self.arrow.is_enabled {
                    if self.entry_toggle_mode {
                        "엔트리 파일을 Rust로 컴파일합니다."
                    } else {
                        "Rust 파일을 엔트리로 컴파일합니다."
                    }
                } else {
                    "컴파일할 파일을 열어주세요."
                }
                .to_owned();
            },
        );
    }

    /// 컴파일/추출 진행 화면 — 스피너 + 누적 메시지 + 백그라운드 join 폴링.
    fn show_compiling(&mut self, ui: &mut egui::Ui) {
        use style::*;

        // 백그라운드 완료 확인
        if let Some(handle) = self.join.take() {
            if handle.is_finished() {
                match handle.join() {
                    Ok(out) => {
                        self.last_output = Some(out);
                        self.view = View::Result;
                        return;
                    }
                    Err(_) => {
                        self.last_output = Some(RunOutput {
                            stdout: Vec::new(),
                            stderr: vec!["컴파일 스레드 panic".to_string()],
                            status: "컴파일 스레드가 비정상 종료됨".to_string(),
                            ok: false,
                        });
                        self.view = View::Result;
                        return;
                    }
                }
            } else {
                self.join = Some(handle);
                ui.ctx().request_repaint();
            }
        }

        // 헤더 — 스피너는 본문 액션 바로 이동.
        draw_header(
            ui,
            "컴파일 진행 중",
            Some("백그라운드에서 처리 중입니다"),
            PROGRESS,
            None,
        );

        // ─── 로그 영역 — 가용 영역 전체 사용 ───
        // 좌우 여백 CONTENT_PAD_X 두고 박스 할당 후 카드 + ScrollArea.
        let total_h = (ui.available_height() - ACTION_BAR_HEIGHT - 8.0).max(0.0);
        let (_id, outer_rect) = ui.allocate_space(egui::vec2(ui.available_width(), total_h));
        let card_rect = egui::Rect::from_min_max(
            outer_rect.min + egui::vec2(CONTENT_PAD_X, 0.0),
            outer_rect.max - egui::vec2(CONTENT_PAD_X, 0.0),
        );
        ui.painter()
            .rect_filled(card_rect, egui::CornerRadius::same(8), CARD_BG);
        let inner = card_rect.shrink(16.0);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(&mut child, |ui| {
                let progress = self.progress.lock().unwrap().clone();
                if progress.is_empty() {
                    ui.label(RichText::new("대기 중...").italics().color(MUTED));
                } else {
                    for line in &progress {
                        ui.label(
                            egui::RichText::new(line)
                                .family(egui::FontFamily::Monospace)
                                .size(12.0),
                        );
                    }
                }
            });

        // ─── 하단 액션 바 — 스피너를 status 옆에 ───
        draw_action_bar(
            ui,
            Some("진행 중"),
            Some(|ui: &mut egui::Ui| {
                ui.spinner();
            }),
            |ui| {
                ui.add_enabled(false, |ui: &mut egui::Ui| {
                    ui.add_sized([100.0, 32.0], Button::new("홈으로"))
                });
            },
        );
    }

    /// 완료된 결과 화면.
    fn show_result(&mut self, ui: &mut egui::Ui) {
        use style::*;

        let out = match &self.last_output {
            Some(o) => o.clone(),
            None => {
                self.view = View::Home;
                return;
            }
        };

        let title = if out.ok { "성공" } else { "실패" };
        let color = if out.ok { SUCCESS } else { DANGER };
        draw_header(ui, title, Some(&out.status), color, None);

        // ─── 결과 본문 — 가용 영역 전체 사용 ───
        let total_h = (ui.available_height() - ACTION_BAR_HEIGHT - 8.0).max(0.0);
        let (_id, outer_rect) = ui.allocate_space(egui::vec2(ui.available_width(), total_h));
        let card_rect = egui::Rect::from_min_max(
            outer_rect.min + egui::vec2(CONTENT_PAD_X, 0.0),
            outer_rect.max - egui::vec2(CONTENT_PAD_X, 0.0),
        );
        ui.painter()
            .rect_filled(card_rect, egui::CornerRadius::same(8), CARD_BG);
        let inner = card_rect.shrink(16.0);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(false)
            .show(&mut child, |ui| {
                if !out.stdout.is_empty() {
                    ui.label(RichText::new("stdout").size(13.0).strong().color(MUTED));
                    ui.add_space(4.0);
                    for line in &out.stdout {
                        ui.label(
                            egui::RichText::new(line)
                                .family(egui::FontFamily::Monospace)
                                .size(12.0),
                        );
                    }
                }
                if !out.stderr.is_empty() {
                    if !out.stdout.is_empty() {
                        ui.add_space(12.0);
                    }
                    ui.label(RichText::new("stderr").size(13.0).strong().color(DANGER));
                    ui.add_space(4.0);
                    for line in &out.stderr {
                        ui.label(
                            egui::RichText::new(line)
                                .family(egui::FontFamily::Monospace)
                                .size(12.0)
                                .color(DANGER),
                        );
                    }
                }
            });

        draw_action_bar(ui, None, None::<fn(&mut egui::Ui)>, |ui| {
            if ui.add_sized([100.0, 32.0], Button::new("홈으로")).clicked() {
                self.view = View::Home;
                self.last_output = None;
            }
        });
    }

    /// MSGBOX 헬퍼 — 메시지 받아서 모달 띄움. 다음 프레임의 `show_error` 가 렌더.
    /// 어디서든 호출 가능 (핸들러, 스폰 직전 등). 모달 확인 시 자동 홈 복귀.
    fn show_error_modal(&mut self, msg: impl Into<String>) {
        self.error_message = Some(msg.into());
        self.view = View::Error;
    }

    /// 모달 — 에러 알림. 메시지 영역 + 확인 버튼. 확인 시 홈으로.
    /// `error_message` 가 None 이면 안전하게 홈으로 폴백.
    fn show_error(&mut self, ui: &mut egui::Ui) {
        use style::*;

        let msg = self
            .error_message
            .clone()
            .unwrap_or_else(|| "알 수 없는 오류".to_string());

        draw_header(ui, "오류", None, DANGER, None);

        // 메시지 카드
        let total_h = (ui.available_height() - ACTION_BAR_HEIGHT - 8.0).max(0.0);
        let (_id, outer_rect) = ui.allocate_space(egui::vec2(ui.available_width(), total_h));
        let card_rect = egui::Rect::from_min_max(
            outer_rect.min + egui::vec2(CONTENT_PAD_X, 0.0),
            outer_rect.max - egui::vec2(CONTENT_PAD_X, 0.0),
        );
        ui.painter()
            .rect_filled(card_rect, egui::CornerRadius::same(8), CARD_BG);
        let inner = card_rect.shrink(16.0);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(false)
            .show(&mut child, |ui| {
                ui.label(
                    egui::RichText::new(&msg)
                        .family(egui::FontFamily::Monospace)
                        .size(13.0)
                        .color(DANGER),
                );
            });

        draw_action_bar(ui, None, None::<fn(&mut egui::Ui)>, |ui| {
            if ui.add_sized([100.0, 32.0], Button::new("확인")).clicked() {
                self.view = View::Home;
                self.error_message = None;
            }
        });
    }

    fn show_about(&mut self, ui: &mut egui::Ui) {
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
                boxed(ui, |ui| {
                    let image = egui::Image::new(ImageSource::Bytes {
                        uri: "bytes://crab.png".into(),
                        bytes: include_bytes!("../assets/image/crab.png").into(),
                    });
                    ui.add_sized(Vec2::new(80.0, 80.0), image);
                    ui.label(RichText::new("EntryC GUI").size(48.0).strong());
                });
                let about_text = format!(
                    "Version {0}\n{1}\n{2}",
                    env!("CARGO_PKG_VERSION"),
                    env!("CARGO_PKG_AUTHORS"),
                    env!("CARGO_PKG_DESCRIPTION")
                );
                ui.label(RichText::new(about_text).text_style(egui::TextStyle::Body));
                ui.hyperlink_to("프로젝트 홈페이지", env!("CARGO_PKG_HOMEPAGE"));
                ui.add_space(20.5);
                if ui.add_sized([100.0, 32.0], Button::new("확인")).clicked() {
                    self.view = View::Home;
                    self.error_message = None;
                }
            },
        );
    }
    /// 백그라운드 스레드에서 entryc::run_build 를 호출하고 결과로 전환.
    /// 진행 메시지는 progress Arc<Mutex<...>> 에 push 한다.
    /// `template` 이 Some 이면 base project.json 으로 머지, None 이면 빈 프로젝트.
    fn spawn_build(&mut self, rs_files: Vec<PathBuf>, out: PathBuf, template: Option<PathBuf>) {
        // 화살표 / 토글 갱신
        self.arrow.is_enabled = true;
        self.entry_toggle_mode = false;
        self.arrow.target = 0.0;

        // 진행 메시지 버퍼 초기화
        self.progress = Arc::new(Mutex::new(Vec::new()));
        let progress = Arc::clone(&self.progress);

        self.view = View::Compiling;
        self.join = Some(std::thread::spawn(move || {
            // 진행 메시지를 별도 Arc 로 받아서 run_build 끝난 뒤 merge
            // (run_build 내부에서 progress 직접 접근은 못하므로,
            //  RunOutput 의 stdout/stderr 를 그대로 progress 로 전달)
            let template_ref = template.as_deref();
            let result = entryc::run_build(
                &rs_files,
                template_ref,
                &out,
                None,
                false,
                None,
                Some(&progress),
            );
            let o = match result {
                Ok(o) => o,
                Err(o) => o,
            };
            {
                let mut p = progress.lock().unwrap();
                p.extend(o.stdout.iter().cloned());
                p.extend(o.stderr.iter().cloned());
            }
            o
        }));
    }

    /// 백그라운드 스레드에서 entryc::run_extract 를 호출.
    fn spawn_extract(&mut self, ent: PathBuf, out: Option<PathBuf>) {
        self.arrow.is_enabled = true;
        self.entry_toggle_mode = true;
        self.arrow.target = std::f32::consts::PI;

        self.progress = Arc::new(Mutex::new(Vec::new()));
        let progress = Arc::clone(&self.progress);

        self.view = View::Compiling;
        self.join = Some(std::thread::spawn(move || {
            let result = entryc::run_extract(ent, out, None, Some(&progress));
            let o = match result {
                Ok(o) => o,
                Err(o) => o,
            };
            {
                let mut p = progress.lock().unwrap();
                p.extend(o.stdout.iter().cloned());
                p.extend(o.stderr.iter().cloned());
            }
            o
        }));
    }

    // Rust 소스 폴더 열기 버튼 핸들러 — 폴더 → 폴더 안 .rs 빌드.
    fn on_click_open_rust_source_folder(&mut self) {
        let folder = match FileDialog::new()
            .set_title("Rust 소스 폴더 선택")
            .pick_folder()
        {
            Some(f) => f,
            None => return,
        };

        let rs_files: Vec<PathBuf> = std::fs::read_dir(&folder)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "rs"))
            .collect();

        if rs_files.is_empty() {
            // 빈 폴더는 백그라운드 없이 바로 결과 화면으로
            self.last_output = Some(RunOutput {
                stdout: Vec::new(),
                stderr: vec![format!("폴더에 .rs 파일 없음: {}", folder.display())],
                status: "폴더 안에 .rs 파일이 없습니다.".to_string(),
                ok: false,
            });
            self.view = View::Result;
            return;
        }

        let folder_name = folder
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("build");
        let out = folder.with_file_name(format!("{folder_name}.ent"));
        match pick_template_for(&folder, Some(&folder_name)) {
            TemplateChoice::Found(t) => self.spawn_build(rs_files, out, Some(t)),
            TemplateChoice::Missing => {
                self.show_error_modal(format!(
                    "같은 이름의 .ent 가 없습니다: {folder_name}.ent\n폴더 안에 {folder_name}.ent 를 두거나 빈 프로젝트로 빌드합니다."
                ));
            }
            TemplateChoice::PickFromUser => {
                let picked = FileDialog::new()
                    .add_filter("Entry Project", &["ent"])
                    .set_title("머지할 템플릿 .ent 선택")
                    .set_directory(&folder)
                    .pick_file();
                if let Some(t) = picked {
                    self.spawn_build(rs_files, out, Some(t));
                }
            }
        }
    }

    // 엔트리 프로젝트 열기 버튼 핸들러 — .ent → .rs 추출.
    fn on_click_open_entry_project(&mut self) {
        let ent = match FileDialog::new()
            .add_filter("Entry Project", &["ent"])
            .set_title("엔트리 프로젝트 선택")
            .pick_file()
        {
            Some(f) => f,
            None => return,
        };
        self.spawn_extract(ent, None);
    }

    // Rust 소스 열기 버튼 핸들러 — 단일 .rs → .ent 빌드.
    fn on_click_open_rust_source(&mut self) {
        let rs = match FileDialog::new()
            .add_filter("Rust", &["rs"])
            .set_title("Rust 소스 선택")
            .pick_file()
        {
            Some(f) => f,
            None => return,
        };

        // out: .rs 옆에 같은 stem 으로 .ent
        let stem = rs.file_stem().and_then(|n| n.to_str()).unwrap_or("build");
        let out = rs.with_file_name(format!("{stem}.ent"));
        let dir = rs.parent().unwrap_or_else(|| std::path::Path::new("."));
        match pick_template_for(dir, Some(stem)) {
            TemplateChoice::Found(t) => self.spawn_build(vec![rs], out, Some(t)),
            TemplateChoice::Missing => {
                self.show_error_modal(format!(
                    "같은 이름의 .ent 가 없습니다: {stem}.ent\n옆에 {stem}.ent 를 두거나 빈 프로젝트로 빌드합니다."
                ));
            }
            TemplateChoice::PickFromUser => {
                let picked = FileDialog::new()
                    .add_filter("Entry Project", &["ent"])
                    .set_title("머지할 템플릿 .ent 선택")
                    .set_directory(dir)
                    .pick_file();
                if let Some(t) = picked {
                    self.spawn_build(vec![rs], out, Some(t));
                }
            }
        }
    }
}

/// template 선택 결과.
enum TemplateChoice {
    /// 후보 1개 — 바로 머지.
    Found(PathBuf),
    /// 후보 0개 — 같은 이름의 .ent 가 없음. 호출자가 모달 띄우고 빌드 취소.
    Missing,
    /// 후보 2개 이상 — 호출자가 rfd 로 사용자에게 선택 받음.
    PickFromUser,
}

/// 같은 위치(또는 부모 디렉토리)에 있는 .ent 후보를 탐지해 template 선택 결과를 반환.
///
/// stem 이 주어지면 다음 위치에서 `<stem>.ent` 를 찾는다 (둘 다 후보 가능):
///   1. `dir/<stem>.ent`           — `.rs` 와 같은 폴더
///   2. `dir/../<stem>.ent`        — 부모 폴더 (.rs 폴더와 .ent 가 형제인 케이스)
/// stem 이 None 이면 그 디렉토리의 모든 .ent 만 후보.
fn pick_template_for(dir: &std::path::Path, stem: Option<&str>) -> TemplateChoice {
    let candidates: Vec<PathBuf> = match stem {
        Some(s) => {
            let mut found = Vec::new();
            let direct = dir.join(format!("{s}.ent"));
            if direct.is_file() {
                found.push(direct);
            }
            if let Some(parent) = dir.parent() {
                let in_parent = parent.join(format!("{s}.ent"));
                if in_parent.is_file() && !found.contains(&in_parent) {
                    found.push(in_parent);
                }
            }
            found
        }
        None => std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "ent"))
            .collect(),
    };
    match candidates.len() {
        0 => TemplateChoice::Missing,
        1 => TemplateChoice::Found(candidates.into_iter().next().unwrap()),
        _ => TemplateChoice::PickFromUser,
    }
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_extras::install_image_loaders(ctx);
    fonts.font_data.insert(
        "nanum".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/font/NanumSquareR.ttf")).into(),
    );
    fonts.font_data.insert(
        "d2coding".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/font/D2Coding.ttf")).into(),
    );

    // Proportional: 한글 본문 (NanumSquareR)
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .expect("Proportional family missing")
        .insert(0, "nanum".to_owned());

    // Monospace: 한글 로그 (D2Coding — 진짜 모노스페이스 + 한글 glyph)
    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .expect("Monospace family missing")
        .insert(0, "d2coding".to_owned());

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
