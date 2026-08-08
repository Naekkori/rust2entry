# AGENT.md

AI/에이전트 협업용 진행 문서. Readme와 동기화.

## 진행 상태

| # | 단계 | 상태 | 테스트 |
|---|------|------|--------|
| 1 | `parse` — Rust 소스 → IR Program | ✅ | 19/19 |
| 2 | `block` — IR → Block enum | ✅ | (in 3) |
| 3 | `codegen` — Block → project.json (패치) | ✅ | 9/9 |
| 4 | `deparse` — project.json → IR (역방향) | ✅ | (in 3 라운드트립) |
| 5 | `decodegen` — IR → DSL (Rust-like) | ✅ | (in 1 라운드트립) |
| 6 | `var` — VarInfo / VarMap | ✅ | - |
| 7 | `for-range` — `for i in a..b` → `repeat_basic` 펼침 | ✅ | in 1, 3 |
| 8 | 변수 kind (Timer/Answer/List) 인식 | ✅ | in 3 |
| 9 | `entryc extract` — `.ent` → `.rs` | ✅ | - |
| 10 | `entryc build` — `.rs` → `.ent` | ✅ | 5/5 |
| 11 | `lib::compile` — 전체 조립 | ✅ | 15/15 |

## 완료된 모듈

- `parse` — `syn::File` → `ir::Program`
  - `syn::Item::Fn` → `when_start` / `when_click` / `when_*` 시점은 본문 평탄화, `fn main()` 등은 `FuncDef`
  - `Stmt::VarDecl`, `SetVar`, `FuncDef`, `If`, `While`, `Repeat`, `For`, `Return`, `Break`, `Continue`
  - `Expr::Lit` (Int/Float/Str/Bool), `Binary` (12개), `Unary`, `Path`, `Call`, `Paren`, `Range`
- `block` — `ir` → `Block` enum + `ParamBlock`
  - 타이밍: `WhenStart`/`WhenClick`/`WhenCloneStart`/`WhenMessageRecv`
  - 변수: `SetVar`/`ChangeVar`/`GetVar`/`ShowVar`/`HideVar`
  - 흐름: `If`/`IfElse`/`While`/`Repeat`/`Forever`/`Break`/`Continue`/`StopAll`
  - 산술: `CalcBinOp`/`Compare`/`BoolOp`/`UnaryOp`
  - 리터럴: `Number`/`Text`/`Boolean`
  - 문자열: `StringConcat`/`StringIncludes`
  - 함수: `FuncCall`/`FuncDef`/`Return`
- `codegen` — `Block` → `serde_json::Value`
  - Entry 슬롯 형식 (`params` + `statements` 분리)
  - 변수 드롭다운 `{ id, name, variableType }` (kind별 분기)
  - BinOp → Entry 산술/비교 기호, 슬롯 `[lhs, op, rhs]` 정렬
  - `generate(program, original)` — 원본 project.json에 `scripts`/`variables` 패치
  - `collect_var_map` — IR에서 VarMap 빌드
- `deparse` — Entry project.json `scripts` → IR (역방향, 라운드트립 검증용)
- `decodegen` — IR → Rust-like DSL (라운드트립 검증용, top-level stmt를 `when_start`로 wrap)
- `var` — `VarInfo` / `VarKind` / `VarInit` / `VarMap`
- `for-range` 펼침 — `for i in a..b { body }` → `repeat_basic(b - a)` 안에 `set_variable i a` + body + `change_variable i 1`

## 변수 kind (EntryJS 호환)

이름 기반 자동 인식:

| 변수명 | 인식 kind | Entry `variableType` |
|---|---|---|
| `초시계` / `timer` / `Timer` | `Timer` | `"timer"` |
| `대답` / `answer` / `Answer` | `Answer` | `"answer"` |
| `리스트` / `list` / `List` | `List` | `"list"` |
| 그 외 | `Variable` | `"variable"` |

제약 (A방안, strict):

- `let 초시계 = ...` / `초시계 = ...` → `Error::UnmappedBlock` (Entry 전용 블록만 받음)
- `Expr::Var("초시계")` → 거부 (전용 `get_project_timer_value` 블록 필요)
- `대답`도 동일
- `리스트`는 일반 변수처럼 사용 가능 (Entry가 리스트 슬롯에서 처리)

## 라운드트립 검증

- **codegen ↔ deparse**: `parse → codegen → deparse → IR'` 구조 보존 (스크립트/변수/연산)
- **parse ↔ decodegen**: `parse(src) → IR → emit → dsl → parse(dsl) → IR'` 구조 보존 (조건/반복/변수)

## EntryJS 블록 매핑 현황 (203개 중)

✅ = 매핑됨. `deparse.rs::block_from_value` 의 매치 arm 기준.

### 시작 (3/26)
- ✅ `when_run_button_click` / `when_run` → `WhenStart` (→ `fn when_start()`)
- ✅ `when_object_click` / `when_click` → `WhenClick` (→ `fn when_click()`)
- ✅ `when_clone_start` → `WhenCloneStart` (→ `fn when_clone_start()`)
- ✅ `when_message_cast` → `WhenMessageRecv` (→ `fn when_message_<msg>()`)
- ⬜ `when_some_key_pressed` — □ 키를 눌렀을 때
- ⬜ `mouse_clicked` / `mouse_click_cancled` — 마우스 클릭/해제
- ⬜ `when_object_click_canceled` — 오브젝트 클릭 해제
- ⬜ `message_cast` / `message_cast_wait` — 신호 보내기/보내고 기다리기
- ⬜ `when_scene_start` — 장면이 시작되었을 때
- ⬜ `start_scene` / `start_neighbor_scene` — 장면 시작하기
- ⬜ 내부용 (이름 없음): `check_object_property`, `check_block_execution`, `switch_scope`, `is_answer_submited`, `check_lecture_goal`, `check_variable_by_name`, `show_prompt`, `check_goal_success`, `positive_number`, `negative_number`, `wildcard_string`, `wildcard_boolean`, `register_score`

### 흐름 (8/15)
- ✅ `repeat_basic` → `Repeat` (for-range 펼침)
- ✅ `repeat_while` / `repeat_while_true` → `While`
- ✅ `repeat_inf` / `repeat_forever` → `Forever`
- ✅ `_if` / `if` → `If`
- ✅ `if_else` → `IfElse`
- ✅ `stop_repeat` / `stop_object` → `Break`
- ✅ `continue_repeat` / `_continue` → `Continue`
- ✅ `stop_object` (전체 정지 의미) / `stop_run_all` → `StopAll`
- ⬜ `wait_second` — □ 초 기다리기
- ⬜ `wait_until_true` — □ 이(가) 될 때까지 기다리기
- ⬜ `restart_project` — 처음부터 다시 실행하기
- ⬜ `when_clone_start` → 이미 `WhenCloneStart` 로 매핑됨
- ⬜ `create_clone` / `delete_clone` / `remove_all_clones` — 복제본 생성/삭제

### 움직임 (0/19)
- ⬜ `move_direction` — 이동 방향으로 □ 만큼 움직이기
- ⬜ `bounce_wall` — 화면 끝에 닿으면 튕기기
- ⬜ `move_x` / `move_y` — x/y 좌표를 □ 만큼 바꾸기
- ⬜ `move_xy_time` — □ 초 동안 x:□ y:□ 만큼 움직이기
- ⬜ `locate_x` / `locate_y` / `locate_xy` — x/y/x,y 위치로 이동하기
- ⬜ `locate_xy_time` — □ 초 동안 x:□ y:□ 위치로 이동하기
- ⬜ `locate` — □ 위치로 이동하기
- ⬜ `locate_object_time` — □ 초 동안 □ 위치로 이동하기
- ⬜ `rotate_relative` / `direction_relative` — 방향/이동방향을 □ 만큼 회전하기
- ⬜ `rotate_by_time` / `direction_relative_duration` — □ 초 동안 회전
- ⬜ `rotate_absolute` / `direction_absolute` — 방향/이동방향을 □ (으)로 정하기
- ⬜ `see_angle_object` — □ 쪽 바라보기
- ⬜ `move_to_angle` — □ 방향으로 □ 만큼 움직이기

### 형태 (0/17)
- ⬜ `show` / `hide` — 모양 보이기/숨기기
- ⬜ `dialog` / `dialog_time` — □ 을(를) □ (초 동안) □ □
- ⬜ `remove_dialog` — 말풍선 지우기
- ⬜ `change_to_some_shape` / `change_to_next_shape` — □ 모양으로 바꾸기
- ⬜ `add_effect_amount` / `change_effect_amount` — □ 효과 주기/정하기
- ⬜ `erase_all_effects` — 효과 모두 지우기
- ⬜ `change_scale_size` / `set_scale_size` — 크기 바꾸기/정하기
- ⬜ `stretch_scale_size` — □ 를 □ 만큼 늘이기
- ⬜ `reset_scale_size` — 원래 크기로 되돌리기
- ⬜ `flip_x` / `flip_y` — 상하/좌우 뒤집기
- ⬜ `change_object_index` — □ 보내기 (레이어)

### 붓 (0/13)
- ⬜ `brush_stamp` — 도장 찍기
- ⬜ `start_drawing` / `stop_drawing` — 그리기 시작/멈추기
- ⬜ `start_fill` / `stop_fill` — 채우기 시작/멈추기
- ⬜ `set_color` / `set_random_color` / `set_fill_color` — 색 정하기
- ⬜ `change_thickness` / `set_thickness` — 그리기 굵기
- ⬜ `change_brush_transparency` / `set_brush_tranparency` — 붓 투명도
- ⬜ `brush_erase_all` — 모든 붓 지우기

### 텍스트 (0/9)
- ⬜ `text_read` — 글상자 □의 내용
- ⬜ `text_write` — □ (이)라고 글쓰기
- ⬜ `text_append` / `text_prepend` — 뒤/앞에 추가하기
- ⬜ `text_change_effect` — 텍스트에 효과
- ⬜ `text_change_font` / `text_change_font_color` / `text_change_bg_color` — 글씨체/색/배경색
- ⬜ `text_flush` — 텍스트 모두 지우기

### 소리 (0/16)
- ⬜ `sound_something_with_block` — 소리 □ 재생하기
- ⬜ `sound_something_second_with_block` — 소리 □ □ 초 재생하기
- ⬜ `sound_from_to` — 소리 □ □ 초 부터 □ 초까지 재생하기
- ⬜ `sound_something_wait_with_block` — 소리 □ 재생하고 기다리기
- ⬜ `sound_something_second_wait_with_block` — 소리 □ □ 초 재생하고 기다리기
- ⬜ `sound_from_to_and_wait` — 소리 □ □ 초 부터 □ 초까지 재생하고 기다리기
- ⬜ `sound_volume_change` / `sound_volume_set` — 소리 크기
- ⬜ `get_sound_speed` — 소리 빠르기
- ⬜ `sound_speed_change` / `sound_speed_set` — 소리 빠르기
- ⬜ `sound_silent_all` — □ 소리 멈추기
- ⬜ `play_bgm` / `stop_bgm` — 배경음악
- ⬜ `get_sound_volume` / `get_sound_duration` — 값 슬롯

### 판단 (3/11)
- ✅ `boolean_basic` / `boolean_basic_operator` → `Compare`
- ✅ `boolean_and_or` → `BoolOp`
- ✅ `calc_unary` (boolean_not) → `UnaryOp`
- ⬜ `is_clicked` — 클릭했는가?
- ⬜ `is_object_clicked` — 오브젝트 클릭했는가?
- ⬜ `is_press_some_key` — 키 눌렸는가?
- ⬜ `reach_something` — □ 에 닿았는가?
- ⬜ `is_type` — 타입 체크 (숫자/문자/리스트)
- ⬜ `is_boost_mode` — 부스트 모드인가?
- ⬜ `is_current_device_type` — □ 에서 실행하는가?
- ⬜ `is_touch_supported` — 터치 가능한가?

### 연산 (3/26)
- ✅ `calc_basic` → `CalcBinOp`
- ✅ `number` / `text` / `boolean` → 리터럴
- ⬜ `calc_rand` — □ 부터 □ 사이의 무작위 수
- ⬜ `coordinate_mouse` — 마우스 x/y 좌표
- ⬜ `coordinate_object` — 오브젝트 x/y 좌표
- ⬜ `quotient_and_mod` — □ 를 □ 로 나눈 몫/나머지
- ⬜ `calc_operation` — 삼각함수/절댓값/제곱/제곱근
- ⬜ `get_project_timer_value` — 타이머 값
- ⬜ `choose_project_timer_action` — 타이머 시작/정지/리셋
- ⬜ `set_visible_project_timer` — 타이머 보이기/숨기기
- ⬜ `get_date` — 날짜/시/분/초
- ⬜ `distance_something` — 두 점 사이 거리
- ⬜ `get_user_name` — 아이디
- ⬜ `get_nickname` — 닉네임
- ⬜ `length_of_string` — 문자열 길이
- ⬜ `reverse_of_string` — 문자열 뒤집기
- ⬜ `combine_something` — 문자열 결합
- ⬜ `char_at` — N번째 문자
- ⬜ `substring` — 부분 문자열
- ⬜ `count_match_string` — 포함 횟수
- ⬜ `index_of_string` — 위치 찾기
- ⬜ `replace_string` — 치환
- ⬜ `change_string_case` — 대소문자
- ⬜ `get_block_count` — 블록 수
- ⬜ `change_rgb_to_hex` — RGB → HEX
- ⬜ `change_hex_to_rgb` — HEX → R/G/B
- ⬜ `get_boolean_value` — 값 슬롯 (boolean)

### 변수 (5/19)
- ✅ `set_variable` → `SetVar`
- ✅ `change_variable` → `ChangeVar`
- ✅ `get_variable` → `GetVar`
- ✅ `show_variable` / `hide_variable` → `ShowVar`/`HideVar`
- ⬜ `ask_and_wait` — □ 을(를) 묻고 대답 기다리기
- ⬜ `get_canvas_input_value` — 대답 값
- ⬜ `set_visible_answer` — 대답 보이기/숨기기
- ⬜ `value_of_index_from_list` — 리스트 N번째 값
- ⬜ `add_value_to_list` — 항목 추가
- ⬜ `remove_value_from_list` — N번째 삭제
- ⬜ `insert_value_to_list` — N번째에 삽입
- ⬜ `change_value_list_index` — N번째 값 바꾸기
- ⬜ `length_of_list` — 리스트 길이
- ⬜ `is_included_in_list` — 포함 여부
- ⬜ `show_list` / `hide_list` — 리스트 보이기/숨기기

### 함수 (3/14)
- ✅ `function_call` → `FuncCall`
- ✅ `function_create` → `FuncDef`
- ✅ `function_return` → `Return`
- ⬜ `function_general` — 함수 □ (호출)
- ⬜ `function_value` — 함수 (값)
- ⬜ `function_field_label` — □□
- ⬜ `function_field_string` — □□ (문자)
- ⬜ `function_field_boolean` — □□ (판단)
- ⬜ `function_param_string` — 문자/숫자값 매개변수
- ⬜ `function_param_boolean` — 판단값 매개변수
- ⬜ `function_create_value` — 결괏값 반환 함수 정의
- ⬜ `set_func_variable` / `get_func_variable` — 함수 변수

### 데이터분석 (0/18)
- ⬜ `append_row_to_table` — 테이블에 행 추가
- ⬜ `insert_row_to_table` — 테이블 N번째에 추가
- ⬜ `delete_row_from_table` — 테이블 N번째 삭제
- ⬜ `set_value_from_table` — 테이블 N번째 행의 열 값 바꾸기
- ⬜ `save_current_table` — 테이블 현재 상태로 남기기
- ⬜ `get_table_count` — 테이블 행/열 개수
- ⬜ `get_value_from_table` — 테이블 값
- ⬜ `get_value_from_last_row` — 테이블 마지막 행 값
- ⬜ `calc_values_from_table` — 테이블 통계 (합/평균/최대/최소)
- ⬜ `open_table` / `open_table_wait` — 테이블 창 열기
- ⬜ `open_table_chart` / `close_table_chart` — 차트 창
- ⬜ `get_coefficient` — 상관계수
- ⬜ `set_value_from_cell` / `get_value_from_cell` — 셀 값
- ⬜ `get_value_v_lookup` — VLOOKUP

### 합계

**17/203** 매핑됨 (약 8.4%)

## 남은 작업 (TODO)

- [x] `for-range` IR → Entry 풀어쓰기 (`for i in a..b` → `repeat_basic(b-a)`)
- [x] 변수 kind (Timer/Answer/List) 자동 인식 + 전용 변수 거부
- [x] `generate(program, original)` project.json 패치
- [x] 라운드트립 테스트 (codegen/deparse, parse/decodegen)
- [x] `entryc build` — `.rs` → `.ent` 빌드 모드 (subcommand, --rs/--out/--ent-template)
- [x] `lib::compile` — 전체 조립 + extract 라운드트립용 가짜 오브젝트 패치
- [x] extract 출력 개선 — raw JSON 들여쓰기 + 에러 메시지 다단계 코멘트 + 미매핑 블록 집계 출력
- [x] 매핑 추가 — `when_run`, `when_object_click`, `number` (String 숫자 허용)
- [x] `if_else` 블록 — parse/codegen/roundtrip 테스트
- [ ] **다음 우선순위**
  - [ ] 흐름: `wait_second`, `wait_until_true` (쉬움, 즉시 가치)
  - [ ] 흐름: `repeat_while_true` 별칭 추가 (현재 `repeat_while` 만 매핑)
  - [ ] 연산: `calc_rand` (난수) / `get_project_timer_value` (타이머 값)
  - [ ] 변수: `ask_and_wait` (입력 묻기) / `get_canvas_input_value` (대답 값)
  - [ ] 형태: `show` / `hide` (오브젝트 보이기/숨기기)
- [ ] 중기
  - [ ] Timer/Answer 전용 블록 신택스 (`start_timer()` 등)
  - [ ] Entry scripts 오브젝트별 분배 (extract 진짜 라운드트립)
  - [ ] 나머지 매핑 (이동/회전/소리/리스트/함수 매개변수)
- [ ] 후기
  - [ ] 이미지 차원 자동 측정 (스프라이트 PNG → width/height)
  - [ ] `entities.default` (위치/크기) 처리
  - [ ] 실제 EntryJS import 테스트 (실행 환경 검증)

## 디렉토리

```
entrycore/   라이브러리 (parse/block/codegen/deparse/decodegen/var) + lib::compile
entryc/      CLI (extract/build subcommand, --rs/--out/--ent-template)
target/      빌드 산출물
entryjs-basic-blocks-v2.md  EntryJS 블럭 카탈로그 + 매핑 현황 (203개, 17개 완료)
```
