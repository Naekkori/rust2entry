//! collect_sprites 테스트.

use std::fs;
use std::path::PathBuf;

use entrycore::project::collect_sprites;

/// 임시 디렉토리 helper (테스트 종료 시 삭제 안 함 — 디버그 편의).
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("rust2entry_sprite_test").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// 가장 작은 유효 PNG (1x1 빨간색).
/// 바이트: 시그니처(8) + IHDR 길이(4) + "IHDR"(4) + w(4)=1 + h(4)=1 + ...
fn png_1x1() -> Vec<u8> {
    vec![
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, // sig
        0x00, 0x00, 0x00, 0x0d, // IHDR len
        b'I', b'H', b'D', b'R',
        0x00, 0x00, 0x00, 0x01, // width=1
        0x00, 0x00, 0x00, 0x01, // height=1
        0x08, 0x02, 0x00, 0x00, 0x00,
        0x90, 0x77, 0x53, 0xde, // CRC
        // IDAT 생략 — collect는 crc 검사 안 함, width/height만 읽음
        0x00, 0x00, 0x00, 0x00, // IEND len
        b'I', b'E', b'N', b'D',
        0xae, 0x42, 0x60, 0x82, // IEND crc
    ]
}

#[test]
fn collect_picture_only() {
    let root = temp_dir("pic_only");
    let sprite = root.join("hero");
    fs::create_dir_all(&sprite).unwrap();
    fs::write(sprite.join("hero.png"), png_1x1()).unwrap();

    let entries = collect_sprites(&root).expect("collect");
    assert_eq!(entries.len(), 1);
    let s = &entries[0];
    assert_eq!(s.name, "hero");
    assert_eq!(s.pictures.len(), 1);
    assert_eq!(s.sounds.len(), 0);

    let pic = &s.pictures[0];
    assert_eq!(pic.meta.name, "hero");
    assert_eq!(pic.meta.ext, "png");
    assert_eq!(pic.meta.width, 1);
    assert_eq!(pic.meta.height, 1);
    assert_eq!(pic.meta.fileurl, format!("image/{}.png", pic.meta.filename));
    assert!(pic.thumb.is_none());
    assert_eq!(pic.image.len(), png_1x1().len());
}

#[test]
fn collect_picture_with_thumb() {
    let root = temp_dir("pic_thumb");
    let sprite = root.join("npc");
    fs::create_dir_all(sprite.join("thumb")).unwrap();
    fs::write(sprite.join("walk.png"), png_1x1()).unwrap();
    fs::write(sprite.join("thumb").join("walk.png"), png_1x1()).unwrap();

    let entries = collect_sprites(&root).expect("collect");
    assert_eq!(entries.len(), 1);
    let pic = &entries[0].pictures[0];
    assert!(pic.thumb.is_some(), "thumb 매칭되어야 함");
    assert!(pic.meta.thumburl.starts_with("thumb/"));
}

#[test]
fn collect_sound() {
    let root = temp_dir("sound");
    let sprite = root.join("enemy");
    fs::create_dir_all(sprite.join("sound")).unwrap();
    fs::write(sprite.join("sound").join("hit.mp3"), b"fake-mp3-bytes").unwrap();

    let entries = collect_sprites(&root).expect("collect");
    let s = &entries[0];
    assert_eq!(s.pictures.len(), 0);
    assert_eq!(s.sounds.len(), 1);
    let snd = &s.sounds[0];
    assert_eq!(snd.meta.name, "hit");
    assert_eq!(snd.meta.ext, "mp3");
    assert_eq!(snd.meta.duration, 0.0);
    assert!(snd.meta.fileurl.starts_with("sound/"));
}

#[test]
fn collect_multiple_sprites() {
    let root = temp_dir("multi");
    for name in ["a", "b"] {
        let sprite = root.join(name);
        fs::create_dir_all(&sprite).unwrap();
        fs::write(sprite.join("x.png"), png_1x1()).unwrap();
    }

    let entries = collect_sprites(&root).expect("collect");
    assert_eq!(entries.len(), 2);
    let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"a"));
    assert!(names.contains(&"b"));
}

#[test]
fn empty_dir() {
    let root = temp_dir("empty");
    let entries = collect_sprites(&root).expect("collect");
    assert!(entries.is_empty());
}

#[test]
fn hashes_unique() {
    let root = temp_dir("hash");
    let sprite = root.join("s");
    fs::create_dir_all(&sprite).unwrap();
    fs::write(sprite.join("a.png"), png_1x1()).unwrap();
    fs::write(sprite.join("b.png"), png_1x1()).unwrap();

    let entries = collect_sprites(&root).expect("collect");
    let pics = &entries[0].pictures;
    assert_eq!(pics.len(), 2);
    assert_ne!(pics[0].meta.filename, pics[1].meta.filename);
    assert_ne!(pics[0].meta.id, pics[1].meta.id);
}