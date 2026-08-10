# Rust2Entry

Rust 소스를 Entry 프로젝트 파일(`.ent`)로 변환하고, Entry 프로젝트 파일을 Rust 소스로 추출하는 도구입니다.

## 현재 기능

- `entrycore`: Rust 소스 파싱, IR 변환, Entry 블록 변환
- `entryc build`: 하나 이상의 `.rs` 파일을 `.ent`로 빌드
- `entryc extract`: `.ent`의 오브젝트별 스크립트를 `.rs` 파일로 추출
- 기존 `.ent` 템플릿 사용
- Rust 파일 이름과 Entry 오브젝트 이름을 대소문자 구분 없이 매칭
- 매칭되지 않은 Rust 파일을 새 sprite 오브젝트로 추가
- Rust 변수의 Entry 프로젝트 변수 생성 및 템플릿 변수 병합
- 변환할 수 없는 블록을 빌드 경고로 표시
- 추출할 수 없는 블록을 Rust 파일의 raw JSON 주석으로 보존

지원되는 Rust 문법과 Entry 블록은 구현 범위에 한정됩니다. 지원되지 않는 문법은 변환되지 않습니다.

## 빌드

```text
cargo build --workspace
```

## 사용법

### Rust에서 Entry 프로젝트 빌드

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

### Entry 프로젝트에서 Rust 추출

```text
cargo run --bin entryc -- extract --ent sample.ent
```

출력 폴더를 지정할 수 있습니다.

```text
cargo run --bin entryc -- extract --ent sample.ent --out extracted
```

기본 출력 폴더는 `.ent` 파일 위치의 프로젝트 이름 폴더입니다. 추출된 Rust 파일에는 `entryc` 생성기 헤더가 포함됩니다.

## Rust 입력 규칙

- `when_`으로 시작하는 함수는 Entry 트리거로 변환됩니다.
- 그 외 함수는 Entry 함수로 변환됩니다.
- top-level `static`은 전역 변수로 변환됩니다.
- `const`와 지원되지 않는 top-level item은 변환되지 않습니다.
- 변환할 수 없는 Rust 구문은 오류가 될 수 있습니다.
- Entry 블록으로 표현할 수 없는 일부 IR은 경고와 함께 변환에서 제외될 수 있습니다.

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

## 관련 자료

- `entryjs-basic-blocks-v2.md`: EntryJS 기본 블록 조사 자료
- `entrycore/src/parse`: Rust 소스 파서
- `entrycore/src/codegen`: Entry 프로젝트 데이터 생성기
- `entrycore/src/deparse.rs`: Entry 블록에서 Rust 소스 추출기
- `entryc/src/main.rs`: `entryc` CLI
