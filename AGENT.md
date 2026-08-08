# AGENT.md

AI/에이전트 협업용 진행 문서. Readme와 동기화.

## 진행 상태

| # | 단계 | 상태 | 테스트 |
|---|------|------|--------|
| 1 | `parse` — Rust 소스 → IR Program | ✅ | 19/19 |
| 2 | `block` — IR → Block enum | ✅ | (in 3) |
| 3 | `codegen` — Block → project.json (패치) | ✅ (deprecate: 새 코드에선 compile_with_options 사용) | 9/9 |
| 4 | `deparse` — project.json → IR (역방향) | ✅ | (in 3 라운드트립) |
| 5 | `decodegen` — IR → DSL (Rust-like) | ✅ | (in 1 라운드트립) |
| 6 | `var` — VarInfo / VarMap | ✅ | - |
| 7 | `for-range` — `for i in a..b` → `repeat_basic` 펼침 | ✅ | in 1, 3 |
| 8 | 변수 kind (Timer/Answer/List) 인식 | ✅ | in 3 |
| 9 | `entryc extract` — `.ent` → `.rs` | ✅ | - |
| 10 | `entryc build` — `.rs` → `.ent` (+ `--scene` 플래그) | ✅ | 5/5 |
| 11 | `lib::compile` — 전체 조립 (object 매칭, thread 분리, functions/messages emit, Entry 형식) | ✅ | 57/57 |

### lib::compile 세부 동작 (현재)

- **rs 파싱**: `parse::parse` (트리거 body 평탄화, variables 집계) + `parse::parse_with_triggers` (트리거 분리, `TriggerDef`) 이중 호출.
- **object 매칭**: rs stem ↔ `objects[].name` 대소문자 무시. 매칭된 object 의 `script` 를 thread 배열로 패치.
- **trigger 스레드**: 각 `TriggerDef` 별로 `[when_run (또는 when_click/when_clone_start/when_message_cast), ...body_blocks]`. 여러 트리거 → thread 여러 개.
- **helper FuncDef**: object script 가 아니라 `project.functions` 로 emit. 각 항목 = `{id: fn_<hash>, name, content:[function_create_head], param:[{name}]}`. EntryJS `Entry.Code` 호환을 위해 `content` 는 스레드 배열 (`[[block,...],...]`) 이며 thread[0] 은 `function_create` 헤드 블록. 헤드의 `statements[0]` 에 body.
- **function param type 신택스**: 함수 정의 시 `fn f(a: &str, b: BoolParam)` 형태로 param 타입 지정. `StringParam` (default) 또는 `BoolParam`. function_create head 의 `params[0]` 에 `function_field_label` + 각 param 마다 `function_field_string` / `function_field_boolean` chain 으로 emit (EntryJS 가 chain 을 읽어 동적 `func_<id>` 호출 블록 schema 생성).
- **function_call 재작성**: 빌드 시 helper 의 `name -> id` 맵과 `(id, param_names)` 를 만들고 object.script 의 모든 `function_call` 블록을 `func_<id>` 동적 호출 블록으로 재작성. 호출부 params 슬롯은 정의된 param 개수에 맞춰 emit (부족분 null, 초과분 무시). EntryJS `Func.registerFunction` 가 사용자 정의 함수를 `func_<id>` 타입으로 동적 등록. 미정의 호출은 stderr 경고 + 원본 유지.
- **function 이름 중복**: base `functions[].name` 과 충돌 시 `_2`, `_3`, ... suffix (EntryJS 가 name 으로 호출 매칭하므로 중복 방지).
- **빈 배열 항상 emit**: helper/messages 가 없어도 `project.functions = []`, `project.messages = []` emit (EntryJS 가 키 부재 시 안전하지만 명시적 빈 배열이 안전).
- **when_message 트리거**: 메시지 이름 수집 → `project.messages` 에 `{id: <name>, name}` emit (id = name, EntryJS 가 name 으로 매칭).
- **variables**: Entry 실제 .ent 형식 — `{id, name, variableType, value, visible, isCloud, isRealTime, cloudDate, object, x, y}`. `object` 필드는 변수가 등장한 rs stem; **Timer/Answer/Cloud/RealTime/List 는 항상 전역 (null)**.
- **가짜 object** (base 와 매칭 안 되는 rs): `make_fake_object` 가 base 의 첫 sprite 메타 복사하되 pictures/sounds/selectedPictureId 는 비움, id 는 `obj_<djb2(stem)>` stable hash, `objectType` 보존, `scene` 은 `CompileOptions.default_scene` > base 첫 sprite > `"scene1"`, `rotateMethod:"free"`, `lock:false` 기본값 추가.
- **object.script**: 실제 .ent 형식과 동일하게 **JSON 문자열**로 emit (raw 배열 X).
- **`project.scripts`**: base 값으로 복원 (codegen 의 단일 scripts 패치는 무시).
- **variables 머지**: base + 새 빌드 id 기준 union (base 변수 보존, 같은 id 는 새 빌드가 덮음).
- **`unmapped` 누적**: `from_stmt`/`to_value` 의 `UnmappedBlock` 을 `(Value, Vec<String>)` 의 두 번째 반환에 수집. `main::run_build` 가 eprintln 으로 경고 출력. `push_unmapped` 헬퍼로 dedup.
- **codegen::generate** 직접 호출은 deprecated — 새 코드는 `lib::compile_with_options(&rs, &base, &options)` 사용.

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

타입 어노테이션 신택스 (`let x: T = ...`):

| 타입 | 인식 kind | Entry `variableType` |
|---|---|---|
| `CloudVar` | `Cloud` | `"cloud"` (`isCloud: true`) |
| `RealtimeVar` / `RealTimeVar` | `RealTime` | `"realtime"` (`isRealTime: true`) |

타입 어노테이션이 우선 (이름 기반 자동보다). 알 수 없는 타입은 `UnmappedBlock` 에러.

변수 scope 신택스:

| 키워드 | scope | EntryJS `variables[*].object` |
|---|---|---|
| `let x = ...` (함수 내) | `Local` | `object: <rs stem>` (해당 object 에 묶임) |
| `static x: T = ...` (top-level) | `Global` | `object: null` (모든 object 공유) |

Rust 의미 차용 (`let` = 블록 scope, `static` = 프로그램 전역). `const` 는 미지원 (UnmappedBlock).

## 함수 param type (EntryJS 호환)

함수 정의 시 param 에 type 어노테이션 신택스:

| 타입 | EntryJS chain | 기본값 |
|---|---|---|
| `StringParam` | `function_field_string` | default (미지정 시 자동) |
| `BoolParam` | `function_field_boolean` | 명시 필요 |
| `&str` / `&String` / `String` / `i32` / 기타 | `function_field_string` (default) | 자동 |

신택스:
```rust
fn greet(a: StringParam, b: BoolParam) {
    // ...
}
```

호출부 (`greet("hi", true)`) 의 args 는 EntryJS `function_param_*` chain 과 자동 매칭. args 슬롯 개수 = 정의된 param 개수. 부족분 null, 초과분 무시.

제약 (A방안, strict):

- `let 초시계 = ...` / `초시계 = ...` → `Error::UnmappedBlock` (Entry 전용 블록만 받음)
- `Expr::Var("초시계")` → 거부 (전용 `get_project_timer_value` 블록 필요)
- `대답`도 동일
- `리스트`는 일반 변수처럼 사용 가능 (Entry가 리스트 슬롯에서 처리)
- Cloud/RealTime 변수는 `let x: CloudVar = ""` / `let x: RealtimeVar = ""` 형태로 선언. 일반 변수처럼 read/write 가능.

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

### 함수 (8/14)
- ✅ `function_call` → `FuncCall` (빌드 시 `func_<id>` 동적 호출 블록으로 재작성)
- ✅ `function_create` → `FuncDef`
- ✅ `function_return` → `Return`
- ✅ `func_<id>` (동적 함수 호출) → `FuncCall` (deparse 라운드트립)
- ✅ `function_field_label` — 함수 이름 + param chain 시작점
- ✅ `function_field_string` — StringParam param chain
- ✅ `function_field_boolean` — BoolParam param chain
- ⬜ `function_general` — 함수 □ (호출) (EntryJS 동적 func 블록, func_<id> 로 처리됨)
- ⬜ `function_value` — 함수 (값)
- ⬜ `function_param_string` — 값 슬롯 (chain 의 placeholder 로 emit)
- ⬜ `function_param_boolean` — 값 슬롯
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

**22/203** 매핑됨 (약 10.8%)

## 남은 작업 (TODO)

- [x] `for-range` IR → Entry 풀어쓰기 (`for i in a..b` → `repeat_basic(b-a)`)
- [x] 변수 kind (Timer/Answer/List) 자동 인식 + 전용 변수 거부
- [x] `generate(program, original)` project.json 패치 (이제 deprecated, compile_with_options 사용)
- [x] 라운드트립 테스트 (codegen/deparse, parse/decodegen)
- [x] `entryc build` — `.rs` → `.ent` 빌드 모드 (subcommand, --rs/--out/--ent-template, --scene)
- [x] `lib::compile` — 전체 조립 + extract 라운드트립용 가짜 오브젝트 패치
- [x] extract 출력 개선 — raw JSON 들여쓰기 + 에러 메시지 다단계 코멘트 + 미매핑 블록 집계 출력
- [x] extract 생성기 헤더 — `// Generated by entryc X.Y.Z / at YYYY-MM-DD` 모든 경로 (empty/decodegen 성공/실패/deparse 실패/raw 배열) 에 일관 적용. `SOURCE_DATE_EPOCH` 환경 변수로 결정론적 날짜.
- [x] 매핑 추가 — `when_run`, `when_object_click`, `number` (String 숫자 허용)
- [x] `if_else` 블록 — parse/codegen/roundtrip 테스트
- [x] **잠재 위험 정합화** (대형 작업):
  - [x] 스프라이트 위치 초기화 방지 (base entity 복사)
  - [x] object.script 스키마 정합화 (JSON 문자열, trigger thread 분리, object 매칭)
  - [x] `project.functions` / `project.messages` emit
  - [x] `unmapped` 경고 출력 (build)
  - [x] stable id (`obj_<hash>`)
  - [x] scene CLI (`--scene`)
  - [x] variables Entry 형식 (`visible`, `isCloud`, ..., `object: <rs stem>`)
  - [x] interface 기본값 (`menuWidth`, `canvasWidth`)
  - [x] message id = name (EntryJS name 매칭)
  - [x] object 부수 필드 (`rotateMethod`, `lock`)
  - [x] unmapped dedup
- [x] **보류 (EntryJS 확인 필요)** — 확인 완료:
  - [x] base 변수 처리: 기본은 id 기준 union (template 변수 보존). malformed (id/name/variableType 없음) base 변수는 EntryJS silent hash 노이즈 방지를 위해 union 모드에서도 필터링. `--replace-vars` 플래그 / `CompileOptions.replace_variables = true` 로 base 통째 교체 가능.
  - [x] object 필수 필드: `IRawObject` (`src/class/pixi/atlas/model/IRawObject.ts`) 기준 필수 = `id`, `name`, `script`, `objectType`, `rotateMethod`, `scene`, `sprite.pictures`, `sprite.sounds`, `text`, `lock`, `entity`. 부족분 `text` 추가 (textBox base 면 복사, 그 외 name fallback).
- [x] **잠재 위험 추가 정합화 (2차)**:
  - [x] `functions[].content` EntryJS Entry.Code 형식 (스레드 배열) — `[{blocks:[...]}]` → `[[function_create_head]]`
  - [x] `function_call` → `func_<id>` 동적 블록 재작성 (EntryJS 가 호출 시 동적 등록)
  - [x] 호출부 params 슬롯 보존: 정의된 param 개수에 맞춰 emit (부족분 null, 초과분 무시)
  - [x] `deparse` 에 `func_<id>` 매핑 (라운드트립)
  - [x] function param type 신택스 (`StringParam` / `BoolParam`) → `function_field_string` / `function_field_boolean` chain emit
  - [x] function 이름 중복 시 suffix (`_2`, `_3`, ...)
  - [x] 빈 `functions` / `messages` 배열 항상 emit
  - [ ] 트리거 없는 thread 시 `when_run` 자동 prepend (현재 `parse` 가 Item::Fn 만 허용해 dead code, 방어용으로만 유지)
- [ ] **다음 우선순위 (블록 추가)**
  - [ ] 흐름: `wait_second`, `wait_until_true` (쉬움, 즉시 가치)
  - [ ] 흐름: `repeat_while_true` 별칭 추가 (현재 `repeat_while` 만 매핑)
  - [ ] 연산: `calc_rand` (난수) / `get_project_timer_value` (타이머 값)
  - [ ] 변수: `ask_and_wait` (입력 묻기) / `get_canvas_input_value` (대답 값)
  - [ ] 형태: `show` / `hide` (오브젝트 보이기/숨기기)
- [ ] 중기
  - [ ] Timer/Answer 전용 블록 신택스 (`start_timer()` 등)
  - [x] Cloud/RealTime 변수 신택스 (`let x: CloudVar = ""` / `: RealtimeVar = ""`)
  - [ ] Entry scripts 오브젝트별 분배 (extract 진짜 라운드트립)
  - [ ] 나머지 매핑 (이동/회전/소리/리스트/함수 매개변수)
- [ ] 후기
  - [ ] 이미지 차원 자동 측정 (스프라이트 PNG → width/height)
  - [ ] `entities.default` (위치/크기) 처리
  - [ ] 실제 EntryJS import 테스트 (실행 환경 검증)

## 다른 컴퓨터에서 이어서 시작할 때

**현재 작업 디렉토리**: `D:\kkori\rust2entry` (Windows / PowerShell 5.1)

**현재 working tree 상태**: clean (모든 변경 커밋됨)

**마지막 커밋들**:
- `a045d4c feat(generator): extract 출력에 생성기 헤더 prepend`
- `3df669c feat(build): 함수 param type 신택스 (StringParam / BoolParam)`
- `3e473e7 fix(build): function_call args 슬롯 보존 (param arity 맞춤)`
- `ad4ab92 feat(build): 잠재 위험 정합화 3차 + Cloud/RealTime 변수 + let/static scope`

**빌드/테스트 명령**:
```
cargo test                  # 전체 (entryc 6 + codegen 9 + compile 57 + parse 26 = 98 통과)
cargo test -p entrycore     # entrycore 만
cargo test -p entryc        # entryc 만
cargo build                 # 빌드만
```

**샘플 .ent 위치**: `C:\Users\NEKO\Documents\test.ent` (EntryJS 실제 export 형식 참고용, 이 컴퓨터엔 없을 수 있음 — GitHub entryjs 코드 직접 참고)

**다음 할 일 추천 순서**:
1. 블록 매핑 시작 (TODO 의 "다음 우선순위" 섹션) — `wait_second` 같은 쉬운 것부터

## 디렉토리

```
entrycore/   라이브러리 (parse/block/codegen/deparse/decodegen/var) + lib::compile_with_options
             - parse::parse_with_triggers: TriggerDef 분리 (build 전용)
             - CompileOptions: default_scene, replace_variables
             - ir::VarScope: Local (let) / Global (static)
             - ir::ParamKind: String (StringParam) / Bool (BoolParam)
entryc/      CLI (extract/build subcommand, --rs/--out/--ent-template, --scene, --replace-vars)
target/      빌드 산출물
entryjs-basic-blocks-v2.md  EntryJS 블럭 카탈로그 + 매핑 현황 (203개, 22개 완료)
AGENT.md     이 문서
```
