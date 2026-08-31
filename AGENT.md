# AGENT.md

AI/에이전트 협업용 진행 문서. Readme와 동기화.

> 블록 매핑 현황의 합계는 **기본 187 + AI 105 + 확장 42 = 334개** 목표. EntryJS 의 350개 중 시작 13개 (check_* / wildcard_* / register_score / positive_number / negative_number / show_prompt / check_goal_success / check_lecture_goal / check_object_property / check_block_execution / switch_scope / is_answer_submited / check_variable_by_name) + 함수 UI 3개 (functionAddButton / function_name / showFunctionPropsButton) 는 평가/검증용 으로 사용자라 직접 블록 패널에서 사용하지 않음. 흐름의 `repeat_while_true` (Rust native `while` 로 커버) 와 `when_clone_start` (시작 카테고리에 이미 매핑) 도 duplicate 이라 매핑 대상 아님.
> 2026-08-13 AI/확장 목표 추가 — 기본 187 + AI 105 + 확장 42 = 334개, 기본 매핑 합계 80.

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
| 10 | `entryc build` — `.rs` → `.ent` (+ `--scene` 플래그) | ✅ | 6/6 |
| 11 | `lib::compile` — 전체 조립 (object 매칭, thread 분리, functions/messages emit, Entry 형식) | ✅ | 191/191 |

### lib::compile 세부 동작 (현재)

- **rs 파싱**: `parse::parse` (트리거 body 평탄화, variables 집계) + `parse::parse_with_triggers` (트리거 분리, `TriggerDef`) 이중 호출.
- **object 매칭**: rs stem ↔ `objects[].name` 대소문자 무시. 매칭된 object 의 `script` 를 thread 배열로 패치.
- **trigger 스레드**: 각 `TriggerDef` 별로 `[when_run (또는 when_click/when_clone_start/when_message_cast), ...body_blocks]`. 여러 트리거 → thread 여러 개.
- **helper FuncDef**: object script 가 아니라 `project.functions` 로 emit. 각 항목 = `{id: fn_<hash>, name, content:[function_create_head], param:[{name}]}`. EntryJS `Entry.Code` 호환을 위해 `content` 는 스레드 배열 (`[[block,...],...]`) 이며 thread[0] 은 `function_create` 헤드 블록. 헤드의 `statements[0]` 에 body.
- **function param type 신택스**: 함수 정의 시 `fn f(a: &str, b: BoolParam)` 형태로 param 타입 지정. `StringParam` (default) 또는 `BoolParam`. function_create head 의 `params[0]` 에 `function_field_label` (EntryJS `params[0]` = `{type:'TextInput', value:name}` 객체 — `script.getField('NAME')` 경로) + 각 param 마다 `function_field_string` / `function_field_boolean` chain 으로 emit (EntryJS 가 chain 을 읽어 동적 `func_<id>` 호출 블록 schema 생성).
- **function_call 재작성**: 빌드 시 helper 의 `name -> Vec<(id, param_names)>` (같은 이름 + 다른 arity 가 공존할 수 있어 arity 별로 누적) �을 만들고 object.script 의 모든 `function_call` 블록을 `func_<id>` 동적 호출 블록으로 재작성. 매칭은 `args.len()` 으로 정확 매칭 우선, 실패 시 가장 가까운 arity fallback. 호출부 params 슬롯은 정의된 param 개수에 맞춰 emit (부족분 null, 초과분 무시). EntryJS `Func.registerFunction` 가 사용자 정의 함수를 `func_<id>` 타입으로 동적 등록. 미정의 호출은 stderr 경고 + 원본 유지.
- **function 이름 중복**: base `functions[].name` 과 충돌 시 `_2`, `_3`, ... suffix (EntryJS 가 name 으로 호출 매칭하므로 중복 방지).
- **빈 배열 항상 emit**: helper/messages 가 없어도 `project.functions = []`, `project.messages = []` emit (EntryJS 가 키 부재 시 안전하지만 명시적 빈 배열이 안전).
- **when_message 트리거**: 메시지 이름 수집 → `project.messages` 에 `{id: <name>, name}` emit (id = name, EntryJS 가 name 으로 매칭).
- **시작 액션 reserved 호출**: `send_message("foo")` / `wait_message("foo")` / `start_scene("id")` / `start_next_scene()` / `start_prev_scene()` 는 IR 에서는 일반 `Expr::Call` 으로 파싱되지만 `block::from_stmt` 에서 reserved 이름 매칭 시 별도 Block variant (`MessageCast` 등) 으로 변환. FuncCall 로 emit 되지 않고 EntryJS 의 실제 블록 type (`message_cast` 등) 으로 emit.
- **variables**: Entry 실제 .ent 형식 — `{id, name, variableType, value, visible, isCloud, isRealTime, cloudDate, object, x, y}`. `object` 필드는 변수가 등장한 rs stem; **Timer/Answer/Cloud/RealTime/List 는 항상 전역 (null)**.
- **가짜 object** (base 와 매칭 안 되는 rs): `make_fake_object` 가 base 의 첫 sprite 메타 복사하되 pictures/sounds/selectedPictureId 는 비움, id 는 `obj_<djb2(stem)>` stable hash, `objectType` 보존, `scene` 은 `CompileOptions.default_scene` > base 첫 sprite > `"scene1"`, `rotateMethod:"free"`, `lock:false` 기본값 추가.
- **object.script**: 실제 .ent 형식과 동일하게 **JSON 문자열**로 emit (raw 배열 X).
- **`project.scripts`**: base 값으로 복원 (codegen 의 단일 scripts 패치는 무시).
- **variables 머지**: base + 새 빌드 id 기준 union (base 변수 보존, 같은 id 는 새 빌드가 덮음).
- **`unmapped` 누적**: `from_stmt`/`to_value` 의 `UnmappedBlock` 을 `(Value, Vec<String>)` 의 두 번째 반환에 수집. `main::run_build` 가 eprintln 으로 경고 출력. `push_unmapped` 헬퍼로 dedup.
- **codegen::generate** 직접 호출은 deprecated — 새 코드는 `lib::compile_with_options(&rs, &base, &options)` 사용.

## 하드웨어 소스맵 (hw_sourcemap) — 2026-08-13

하드웨어 블럭(entryjs 하드웨어 장치, 수천 개)은 하나하나 매핑하지 않고 **소스맵**(`hw_sourcemap.json`)으로 관리한다.

- **생성**: `tool/` (entryjs-sourcemap Node CLI) — `node tool/bin/entryjs-sourcemap.js --src <entryjs> --out hw_sourcemap.json`. 201 장치 / 5,531 블럭, 로드 실패 0건. (로더 보강: case-insensitive 상대 require, entryModuleLoader 스텁, 공유 base 상태 오염 해소)
- **검증**: `entryc hw --sourcemap hw_sourcemap.json` — 장치/블럭 수 리포트 + Tier-0 스키마 검증(`validate_hw_sourcemap`). 위반 시 nonzero exit.
- **내장**: `hw_sourcemap.json` 을 entryc 바이너리에 `include_str!` 로 내장. `entryc build`/`extract` 는 ① `--hw` 지정 경로 ② cwd 의 `hw_sourcemap.json` ③ **내장 소스맵** 순으로 로드 (파일/옵션 없이도 항상 동작).
- **정방향 (.rs→.ent)**: Rust 에서 하드웨어 블럭 id 를 함수명으로 호출 → `pyocoding_serial_set("COM1")` → `Block::Raw` → .ent 하드웨어 블럭 생성 (중첩 getter 포함).
- **역방향 (.ent→.rs)**: `.ent` 하드웨어 블럭 → `pyocoding_serial_set("COM1")` Rust 호출로 복원 (기존 raw JSON 코멘트 폴백 아님).
- **손실 없는 라운드트립**: `.ent` 하드웨어 블럭의 원본 params/statements JSON 을 `// @hwraw {json}` 주석으로 .rs 에 보존 → 재빌드 시 원본 .ent 블럭 정확히 재생성.
- **핵심 구현**: `Block::Raw { type_id, raw }` 동적 variant, `ir::FuncRef.raw`(raw 운반), registry 전역 하드웨어 인덱스(`set_hw_index`/`is_hw_block`), deparse(`block_from_value` 소스맵 가드) / decodegen(raw post-order 누적·@hwraw emit) / parse(@hwraw 큐 복구).
- **제약 (하드웨어 무관 기존 DSL 갭)**: `v = getter()` (프로젝트 변수에 함수 호출 결과 할당) 는 `convert_expr` 이 `Expr::Assign` 을 처리하지 않아 파싱 실패 — 일반 함수도 동일. 하드웨어 getter 는 `let v = getter()` 형태로 정상.

## 완료된 모듈

- `parse` — `syn::File` → `ir::Program`
  - `syn::Item::Fn` → `when_start` / `when_click` / `when_*` 시점은 본문 평탄화, `fn main()` 등은 `FuncDef`
  - `Stmt::VarDecl`, `SetVar`, `FuncDef`, `If`, `While`, `Repeat`, `For`, `Return`, `Break`, `Continue`
  - `Expr::Lit` (Int/Float/Str/Bool), `Binary` (12개), `Unary`, `Path`, `Call`, `Paren`, `Range`
- `block` — `ir` → `Block` enum + `ParamBlock`
  - 시작 (트리거): `WhenStart`/`WhenClick`/`WhenCloneStart`/`WhenMessageRecv`/`WhenKeyPressed`/`WhenMouseClicked`/`WhenMouseReleased`/`WhenObjectReleased`/`WhenSceneStart`
  - 시작 (액션): `MessageCast`/`MessageCastWait`/`StartScene`/`StartNeighborScene`
  - 변수: `SetVar`/`ChangeVar`/`GetVar`/`ShowVar`/`HideVar`
  - 흐름: `If`/`IfElse`/`While`/`Repeat`/`Forever`/`Break`/`Continue`/`StopAll`
  - 산술: `CalcBinOp`/`Compare`/`BoolOp`/`UnaryOp`
  - 문자열: `StringConcat`/`StringIncludes`
  - 함수: `FuncCall`/`FuncDef`/`Return`
- `codegen` — `Block` → `serde_json::Value`
  - Entry 슬롯 형식 (`params` + `statements` 분리)
  - 변수 드롭다운 `{ id, name, variableType }` (kind별 분기)
  - BinOp → Entry 산술/비교 기호, 슬롯 `[lhs, op, rhs]` 정렬
  - `generate(program, original)` — 원본 project.json에 `scripts`/`variables` 패치
  - `collect_var_map` — `analyze_variables` 단일 순회 결과로 VarMap 빌드
    - 변수명/explicit kind/scope/리스트 문맥을 한 번에 분석
    - 리스트 전용 호출의 리스트 인자는 이름과 무관하게 `VarKind::List`로 추론
    - 기존 분리형 explicit/scopes/list-context collector 제거
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

## EntryJS 블록 매핑 현황 (사용자 작성 가능 334개 중, 내부용 16개 제외)

> **2026-08-13 갱신**: v3.md 도입 — AI(105) + 확장(42) 추가. 목표 187 → 334.

> 내부용 16개 제외: 내부용 시작 13개 (check_*, wildcard_*, register_score, positive_number, negative_number, show_prompt, check_goal_success, check_lecture_goal, check_object_property, check_block_execution, switch_scope, is_answer_submited, check_variable_by_name) + 함수 UI 3개 (functionAddButton, function_name, showFunctionPropsButton). 사용자가 블록 패널에서 쓰는 게 아니라 EntryJS 가 평가/검증 환경에서 자동 끼는 블록들이라 매핑 대상 아님.
> 
> 추가 제외: `when_clone_start` (흐름 카테고리에 중복 등재, 시작에서 이미 매핑) / `repeat_while_true` (Rust native `while` 로 커버, 별도 블록 필요 없음). 두 블록은 카탈로그에 나오지만 별도 매핑 작업 필요 없음.

✅ = 매핑됨. `deparse.rs::block_from_value` 의 매치 arm 기준.

### 시작 (13/13) ✅ 완료 — 내부용 13개 제외
- ✅ `when_run_button_click` / `when_run` → `WhenStart` (→ `fn when_start()`)
- ✅ `when_object_click` / `when_click` → `WhenClick` (→ `fn when_click()`)
- ✅ `when_clone_start` → `WhenCloneStart` (→ `fn when_clone_start()`)
- ✅ `when_message_cast` → `WhenMessageRecv` (→ `fn when_message_<msg>()`)
- ✅ `when_some_key_pressed` → `WhenKeyPressed` (→ `fn when_key_pressed(key: &str)`)
- ✅ `mouse_clicked` → `WhenMouseClicked` (→ `fn when_mouse_clicked()`)
- ✅ `mouse_click_cancled` → `WhenMouseReleased` (→ `fn when_mouse_released()`)
- ✅ `when_object_click_canceled` → `WhenObjectReleased` (→ `fn when_object_released()`)
- ✅ `when_scene_start` → `WhenSceneStart` (→ `fn when_scene_start()`)
- ✅ `message_cast` → `MessageCast` (→ `send_message("foo");`)
- ✅ `message_cast_wait` → `MessageCastWait` (→ `wait_message("foo");`)
- ✅ `start_scene` → `StartScene` (→ `start_scene("scene2");`)
- ✅ `start_neighbor_scene` → `StartNeighborScene` (→ `start_next_scene();` / `start_prev_scene();`)
- 제외 (내부용, 매핑 대상 아님): `check_object_property`, `check_block_execution`, `switch_scope`, `is_answer_submited`, `check_lecture_goal`, `check_variable_by_name`, `show_prompt`, `check_goal_success`, `positive_number`, `negative_number`, `wildcard_string`, `wildcard_boolean`, `register_score`

### 흐름 (14/14) ✅ 완료 — when_clone_start 중복 외 repeat_while_true 미적용
- ✅ `repeat_basic` → `Repeat` (for-range 펼침)
- ✅ `repeat_while` → `While`
- ✅ `repeat_inf` / `repeat_forever` → `Forever`
- ✅ `_if` / `if` → `If`
- ✅ `if_else` → `IfElse`
- ✅ `stop_repeat` / `stop_object` → `Break`
- ✅ `continue_repeat` / `_continue` → `Continue`
- ✅ `stop_object` (전체 정지 의미) / `stop_run_all` → `StopAll`
- ✅ `wait_second` — □ 초 기다리기 (→ `wait_second(secs)`)
- ✅ `wait_until_true` — □ 이(가) 될 때까지 기다리기 (→ `wait_until_true(cond)`)
- ✅ `restart_project` — 처음부터 다시 실행하기 (→ `restart_project()`)
- 제외 (별도 블록 필요 없음): `repeat_while_true` (Rust native `while` 키워드로 커버), `when_clone_start` (시작 카테고리에 이미 매핑)
- ✅ `create_clone` — □ 의 복제본 만들기 (→ `create_clone()` 로 self, `create_clone("sprite_name")` 로 다른 sprite. `&self` 도 허용)
- ✅ `delete_clone` — 이 복제본 삭제하기 (→ `delete_clone()`)
- ✅ `remove_all_clones` — 모든 복제본 삭제하기 (→ `remove_all_clones()`)

### 움직임 (19/19) ✅ 완료
- ✅ `locate_xy_time` — □ 초 동안 x:□ y:□ 위치로 이동하기 (→ `locate_xy_time(1.0, 100.0, -50.0)`)
- ✅ `move_direction` — 이동 방향으로 □ 만큼 움직이기 (→ `move_direction("forward", 10.0)`)
- ✅ `bounce_wall` — 화면 끝에 닿으면 튕기기 (→ `bounce_wall()`)
- ✅ `move_x` / `move_y` — x/y 좌표를 □ 만큼 바꾸기 (→ `move_x(10.0)` / `move_y(5.0)`)
- ✅ `rotate_relative` / `direction_relative` — 방향/이동방향을 □ 만큼 회전하기 (→ `rotate_relative(45.0)` / `direction_relative(90.0)`)
- ✅ `move_xy_time` — □ 초 동안 x:□ y:□ 만큼 움직이기 (→ `move_xy_time(1.0, 10.0, 5.0)`)
- ✅ `locate_x` / `locate_y` / `locate_xy` — x/y/x,y 위치로 이동하기 (→ `locate_x(100.0)` / `locate_y(-50.0)` / `locate_xy(100.0, -50.0)`)
- ✅ `locate` — □ 위치로 이동하기 (→ `locate("mouse")` / `locate("Sprite1")`)
- ✅ `locate_object_time` — □ 초 동안 □ 위치로 이동하기 (→ `locate_object_time(1.0, "mouse")` / `locate_object_time(2.0, "Sprite1")`)
- ✅ `rotate_by_time` / `direction_relative_duration` — □ 초 동안 회전 (→ `rotate_by_time(1.0, 45.0)` / `direction_relative_duration(1.0, 90.0)`)
- ✅ `rotate_absolute` / `direction_absolute` — 방향/이동방향을 □ (으)로 정하기 (→ `rotate_absolute(90.0)` / `direction_absolute(45.0)`)
- ✅ `see_angle_object` — □ 쪽 바라보기 (→ `see_angle_object("mouse")` / `see_angle_object("Sprite1")`)
- ✅ `move_to_angle` — □ 방향으로 □ 만큼 움직이기 (→ `move_to_angle(45.0, 10.0)`)

### 형태 (17/17) ✅ 완료
- ✅ `show` — 모양 보이기 (→ `show()`)
- ✅ `hide` — 모양 숨기기 (→ `hide()`)
- ✅ `dialog` — □ 을(를) □ □ (말하기) (→ `say(text)`)
- ✅ `dialog` — □ 을(를) □ □ (생각하기) (→ `think(text)`) — 같은 블록, params[1] = "think"
- ✅ `dialog_time` — □ 을(를) □ 초 동안 □ □ (시간 말하기) (→ `say(text, secs)` / `think(text, secs)`)
- ✅ `remove_dialog` — 말풍선 지우기 (→ `remove_dialog()`)
- ✅ `change_to_some_shape` — □ 모양으로 바꾸기 (→ `change_to_some_shape("walk")`)
- ✅ `change_to_next_shape` — 다음/이전 모양으로 바꾸기 (→ `change_to_next_shape()`)
- ✅ `add_effect_amount` — □ 효과를 □ 만큼 주기 (→ `add_effect_amount("color", 50.0)`)
- ✅ `change_effect_amount` — □ 효과를 □ (으)로 정하기 (→ `change_effect_amount("color", 50.0)`)
- ✅ `erase_all_effects` — 효과 모두 지우기 (→ `erase_all_effects()`)
- ✅ `change_scale_size` — 크기를 □ 만큼 바꾸기 (→ `change_scale_size(50.0)`)
- ✅ `set_scale_size` — 크기를 □ (으)로 정하기 (→ `set_scale_size(100.0)`)
- ✅ `stretch_scale_size` — □ 를 □ 만큼 늘이기 (→ `stretch_scale_size("width", 10)` / `("height", 10)`)
- ✅ `reset_scale_size` — 원래 크기로 되돌리기 (→ `reset_scale_size()`)
- ✅ `flip_x` — 좌우 모양 뒤집기 (→ `flip_x()`)
- ✅ `flip_y` — 상하 모양 뒤집기 (→ `flip_y()`)

> EntryJS 의 `flip_x` 는 setScaleY 부호 반전 (좌우), `flip_y` 는 setScaleX 부호 반전 (상하). EntryJS 변수명과 동작이 반대 — EntryJS 호환을 위해 그대로 매핑.
- ✅ `change_object_index` — □ 보내기 (레이어) (→ `change_object_index("front")` / `change_object_index("back")`)

### 붓 (13/13) ✅ 완료
- ✅ `brush_stamp` — 도장 찍기 (→ `brush_stamp()`)
- ✅ `start_drawing` / `stop_drawing` — 그리기 시작/멈추기 (→ `start_drawing()` / `stop_drawing()`)
- ✅ `start_fill` / `stop_fill` — 채우기 시작/멈추기 (→ `start_fill()` / `stop_fill()`)
- ✅ `set_color` — 색 정하기 (→ `set_color(50.0, 100.0, 0.0)`)
- ✅ `set_random_color` — 색을 랜덤으로 정하기 (→ `set_random_color()`)
- ✅ `set_fill_color` — 채우기 색을 □ 로 정하기 (→ `set_fill_color("#FF0000")`)
- ✅ `change_thickness` / `set_thickness` — 그리기 굵기 (→ `change_thickness(5.0)` / `set_thickness(10.0)`)
- ✅ `change_brush_transparency` / `set_brush_tranparency` — 붓 투명도 (→ `change_brush_transparency(10.0)` / `set_brush_tranparency(50.0)`). **EntryJS 의 set_brush_tranparency 는 원본 오타 그대로**
- ✅ `brush_erase_all` — 모든 붓 지우기 (→ `brush_erase_all()`)

### 텍스트 (9/9) ✅ 완료
- ✅ `text_read` — 글상자 □ 의 내용 (→ `text_read("self")`)
- ✅ `text_write` — □ (이)라고 글쓰기 (→ `text_write("...")`; statement 전용 — 자기 textBox 에 작성, textBox 없는 sprite 는 런타임이 silent 로 무시)
- ✅ `text_append` — □ 라고 뒤에 이어쓰기 (→ `text_append("...")`; statement 전용, `text_write` 와 동일 시그니처 — params = `[TextInput, Null]`)
- ✅ `text_prepend` — □ 라고 앞에 추가하기 (→ `text_prepend("...")`; statement 전용, `text_write` 와 동일 시그니처 — params = `[TextInput, Null]`)
- ✅ `text_change_effect` — 텍스트에 효과 (→ `text_change_effect("strike", true)` 또는 `text_change_effect(TextEffect::Strike, true)`; statement 전용, `Block::TextChangeEffect { effect: TextEffect, mode: bool }`. Dropdown 슬롯 2개 + Indicator, params = `["strike"|"underLine"|"fontItalic"|"fontBold", "on"|"off", null]`. `TextEffect` enum (Strike/UnderLine/FontItalic/FontBlold) + `text_effect_to_str`/`str_to_text_effect` helper.)
- ✅ `text_flush` — 텍스트 모두 지우기 (→ `text_flush()`; statement 전용, no-arg. EntryJS `def: { params: [null] }` 가 `.ent` 에선 빈 배열로 emit → `Block::TextFlush` unit variant, params = `[]`.)
- ✅ `text_change_font` — 글씨체 변경 (→ `text_change_font("Nanum Gothic")`; 동적 드롭다운이므로 JSON 문자열 + Indicator 슬롯으로 emit)
- ✅ `text_change_font_color` — 글씨 색 변경 (→ `text_change_font_color("#112233")`; 색상 값 블록 + Indicator 슬롯)
- ✅ `text_change_bg_color` — 글상자 배경색 변경 (→ `text_change_bg_color("#445566")`; 색상 값 블록 + Indicator 슬롯)

### 소리 (16/16)
- ✅ `sound_something_with_block` — 소리 □ 재생하기
- ✅ `sound_something_second_with_block` — 소리 □ □ 초 재생하기
- ✅ `sound_from_to` — 소리 □ □ 초 부터 □ 초까지 재생하기
- ✅ `sound_something_wait_with_block` — 소리 □ 재생하고 기다리기
- ✅ `sound_something_second_wait_with_block` — 소리 □ □ 초 재생하고 기다리기
- ✅ `sound_from_to_and_wait` — 소리 □ □ 초 부터 □ 초까지 재생하고 기다리기
- ✅ `sound_volume_change` — 소리 크기만큼 바꾸기
- ✅ `sound_volume_set` — 소리 크기를 □로 정하기
- ✅ `get_sound_speed` — 소리 빠르기
- ✅ `sound_speed_change` — 소리 빠르기만큼 바꾸기
- ✅ `sound_speed_set` — 소리 빠르기를 □로 정하기
- ✅ `sound_silent_all` — □ 소리 멈추기
- ✅ `play_bgm` — 배경음악 재생하기
- ✅ `stop_bgm` — 배경음악 멈추기
- ✅ `get_sound_volume` — 소리 크기 값
- ✅ `get_sound_duration` — 소리 길이 값

### 판단 (12/12)
- ✅ `boolean_basic` → `Compare`
- ✅ `boolean_basic_operator` → `Compare`
- ✅ `boolean_and_or` → `BoolOp`
- ✅ `calc_unary` (boolean_not) → `UnaryOp`
- ✅ `is_clicked` — 클릭했는가? (→ `is_clicked()`)
- ✅ `is_object_clicked` — 오브젝트 클릭했는가? (→ `is_object_clicked()`)
- ✅ `is_press_some_key` — 키 눌렸는가? (→ `is_press_some_key("space")`)
- ✅ `reach_something` — □ 에 닿았는가? (→ `reach_something("enemy")` / `reach_something()` self)
- ✅ `is_type` — 타입 체크 (숫자/영문/한글)
- ✅ `is_boost_mode` — 부스트 모드인가? (→ `is_boost_mode()`; EntryJS 의 `Entry.options.useWebGL` 반환. EntryRS 듀얼엔진 CappucinoVM / OmochaEngine 에서 파라미터 폴백 용도)
- ✅ `is_touch_supported` — 터치 가능한가? (→ `is_touch_supported()`; 터치/마우스 UI 분기용)
- ✅ `is_current_device_type` — □ 에서 실행하는가?

### 연산 (15/26)
- ✅ `calc_basic` → `CalcBinOp` (사칙연산)
- ✅ `number` → `Number` 리터럴
- ✅ `text` → `Text` 리터럴
- ✅ `boolean` → `Boolean` 리터럴
- ✅ `angle` → `Angle` 리터럴 (각도 슬롯)
- ✅ `color` → `Color` 리터럴 (색상 슬롯)
- ✅ `calc_rand` — □ 부터 □ 사이의 무작위 수 (→ `calc_rand(min, max)`)
- ✅ `get_project_timer_value` — 타이머 값 (→ `get_project_timer_value()`)
- ✅ `choose_project_timer_action` — 타이머 동작
  - `start_timer()` — 시작
  - `stop_timer()` — 정지
  - `reset_timer()` — 리셋
- ✅ `set_visible_project_timer` — 타이머 표시
  - `show_timer()` — 보이기
  - `hide_timer()` — 숨기기
- ✅ `coordinate_mouse` — 마우스 x/y 좌표 (→ `coordinate_mouse("x"|"y")`; 값 블럭)
- ✅ `coordinate_object` — 오브젝트 속성값 (→ `coordinate_object("self"|"오브젝트이름", "x"|"y"|"rotation"|"direction"|"size"|"picture_index"|"picture_name")`; 값 블럭)
- ✅ `quotient_and_mod` — □ 를 □ 로 나눈 몫/나머지 (→ `quotient_and_mod(a, b, "quotient"|"modulo")`)
- ✅ `calc_operation` — 수학 함수
  - `abs(x)` — 절댓값
  - `sqrt(x)` — 제곱근
  - `sin(x)` — 사인
  - `cos(x)` — 코사인
  - `tan(x)` — 탄젠트
  - `asin(x)` — 아크사인
  - `acos(x)` — 아크코사인
  - `atan(x)` — 아크탄젠트
  - `ln(x)` — 자연로그
  - `log(x)` — 상용로그
  - `exp(x)` — 지수
  - `pow10(x)` — 10의 거듭제곱
- ✅ `get_date` — 날짜/시/분/초 (→ `get_date("year"|"month"|"day"|"hour"|"minute"|"second")`; 값 블럭. `DateKind` enum + `date_kind_to_str` / `str_to_date_kind` helper. from_stmt 에서 statement 자리 거부)
- ✅ `distance_something` — 두 점 사이 거리 (→ `distance_something("mouse")` 또는 `distance_something("Sprite1")`; 값 슬롯 블록. `Block::DistanceSomething { target: String }`. 문자열/변수 둘 다 허용 (target). EntryJS params = `[Text, DropdownDynamic, Text]` (spritesWithMouse 메뉴) → `[null, target, null]`. stmt 자리 거부. **EntryJS Runtime 이 `Entry.container.getEntity(id)` 로 lookup 하므로 dropdown 슬롯 값은 sprite id 여야 함** — `AssetMap::object_id_by_name` / `object_name_by_id` 으로 양방향 변환 (`mouse` / `self` 는 reserved keyword 라 그대로 통과). 7개 블록 (`CreateClone`, `SeeAngleObject`, `Locate`, `ReachSomeThing`, `LocateObjectTime`, `CoordinateObject`, `DistanceSomething`) 동시 적용. 정방향은 nested Sub block 까지 recursive 변환 (`resolve_nested_object_target`). **연산 16/26.**)
- ✅ `get_user_name` — 아이디 (→ `get_user_name()`; 값 슬롯 블록. `Block::GetUserName` unit variant. EntryJS `func` 는 `window.user.username` 또는 공백. stmt 자리 거부. 테스트 4개 (basic/roundtrip/arity_check/statement_error).)
- ✅ `get_nickname` — 닉네임 (→ `get_nickname()`; 값 슬롯 블록. `Block::GetNickName` unit variant. EntryJS `func` 는 `window.user.filename` 또는 공백. stmt 자리 거부. 테스트 4개.) **연산 18/26.**
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

### 변수 (19/19) ✅ 완료
- ✅ `set_variable` → `SetVar`
- ✅ `change_variable` → `ChangeVar`
- ✅ `get_variable` → `GetVar`
- ✅ `show_variable` / `hide_variable` → `ShowVar`/`HideVar`
- ✅ `ask_and_wait` — □ 을(를) 묻고 대답 기다리기 (→ `ask_and_wait("질문")`)
- ✅ `get_canvas_input_value` — 대답 값 (→ `get_canvas_input_value()`)
- ✅ `set_visible_answer` — 대답 보이기/숨기기 (→ `show_answer()` / `hide_answer()`)
- ✅ `value_of_index_from_list` → `ListValueAt` (→ `value_of_index_from_list(index, list)`)
- ✅ `add_value_to_list` → `AddValueToList` (→ `add_value_to_list(value, list)`)
- ✅ `remove_value_from_list` → `RemoveValueFromList` (→ `remove_value_from_list(index, list)`)
- ✅ `insert_value_to_list` — N번째에 삽입 (→ `insert_value_to_list(value, index, list)`)
- ✅ `change_value_list_index` — N번째 값 바꾸기 (→ `change_value_list_index(index, value, list)`)
- ✅ `length_of_list` — 리스트 길이 (→ `length_of_list(list)`)
- ✅ `is_included_in_list` — 포함 여부 (→ `is_included_in_list(list, value)`)
- ✅ `show_list` / `hide_list` — 리스트 보이기/숨기기 (→ `show_list(list)` / `hide_list(list)`)

### 함수 (7/11) — UI 버튼 3개 제외
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
- 제외 (UI 버튼/라벨, 실제 실행 블록 아님): `functionAddButton`, `function_name`, `showFunctionPropsButton`

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

## 아래는 추후 작업할 예정 너무 분량이 많아서 이건 나중에 할거 <br> 기본블록 만 있어도 컨텐츠 제작에 문제없음.
### 인공지능 (AI) 학습 (0/26) — 7 파일 공유 (cluster / decisiontree / knn / logistic_regression / regression / svm / learning)
| 블럭 ID | 블럭 이름 |
|---|---|
| get_predict_1 | □ □ 의 분류 결과 |
| get_predict_2 | □ □ □ □ 의 분류 결과 |
| get_predict_3 | □ □ □ □ □ □ 의 분류 결과 |
| get_predict_4 | □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| is_result_1 | □ □ 의 분류 결과가 □ 인가? |
| is_result_2 | □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_3 | □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_4 | □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| learning_title_image | □ |
| learning_title_speech | □ |
| learning_title_text | □ |
| insert_data_for_test | 학습한 모델로 분류하기 □ |
| video_capture_for_image_test | 비디오 화면을 학습한 모델로 분류 □ □ |
| insert_text_block_for_test | □ 을(를) 학습한 모델로 분류하기 □ |
| test_result | 분류 결과 |
| accuracy_of_result | □에 대한 신뢰도 |
| is_group | 분류 결과가 □ 인가? |
| retrain_model | 모델 다시 학습하기 □ |
| model_is_trained | 모델이 학습되었는가? |
| set_train_visible | 모델 □ □ |
| set_train_chart | 모델 차트 창 □ □ |
| get_result_info | 모델의 □ |

### 인공지능 (AI) 활용 (0/79) — 9 파일 누적 (audio / face_landmarker / gesture_recognition / media_pipe / object_detector / pose_landmarker / translate / tts / video)
| 블럭 ID | 블럭 이름 |
|---|---|
| audio_title | □ |
| check_microphone | 마이크가 연결되었는가? |
| get_microphone_volume | 마이크 소리 크기 |
| speech_to_text_title | □ |
| speech_to_text_convert | □ 음성 인식하기 □ |
| timed_speech_to_text_convert | □ 초 동안 □ 음성 인식하기 □ |
| set_visible_speech_to_text | 인식한 음성 □ □ |
| speech_to_text_get_value | 음성을 문자로 바꾼 값 |
| face_landmarker_title | □ |
| when_face_landmarker | □ 얼굴을 인식했을 때 |
| face_landmarker | 얼굴 인식 □ □ |
| draw_detected_face | 인식한 얼굴 □ □ |
| check_detected_face | 얼굴을 인식했는가? |
| count_detected_face | 인식한 얼굴의 수 |
| locate_to_face | □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| locate_time_to_face | □ 초 동안 □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| check_detected_gender | □ 번째 얼굴의 성별이 □ 인가? |
| check_compare_age | □ 번째 얼굴의 나이 □ □ 인가? |
| check_detected_emotion | □ 번째 얼굴의 감정이 □ 인가? |
| axis_detected_face | □ 번째 얼굴의 □ 의 □ 좌표 |
| get_detected_face_value | □ 번째 얼굴의 □ |
| hand_detection_title | □ |
| when_hand_detection | □ 손을 인식했을 때 |
| hand_detection | 손 인식 □ □ |
| draw_detected_hand | 인식한 손 □ □ |
| check_detected_hand | 손을 인식했는가? |
| count_detected_hand | 인식한 손의 수 |
| locate_to_hand | □ 번째 손의 □ □ (으)로 이동하기 □ |
| locate_time_to_hand | □ 초 동안 □ 번째 손의 □ □ (으)로 이동하기 □ |
| axis_detected_hand | □ 번째 손의 □ □ 의 □ 좌표 |
| is_which_hand | □ 번째 손이 □ 인가? |
| get_which_hand | □ 번째 손 |
| is_which_gesture | □ 번째 손의 모양이 □ 인가? |
| get_which_gesture | □ 번째 손의 모양 |
| media_pipe_title | □ |
| media_pipe_video_screen | 비디오 화면 □ □ |
| media_pipe_switch_camera | □ 카메라로 바꾸기 □ |
| check_connected_camera | 카메라가 연결되었는가? |
| media_pipe_flip_camera | 비디오 화면 □ 뒤집기 □ |
| media_pipe_set_opacity_camera | 비디오 투명도 효과를 □ 으로 정하기 □ |
| media_pipe_motion_value | □ 에서 감지한 □ 값 |
| object_detector_title | □ |
| when_object_detector | □ 사물을 인식했을 때 |
| object_detector | 사물 인식 □ □ |
| draw_detected_object | 인식한 사물 □ □ |
| check_detected_object | 사물을 인식했는가? |
| count_detected_object | 인식한 사물의 수 |
| is_detected_among_objects | 사물 중 □ 을(를) 인식했는가? |
| pose_landmarker_title | □ |
| when_pose_landmarker | □ 사람을 인식했을 때 |
| pose_landmarker | 사람 인식 □ □ |
| draw_detected_pose | 인식한 사람 □ □ |
| check_detected_pose | 사람을 인식했는가? |
| count_detected_pose | 인식한 사람의 수 |
| locate_to_pose | □ 번째의 사람의 □ (으)로 이동하기 □ |
| locate_time_to_pose | □ 초 동안 □ 번째의 사람의 □ (으)로 이동하기 □ |
| axis_detected_pose | □ 번째 사람의 □ 의 □ 좌표 |
| translate_title | □ |
| get_translated_string | □ □ 을(를) □(으)로 번역한 값 |
| check_language | □의 언어 |
| tts_title | □ |
| read_text | □ 읽어주기 □ |
| read_text_wait_with_block | □ 읽어주고 기다리기 □ |
| set_tts_property | □ 목소리를 □ 속도 □ 음높이로 설정하기 □ |
| video_title | □ |
| video_change_cam | □ 카메라로 바꾸기 □ |
| video_check_webcam | 비디오가 연결되었는가? |
| video_draw_webcam | 비디오 화면 □ □ |
| video_set_camera_opacity_option | 비디오 투명도 효과를 □ 으로 정하기 □ |
| video_flip_camera | 비디오 화면 □ 뒤집기 □ |
| video_toggle_model | □ 인식 □ □ |
| video_toggle_ind | 인식된 □ □ □ |
| video_number_detect | 인식된 □ 의 수 |
| video_object_detected | 사물 중 □ (이)가 인식되었는가? |
| video_is_model_loaded | □ 인식이 되었는가? |
| video_detected_face_info | □ 번째 얼굴의 □ |
| video_motion_value | □ 에서 감지한 □ 값 |
| video_face_part_coord | □ 번째 얼굴의 □ 의 □ 좌표 |
| video_body_part_coord | □ 번째 사람의 □ 의 □ 좌표 |

### 확장 (교과) (0/42) — 6 파일 누적 (behaviorconduct_disaster / behaviorconduct_lifesafety / disasterAlert / emergencyActionGuidelines / festival / weather)
| 블럭 ID | 블럭 이름 |
|---|---|
| behaviorConductDisaster_title | □ |
| count_disaster_behavior | □ □ 해야할 행동요령 수 |
| get_disaster_behavior | 자연재난□ □ 해야할 행동요령 □ 번째 항목 |
| behaviorConductLifeSafety_title | □ |
| count_lifeSafety_behavior | □ 에서 □ 방법의 수 |
| get_lifeSafety_behavior | □ 에서 □ 방법 □ 번째 항목 |
| disaster_alert_title | (이름 없음) |
| count_disaster_alert | (이름 없음) |
| get_disaster_alert | (이름 없음) |
| check_disaster_alert | (이름 없음) |
| emergencyActionGuidelines_title | (이름 없음) |
| count_disaster_guideline | 자연재난 □ □ 의 행동요령 수 |
| get_disaster_guideline | 자연재난 □ □ 의 행동요령 □ 번째 항목 |
| count_social_disaster_guideline | 사회재난 □ □ 의 행동요령 수 |
| get_social_disaster_guideline | 사회재난 □ □ 의 행동요령 □ 번째 항목 |
| count_safety_accident_guideline | 생활안전 □ □ 의 행동요령 수 |
| get_safety_accident_guideline | 생활안전 □ □ 의 행동요령 □ 번째 항목 |
| festival_title | □ |
| count_festival | □ □ 행사의 수 |
| get_festival_info | □ □ 행사 □ 번째 항목의 □ |
| weather_title | □ |
| check_city_weather | □ □ □의 날씨가 □인가? |
| check_city_finedust | 현재 □ □ 의 미세먼지 등급이 □인가? |
| get_city_weather_data | □ □ □ 의 □ |
| get_current_city_weather_data | 현재 □ □ 의 □ |
| get_today_city_temperature | 오늘 □ □의 □시 기온 |
| check_weather | □ □ 의 날씨가 □인가? |
| check_finedust | 현재 □ 의 미세먼지 등급이 □인가? |
| get_weather_data | □ □ 의 □ |
| get_current_weather_data | 현재 □ 의 □ |
| get_today_temperature | 오늘 □의 □시 기온 |
| get_cur_weather | 현재 □의 날씨 |
| get_cur_wind | 현재 □의 풍향 |
| get_cur_weather_data | 현재 □의 □ |
| check_cur_weather | 현재 □의 날씨가 □인가? |
| check_cur_finddust | 현재 □의 미세먼지 등급이 □인가? |
| get_day_weather | □ □의 날씨 |
| get_day_weather_data | □ □의 □ |
| check_day_weather | □ □의 날씨가 □ 인가? |
| get_time_weather | □의 □시 날씨 |
| get_time_weather_data | □의 □시 □ |
| check_time_weather | □의 □시 날씨가 □ 인가? |

### 합계

**133/334** 매핑됨 (약 39.8%). 목표: 기본 187 + AI 학습 26 + AI 활용 79 + 확장 42 = 334개 (기본 203개 중 내부용 16개 제외).

카테고리별 (✅/전체): 시작 13/13 (완료, 내부용 13개 제외), 흐름 14/14 (완료), 움직임 19/19 (완료), 형태 17/17 (완료), 붓 13/13 (완료), 텍스트 9/9 (완료), 소리 16/16 (완료), 판단 12/12 (완료), 연산 18/26, 변수 19/19 (완료), 함수 7/11 (UI 3개 제외), 데이터분석 0/18, **AI 학습 0/26, AI 활용 0/79, 확장 0/42**.

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
- [x] `locate_xy_time` — □ 초 동안 x:□ y:□ 위치로 이동하기 (→ `locate_xy_time(1.0, 100.0, -50.0)`)
- [x] `locate` — □ 위치로 이동하기 (→ `locate("mouse")` / `locate("Sprite1")`)
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
  - [x] `deparse::value_to_param` 에 variable dropdown 분기 (`{id,name,variableType}` 형태의 슬롯을 `ParamBlock::Variable` 로 — `type` 키 없음)
  - [ ] 트리거 없는 thread 시 `when_run` 자동 prepend (현재 `parse` 가 Item::Fn 만 허용해 dead code, 방어용으로만 유지)
- [ ] **다음 우선순위 (블록 추가)**
  - [x] 시작 (트리거): `when_key_pressed`, `when_mouse_clicked`, `when_mouse_released`, `when_object_released`, `when_scene_start`
  - [x] 시작 (액션): `send_message`/`wait_message`, `start_scene`/`start_next_scene`/`start_prev_scene`
  - [x] 흐름: `wait_second`, `wait_until_true` (쉬움, 즉시 가치)
  - [x] 흐름: `repeat_while_true` 별칭 (Rust native `while` 키워드로 커버 — `f(args) { body }` 신택스는 syn 거부)
  - [x] 연산: `calc_rand` (난수), `get_project_timer_value` (타이머 값), `set_visible_project_timer` (타이머 보이기/숨기기)
    - 타이머 시작/정지/리셋 (`choose_project_timer_action`) ✅ `start_timer()` / `stop_timer()` / `reset_timer()`
  - [x] 변수: `ask_and_wait` (입력 묻기) / `get_canvas_input_value` (대답 값), `set_visible_answer` (대답 보이기/숨기기)
  - [x] 형태: `show` / `hide` (오브젝트 보이기/숨기기), `say`/`think` (말하기/생각하기), `say(text, secs)` / `think(text, secs)` (시간 말하기)
  - [x] 연산: `quotient_and_mod` (몫/나머지) → `quotient_and_mod(a, b, "quotient"|"modulo")`
  - [x] 연산: `calc_operation` (절댓값/제곱/제곱근) → `abs(x)` / `sqrt(x)` / `sin(x)` / ... (12개 함수)
  - [x] 형태: `stretch_scale_size` (□ 를 □ 만큼 늘이기) → `stretch_scale_size("width"|"height", v)`. EntryJS dropdown 값은 대문자 `WIDTH`/`HEIGHT` 라 emit 시 변환 (`dim_to_str`), DSL 신택스는 소문자 (`dim_to_dsl_str` / `str_to_dim`). **형태 카테고리 17/17 완료.**
  - [x] 변수 리스트 보강: `length_of_list` / `is_included_in_list` 매핑. EntryJS 의 text-label 자리 (params[0/2/4]) 는 `Value::Null` 로 emit. length_of_list 는 `[Text, list, Text]` → `[Null, list, Null]`, is_included_in_list 는 `[Text, list, Text, value, Text]` → `[Null, list, Null, value, Null]`. 값 슬롯 블록이라 `SetVar` 내부 value 자리에 그대로 매핑 가능. codegen `analyze_variables` 의 list_context_names 에 두 호출 모두 등록 (list 변수가 자동 `VarKind::List` 로 분류).
  - [x] 변수 리스트 가시성: `show_list` / `hide_list` 매핑. EntryJS 의 `[DropdownDynamic, Indicator]` 슬롯 자리에 `[list_variable_param, Null]` emit. from_stmt 에 reserved name 매칭 추가 (없으면 `FuncCall` fallback 으로 떨어짐). codegen list_context_names 두 호출 등록. **변수 카테고리 19/19 완료.**
  - [x] **리터럴 정합화 (EntryJS 호환)**:
    - [x] `Angle` 리터럴 (`Block::Angle(f64)` → EntryJS `angle` 타입 ID) — 각도 슬롯
    - [x] `Color` 리터럴 (`Block::Color(String)` → EntryJS `color` 타입 ID) — 색상 슬롯
    - [x] `function_field_label.params[0]` = `{type:"TextInput", value:name}` 객체로 변경 (EntryJS `script.getField('NAME')` 경로, raw string 직접 박으면 필드 lookup 실패)
    - [x] 같은 이름 + 다른 arity 함수 정의 → 호출 사이트가 `args.len()` 으로 매칭되어 각각 정확한 `func_<id>` 로 라우팅 (정확 매칭 우선, 실패 시 가장 가까운 arity fallback)
  - [x] 텍스트: `text_write` (□ (이)라고 글쓰기) — statement 전용, `Block::TextWrite { content: ParamBlock }` + params = `[TextInput, Null]` emit. textBox 없는 sprite 는 EntryJS 런타임이 silent 무시. **텍스트 2/9.**
  - [x] 텍스트: `text_append` (□ 라고 뒤에 이어쓰기) / `text_prepend` (□ 라고 앞에 추가하기) — `Block::TextAppend { content }` / `Block::TextPrepend { content }` 분리 variant. text_write 와 동일 시그니처 (params = `[TextInput, Null]`). reserved name 매칭 `text_append("...")` / `text_prepend("...")`. 테스트 8개 (basic/roundtrip/sub_expr/arity_check 각 4). **텍스트 4/9.**
  - [x] 텍스트: `text_change_effect` (텍스트에 효과) — `Block::TextChangeEffect { effect: TextEffect, mode: bool }`. `TextEffect` enum (Strike/UnderLine/FontItalic/FontBlold) + `text_effect_to_str`/`str_to_text_effect` helper. `text_change_effect("strike", true)` 및 `text_change_effect(TextEffect::Strike, true)` 신택스 (effect=string 또는 TextEffect variant, mode=bool). params = `["strike", "on", null]` (Dropdown 슬롯 2개 + Indicator). 문자열/enum 공통 dropdown 변환 규약으로 `EffectType`, `Dimension`, `QamMethod`에도 동일하게 적용. deparse 라운드트립에서 mode string ("on"/"off") ↔ bool 변환. 테스트 7개 (basic/enum/mixed/all_enum/roundtrip/arity_check/type_check). **텍스트 5/9.**
  - [x] 텍스트: `text_flush` (텍스트 모두 지우기) — no-arg statement. `Block::TextFlush` unit variant. EntryJS `def: { params: [null] }` 가 `.ent` 에선 빈 배열로 emit → params = `[]`. deparse 라운드트립에서 `Call(text_flush, [])` 복원. 테스트 3개 (basic/roundtrip/arity_check). **텍스트 6/9.**
  - [x] 텍스트: `text_change_font` / `text_change_font_color` / `text_change_bg_color` — 글씨체는 동적 드롭다운 문자열, 글씨 색·배경색은 색상 값 블록으로 emit. 세 블록의 기본 변환·라운드트립·인자 검증 테스트 추가. `text_change_effect` 비활성 값의 deparse 오타 (`"of"` → `"off"`) 수정. **텍스트 9/9 완료.**
  - [x] 판단: `is_boost_mode` (no-arg, EntryJS 의 `Entry.options.useWebGL`. EntryRS 듀얼엔진 CappucinoVM / OmochaEngine 폴백 용도)
  - [x] 판단: `is_touch_supported` (no-arg, 터치/마우스 UI 분기)
  - [x] 연산: `get_date` — `get_date("year"|"month"|"day"|"hour"|"minute"|"second")` (값 블럭). `DateKind` enum + `date_kind_to_str` / `str_to_date_kind`. params = `[null, kind, null]`. from_stmt 에서 statement 자리 거부. 테스트 4개 (basic/roundtrip/arity_check/statement_error). **연산 15/26.**
  - [x] 연산: `distance_something` (두 점 사이 거리) — 값 슬롯 블록. `Block::DistanceSomething { target: String }`. `distance_something("mouse")` 또는 `distance_something("Sprite1")` 신택스 (target = string literal or variable). EntryJS params `[Text, DropdownDynamic, Text]` (spritesWithMouse 메뉴) → emit `[null, target, null]`. deparse stmt 자리 거부, expr side 에서 `Call(distance_something, [Str(target)])` emit. 테스트 3개 (basic/roundtrip/arity_check). **연산 16/26.**
  - [x] 연산: `get_user_name` (아이디) / `get_nickname` (닉네임) — 값 슬롯 블록. `Block::GetUserName` / `Block::GetNickName` unit variant. no-arg, params = `[]`. EntryJS `func` 는 `window.user.username` / `window.user.filename`. from_stmt 에서 stmt 자리 거부. 테스트 8개 (각 블록당 basic/roundtrip/arity_check/statement_error). **연산 18/26.**
  - [x] 자산 ID 양방향 매핑 — 오브젝트 dropdown 슬롯 (spritesWithMouse / spritesWithSelf / objectWithSelf / collision) 값은 EntryJS Runtime 이 `Entry.container.getEntity(id)` 로 lookup 하므로 sprite id 가 필수. `AssetMap` 에 `object_ids: NameIdMap` 추가 + `object_id_by_name` / `object_name_by_id` 메서드 (`mouse` / `self` reserved keyword 그대로 통과). 7개 블록 (`CreateClone`, `SeeAngleObject`, `Locate`, `ReachSomeThing`, `LocateObjectTime`, `CoordinateObject`, `DistanceSomething`) 동시 적용. 정방향은 `to_value_with_assets` 의 nested recursive (`resolve_nested_object_target`) + `resolve_expr` 매칭. stmt-side object 매칭 arm 도 `reach_something` 추가 (statement 자리 호출 가능). 테스트: AssetMap 단위 4개 + 통합 라운드트립 3개 (`distance_something` name/id, `distance_something` mouse, `reach_something` name/id).
  - [ ] **잠재 문제**: `coordinate_mouse` / `coordinate_object` / `get_date` 의 category 가 현재 `Judgment` 인데 EntryJS source 는 모두 `block_calc.js` 안에 있음 → `Calc` 로 수정 필요. `get_date` 만 Calc 로 수정함, 나머지 2개 미정.
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

**현재 작업 디렉토리**: `D:\source\rust2entry` (Windows / PowerShell 5.1)

**현재 working tree 상태**: clean (모든 변경 커밋됨)

**마지막 커밋들**:
- `85e2af4 feat: add coordinate object value block`
- `fa7c08c fix: align multi-parameter opcode slots`
- `25a0c9f docs: document opcode audit details`
- `b0d9175 fix: align opcode parameter slots`

**빌드/테스트 명령**:
```
cargo test                  # 전체 (entryc build 6 + codegen 9 + compile 302 + parse 26 + 회귀 3 = 351 통과)
cargo test -p entrycore     # entrycore 만
cargo test -p entryc        # entryc 만
cargo build                 # 빌드만
```

> `cargo test` 출력 중 `error: the following required arguments were not provided: --rs <FILE>` 는 CLI 인자 누락을 검증하는 테스트의 정상 stderr 임. 실패 아님.

**샘플 .ent 위치**: `C:\Users\NEKO\Documents\test.ent` (EntryJS 실제 export 형식 참고용, 이 컴퓨터엔 없을 수 있음 — GitHub entryjs 코드 직접 참고)

**알려진 정리 대상**:
- `entrycore/src/block/mod.rs:9` `use std::clone;` — 미사용 import 경고
- `block::Dimension` (Width/Height) vs `codegen/schema.rs::Dimension` (picture width/height i64) — 이름 겹침. 현재는 모듈이 달라 컴파일 되지만 codegen 에서 둘 다 쓰면 alias 강제됨. `ScaleAxis` 로 rename 권장 (참조 ~7곳)

**다음 할 일 추천 순서**:
1. 연산 — 남은 값 블록을 우선 매핑

## 디렉토리

```
entrycore/   라이브러리 (parse/block/codegen/deparse/decodegen/var) + lib::compile_with_options
             - parse::parse_with_triggers: TriggerDef 분리 (build 전용)
             - CompileOptions: default_scene, replace_variables
             - ir::VarScope: Local (let) / Global (static)
             - ir::ParamKind: String (StringParam) / Bool (BoolParam)
entryc/      CLI (extract/build subcommand, --rs/--out/--ent-template, --scene, --replace-vars)
target/      빌드 산출물
entryjs-basic-blocks-v2.md  EntryJS 블럭 카탈로그 (187개 사용자용 중, 80개 매핑; 원본 203개 중 내부용 16개 제외)
AGENT.md     이 문서
```

## 최근 옵코드 파라미터 감사

EntryJS 원본 블럭 정의와 Rust의 `Block` 직렬화(`build_params_and_statements`), 역변환(`block_from_value`), IR 변환(`from_block_owned`/`expr_from_block`)을 서로 대조함. 파라미터 값의 의미뿐 아니라 `.ent` JSON 배열의 위치와 IR 함수의 인자 개수까지 확인함.

### `reach_something`

- EntryJS 정의의 실제 슬롯은 `[Indicator, DropdownDynamic, Indicator]`임.
- 충돌 대상은 `params[1]`에 저장되며, 앞뒤 슬롯은 화면 표시용 indicator임.
- 기존 코드는 `[target, null]`을 생성하고 `params[0]`을 읽어서 EntryJS와 위치가 어긋났음.
- 현재는 `[null, target, null]`을 생성하고 역변환도 `params[1]`을 읽음.
- 기존에 이미 생성된 `[target, null]` 형식은 `params[0]` fallback으로 읽어서 하위 호환함.
- 인자를 생략한 `reach_something()`은 기존 동작대로 `self`를 사용함.

### `is_boost_mode`

- EntryJS의 `is_boost_mode`는 인자를 받지 않는 값 블럭임.
- `from_block_owned`의 IR 메타데이터가 `arity: 1`로 잘못 기록되어 실제 호출 인자 수 `0`과 불일치했음.
- 현재 `arity: 0`으로 수정해 `expr_from_block`과 일치시킴.

### 소리 대기 블럭 역변환

- `SoundSomethingWaitWithBlock`은 `sound_something_wait_with_block(sound)`으로 변환되어야 함.
- `SoundSomethingSecondWaitWithBlock`은 `sound_something_second_wait_with_block(sound, seconds)`로 변환되어야 함.
- 기존 `expr_from_block`에서 두 함수명이 서로 뒤바뀌어 있었음.
- 현재 블럭 이름, 함수 이름, 인자 개수가 모두 일치함.

### 검증 결과

- `reach_something` 대상·기본값·라운드트립 테스트 3개 통과
- 전체 `cargo test -- --test-threads=1` 통과
- EntryJS 스키마 검증 테스트 4개 통과
- `git diff --check` 통과
- `coordinate_mouse`는 값 블럭으로 구현 완료했으며, `coordinate_object`는 별도 구현 대상으로 남김.

## 다음 모델 전달 작업: `from_block_owned` 값 블럭 정리

`from_block_owned`는 실행형 statement를 IR statement로 바꾸는 함수임. 값만 반환하는 블럭을 단독 statement로 변환하면 EntryJS에서 의미 없는 실행이 만들어지므로, 아래 값 블럭들은 statement 위치에서 오류를 반환하도록 정리해야 함.

### 정리 대상

- 변수·리스트 값: `GetVar`, `ListValueAt`, `LengthOfList`, `IsIncludedInList`
- 입력·타이머 값: `GetCanvasInputValue`, `GetProjectTimerValue`
- 계산 값: `CalcRand`, `CalcBinOp`, `Compare`, `BoolOp`, `UnaryOp`, `CalcOperation`, `QuotientAndMod`
- 판단 값: `IsClicked`, `IsObjectClicked`, `IsPressSomeKey`, `ReachSomeThing`, `IsType`, `IsBoostMode`, `IsTouchSupported`, `IsCurrentDeviceType`
- 소리 값: `GetSoundSpeed`, `GetSoundVolume`, `GetSoundDuration`
- 리터럴·문자열 값: `Number`, `Text`, `Boolean`, `Angle`, `Color`, `StringConcat`, `StringIncludes`
- `CoordinateMouse`

### 구현 정책

1. 값 블럭은 `from_expr`/`expr_from_block`에서만 허용함.
2. `from_block_owned`에는 exhaustive match용 오류 분기를 둠.
3. 오류 메시지는 `value block cannot be used as a statement` 의미를 포함함.
4. 기존에 값 블럭을 단독 statement로 허용하던 테스트는 성공 컴파일 기대를 제거하고 오류 검증으로 변경함.
5. 값 블럭이 `let`, 다른 함수 인자, 조건식 안에 들어가는 기존 테스트는 계속 통과해야 함.
6. 각 대상 블럭의 `.ent` 파라미터 슬롯과 역변환 라운드트립은 변경하지 않음.

### 완료 조건

- [ ] 위 목록의 모든 값 블럭에 statement 오류 분기 추가
- [ ] 값 컨텍스트 컴파일·역변환 테스트 유지
- [ ] 단독 statement 오류 테스트 추가
- [ ] `cargo test -- --test-threads=1` 전체 통과
- [ ] `git diff --check` 통과
