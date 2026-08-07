# AGENT.md

AI/에이전트 협업용 진행 문서. Readme와 동기화.

## 진행 상태

| # | 단계 | 상태 | 테스트 |
|---|------|------|--------|
| 1 | `parse` — Rust 소스 → IR Program | ✅ | 15/15 |
| 2 | `block` — IR → Block enum | ✅ | (in 3) |
| 3 | `codegen` — Block → project.json (패치) | ✅ | 9/9 |
| 4 | `deparse` — project.json → IR (역방향) | ✅ | (in 3 라운드트립) |
| 5 | `decodegen` — IR → DSL (Rust-like) | ✅ | (in 1 라운드트립) |
| 6 | `var` — VarInfo / VarMap | ✅ | - |
| 7 | `for-range` — `for i in a..b` → `repeat_basic` 펼침 | ✅ | in 1, 3 |
| 8 | 변수 kind (Timer/Answer/List) 인식 | ✅ | in 3 |
| 9 | `entryc extract` — `.ent` → `.rs` | ✅ | - |
| 10 | `entryc build` — `.rs` → `.ent` | ✅ | 5/5 |
| 11 | `lib::compile` — 전체 조립 | ✅ | 12/12 |

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

## 남은 작업 (TODO)

- [x] `for-range` IR → Entry 풀어쓰기 (`for i in a..b` → `repeat_basic(b-a)`)
- [x] 변수 kind (Timer/Answer/List) 자동 인식 + 전용 변수 거부
- [x] `generate(program, original)` project.json 패치
- [x] 라운드트립 테스트 (codegen/deparse, parse/decodegen)
- [x] `entryc build` — `.rs` → `.ent` 빌드 모드 (subcommand, --rs/--out/--ent-template)
- [x] `lib::compile` — 전체 조립 + extract 라운드트립용 가짜 오브젝트 패치
- [x] extract 출력 개선 — raw JSON 들여쓰기 + 에러 메시지 다단계 코멘트 + 미매핑 블록 집계 출력
- [x] 매핑 추가 — `when_run`, `when_object_click`, `number` (String 숫자 허용)
- [ ] 다른 흐름 블럭 (`if_else`, `wait_second`, `repeat_while_true`, `repeat_inf`, `wait_until_true`)
- [ ] Timer/Answer 전용 블록 신택스 (`start_timer()` 등)
- [ ] 이미지 차원 자동 측정 (스프라이트 PNG → width/height)
- [ ] `entities.default` (위치/크기) 처리
- [ ] Entry scripts 오브젝트별 분배 (extract 진짜 라운드트립)
- [ ] 실제 EntryJS import 테스트 (실행 환경 검증)

## 디렉토리

```
entrycore/   라이브러리 (parse/block/codegen/deparse/decodegen/var) + lib::compile
entryc/      CLI (extract/build subcommand, --rs/--out/--ent-template)
target/      빌드 산출물
entryjs-basic-blocks.md  EntryJS 블럭 카탈로그 (203개)
```
