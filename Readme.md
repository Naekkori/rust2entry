# Rust2Entry
러스트 코드를 엔트리로 컴파일 합니다.

진행 상태/완료 모듈/TODO는 [AGENT.md](./AGENT.md) 참고.

## Editor 통합 (로드맵)

러스트 소스 자체는 `rust-analyzer`가 완전 지원. 본 프로젝트의 editor 통합은 **`.ent` 출력 + 매핑 진단** 두 축.

- **v0.1** — `cargo run --bin entryc -- sample.rs -o sample.ent`로 수동 빌드. EntryJS에 드래그&드롭으로 확인.
- **v0.2** — `build.rs` 훅으로 `cargo build` 중 자동 `.ent` 생성. `target/debug/sample.ent`.
- **v0.3+** — VSCode 확장:
  - `.rs` 저장 시 `.ent` 미리보기/JSON 다이프
  - 매핑 안 되는 Rust 구문 인레이 진단 (e.g. `async {}` → "엔트리에 async 블록 없음")
  - 가능하면 `entryc --lsp`로 사용자 LSP 서버 모드 지원

핵심: **Rust 코드가 1급 시민**, `.ent`는 부산물. 사용자는 Rust LSP만으로 Rust 기능 100% 사용 가능하고, `entryc`는 변환 파이프라인 역할만.

## 스킴구조 (EntryJS 에서 퍼옴)
```javascript
/**
 * MongoDB 스키마 예제.
 */
var ProjectSchema = new Schema({
    speed: { // 초당 실행 프레임수
        type: Number,
        default: 60
    },
    objects: [ // 오브젝트 목록
        {
            id: String, // 오브젝트 ID. Unique.
            name: String, // 오브젝트(또는 글상자 제목) 이름.
            text: String, // 글상자 내용. (objectType이 textBox일 경우)
            order: Number, // TODO
            objectType: String, // 오브젝트 유형. (sprite, textBox)
            scene: String, // 장면 ID. Unique.
            active: { // 오브젝트 활성화 여부
                type: Boolean,
                default: true
            },
            lock: { // 오브젝트 잠금 여부
                type: Boolean,
                default: false
            },
            rotateMethod: String, // 회전방식. (free, vertical, none)
            entity: { // 엔티티 정보
                rotation: Number, // 회전
                direction: Number, // 방향
                x: Number, // x 좌표
                y: Number, // y 좌표
                regX: Number, // 가로 중심점
                regY: Number, // 세로 중심점
                scaleX: Number, // 가로 배율
                scaleY: Number, // 세로 배율
                width: Number, // 넓이
                height: Number, // 높이
                imageIndex: Number, // TODO
                visible: Boolean, // 화면표시 여부
                colour: String, // 글상자 폰트색깔
                font: String, // 글상자 폰트
                bgColor: String, // 글상자 배경색깔
                textAlign: Number, // 글상자 정렬
                lineBreak: Boolean, // 글상자 줄바꿈 여부
                underLine: Boolean, // 글상자
                strike: Boolean // 글상자 밑줄
            },
            script: String, // 블록 스크립트
            sprite: { // 스프라이트 정보
                name: String, // 스프라이트 이름
                pictures: [{ // 모양 목록
                    id: String, // 모양 ID. Unique/
                    name: String, // 모양 이름
                    fileurl: String, // 모양 이미지
                    dimension: { // 모양 크기
                        width: Number,
                        height: Number,
                        scaleX: Number,
                        scaleY: Number
                    },
                    scale: { // 확대, 축소 비율(100% 기준)
                        type: Number,
                        default: 100
                    }
                }],
                sounds: [{ // 소리 목록
                    id: String, // 소리 ID. Unique.
                    name: String, // 이름
                    fileurl: String, // 사운드 파일 URL
                    duration: Number // 재생시간. (초단위)
                }]
            },
            selectedPictureId: String, // 현재 활성화된 모양의 ID
            selectedSoundId: String // 현재 활성화된 소리의 ID

        }
    ],
    variables: [ // 프로젝트 변수
        {
            name: String, // 변수명
            variableType: String, // 변수형. (일반변수: variable, 타이머: timer, 대답: answer, 슬라이드: slide, 리스트: list)
            id: String, // 변수ID. Unique.
            value: String, // 변수 값
            minValue: Number, // 최소값
            maxValue: Number, // 최대값
            visible: Boolean, // 캔버스에 표시여부
            x: Number, // 컨버스 위치 x좌표
            y: Number, // 캔버스 위치 y좌표
            width: Number, // 넓이
            height: Number, // 높이
            isCloud: { // 공유 변수 여부
                type: Boolean,
                default: false
            },
            object: { // 지역변수일 경우 참조하는 오브젝트 ID
                type: String,
                default: null
            },
            array: [{ // 변수형이 list일 경우 값 목록
                data: String // 값 데이터
            }]
        }
    ],
    messages: [ // 신호 목록
        {
            name: String, // 신호명
            id: String // 신호 ID. Unique.
        }
    ],
    functions: [ // 함수 목록
        {
            id: String, // 함수 ID. Unique.
            block: String, // 함수 블록 정보
            content: String, // 함수 실행 정보
                id: String,
                name: String
            }]
        }
    ],
    scenes: { // 장면 정보
        type: [ // 장면 목록
            {
                name: String, // 장면 이름
                id: String // 장면 ID. Unique.
            }
        ]
    },
});
```