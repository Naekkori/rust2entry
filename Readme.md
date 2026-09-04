# Rust2Entry

Rust 소스를 Entry 프로젝트 파일(`.ent`)로 변환하고, Entry 프로젝트 파일을
Rust 소스로 추출하는 양방향 트랜스파일러입니다.

`entryc` 단일 바이너리로 두 방향을 모두 처리합니다. 러스트 툴체인에 의존하지
않으며, 외부 도구 없이 `.rs` ↔ `.ent` 변환을 수행합니다.

## 프로젝트 구조

```
rust2entry/
├── entrycore/      Rust <-> Entry IR 변환 라이브러리 (syn/proc-macro2 기반 파서)
├── entryc/         CLI 바이너리 (build / extract 서브커맨드)
├── entrycgui/      GUI 프론트엔드
├── test_input/     라운드트립 테스트용 샘플 (.rs, .ent, project.json)
├── Cargo.toml      워크스페이스 매니페스트
├── LICENSE         MIT
└── NOTICE          서드파티 라이선스 고지
```

각 크레이트 책임:

- `entrycore` — `.rs` 파싱, IR 변환, Entry 블록 매핑, 역변환
- `entryc` — `clap` 기반 CLI, zip 압축/해제, 프로젝트 입출력
- `entrycgui` — `entryc`를 감싼 GUI

## 기능

### 빌드 (`entryc build`)

- 하나 이상의 `.rs` 파일을 `.ent`로 변환
- 기존 `.ent` 템플릿을 베이스로 사용 가능
- Rust 파일 stem과 Entry 오브젝트 `name`을 대소문자 무시 매칭
- 매칭 실패 시 새 sprite 오브젝트로 추가
- Rust `static` 변수를 Entry 프로젝트 변수로 생성
- 템플릿 변수와 Rust 변수를 ID 기준으로 병합
- Entry 블록으로 표현 불가능한 IR은 빌드 경고로 표시

### 추출 (`entryc extract`)

- `.ent`의 오브젝트별 스크립트를 개별 `.rs` 파일로 추출
- 변환 불가능한 블록은 raw JSON 주석으로 보존 (라운드트립 안전성)
- 기본 출력 폴더는 `.ent` 파일 위치의 프로젝트 이름 폴더
- 생성된 파일에는 `entryc` 생성기 헤더 포함

## 한계

- 지원 Rust 문법은 구현 범위에 한정됩니다. 미지원 문법은 변환되지 않거나
  빌드 경고로 표시됩니다.
- `const`와 미지원 top-level item은 변환 대상에서 제외됩니다.
- Entry 블록으로 표현 불가능한 일부 IR은 경고와 함께 변환에서 제외됩니다.
- 역변환은 정보 손실 가능성을 전제로 합니다 (`raw JSON 주석`으로 보존).

## 빌드

```text
cargo build --workspace
```

릴리스 바이너리:

```text
cargo build --release --workspace
```

## 사용법

### 1. Rust -> Entry 프로젝트 빌드

```text
cargo run --bin entryc -- build --rs sample.rs --out sample.ent
```

여러 Rust 파일을 입력할 수 있습니다.

```text
cargo run --bin entryc -- build --rs player.rs --rs enemy.rs --out game.ent
```

템플릿과 옵션을 함께 사용할 수 있습니다.

```text
cargo run --bin entryc -- build --rs player.rs --ent-template base.ent --out game.ent
cargo run --bin entryc -- build --rs player.rs --out game.ent --scene scene-id
cargo run --bin entryc -- build --rs player.rs --out game.ent --replace-variables
```

옵션:

| 옵션 | 설명 |
|---|---|
| `--rs FILE` | 입력 Rust 파일. 한 번 이상 지정해야 하며 반복할 수 있습니다. |
| `--ent-template FILE` | 기존 Entry 프로젝트를 베이스로 사용합니다. 생략하면 빈 프로젝트에서 시작합니다. |
| `--out FILE` | 출력 `.ent` 경로입니다. |
| `--scene ID` | 새 오브젝트에 적용할 scene ID입니다. 생략하면 템플릿의 첫 sprite scene ID를 사용합니다. |
| `--replace-variables` | 템플릿 변수를 유지하지 않고 Rust에서 생성한 변수로 교체합니다. 기본값은 ID 기준 병합입니다. |

### 2. Entry 프로젝트 -> Rust 추출

```text
cargo run --bin entryc -- extract --ent sample.ent
```

출력 폴더를 지정할 수 있습니다.

```text
cargo run --bin entryc -- extract --ent sample.ent --out extracted
```

기본 출력 폴더는 `.ent` 파일 위치의 프로젝트 이름 폴더입니다. 추출된 Rust
파일에는 `entryc` 생성기 헤더가 포함됩니다.

### 3. 동작 예시

`test_input/test_rect.rs`로 빌드:

```text
cargo run --bin entryc -- build --rs test_input/test_rect.rs --out test_input/test_one.ent
```

`test_input/test.ent`를 Rust 파일들로 추출:

```text
cargo run --bin entryc -- extract --ent test_input/test.ent --out test_input/out
```

## Rust 입력 규칙

- `when_` 접두 함수는 Entry 트리거로 변환됩니다.
- 그 외 함수는 Entry 함수로 변환됩니다.
- top-level `static`은 전역 변수로 변환됩니다.
- `const`와 지원되지 않는 top-level item은 변환되지 않습니다.
- 변환할 수 없는 Rust 구문은 오류가 될 수 있습니다.
- Entry 블록으로 표현할 수 없는 일부 IR은 경고와 함께 변환에서 제외됩니다.

## 오브젝트 매칭

`build`는 Rust 파일 stem과 Entry 오브젝트 `name`을 비교합니다.

- 이름이 일치하면 기존 오브젝트의 `script`를 갱신합니다.
- 일치하지 않으면 새 sprite 오브젝트를 추가합니다.
- 기존 sprite 메타데이터가 있으면 새 오브젝트가 이를 복사합니다.
- `project.scripts`는 템플릿 값을 유지합니다.
- 실제 오브젝트 스크립트는 각 오브젝트의 `script` 필드에 저장됩니다.

## 변수

- 기본 동작은 템플릿 변수와 Rust 변수를 ID 기준으로 병합합니다.
- 같은 ID의 템플릿 변수는 Rust 변수로 교체됩니다.
- `--replace-variables` 사용 시 템플릿 변수는 제거됩니다.
- `static` 변수는 전역 변수로 생성됩니다.
- `CloudVar`, `cloud` 타입은 cloud 변수로 인식됩니다.
- `RealtimeVar`, `RealTimeVar`, `realtime`, `realTime` 타입은 realtime 변수로 인식됩니다.

## 테스트

```text
cargo test --workspace
```

`test_input/` 디렉토리의 `.rs`와 `.ent`로 라운드트립 회귀를 검증합니다.

## 릴리즈

`v*` 태그를 푸시하면 GitHub Actions가 `entryc`와 `entrycgui` 릴리즈 바이너리를
자동 생성합니다. Windows와 Linux 두 플랫폼이 빌드되며, raw 바이너리를
플랫폼별 압축 파일로 묶어 GitHub Release에 첨부합니다.

릴리즈 절차:

1. 버전 태그 생성 후 푸시 (예: `v0.1.0`)
2. `.github/workflows/release.yml` 자동 실행
3. 빌드 산출물이 GitHub Release에 첨부되고 본문은 커밋 목록으로 자동 생성됨

워크플로우 위치:

- `.github/workflows/release.yml`

## 기여

이슈와 PR 환영합니다. PR 전 `cargo test --workspace` 통과를 확인해 주세요.

행동 강령은 별도로 명시하지 않습니다. 상호 존중을 기본으로 합니다.

## 라이선스

MIT License — 자세한 내용은 [LICENSE](LICENSE)를 참조하세요.

서드파티 의존성 라이선스는 [NOTICE](NOTICE)를 참조하세요. 의존성 추가/업그레이드
후 `cargo deny check licenses`로 호환성을 재검증합니다.

## 관련 자료

- `entryjs-basic-blocks-v2.md`: EntryJS 기본 블록 조사 자료
- `entrycore/src/parse`: Rust 소스 파서
- `entrycore/src/codegen`: Entry 프로젝트 데이터 생성기
- `entrycore/src/deparse.rs`: Entry 블록에서 Rust 소스 추출기
- `entryc/src/main.rs`: `entryc` CLI
- `.github/workflows/release.yml`: 태그 푸시 시 릴리즈 자동 생성
