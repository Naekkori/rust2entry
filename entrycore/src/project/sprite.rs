//! 스프라이트 디렉토리 -> Entry sprite/picture/sound 메타.

use std::path::{Path, PathBuf};

use rand::Rng;

use crate::Result;

const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

fn hash_n(n: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..n)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

fn hash_id() -> String {
    hash_n(4)
}

fn hash_filename() -> String {
    hash_n(32)
}

fn png_dim(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 || &data[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((w, h))
}

/// 한 스프라이트 디렉토리에서 수집된 picture/sound.
#[derive(Debug)]
pub struct SpriteEntry {
    pub name: String,
    pub pictures: Vec<PictureEntry>,
    pub sounds: Vec<SoundEntry>,
}

/// picture 원본 + (옵션) 썸네일.
#[derive(Debug)]
pub struct PictureEntry {
    pub meta: PictureMeta,
    pub image: Vec<u8>,
    pub thumb: Option<Vec<u8>>,
}

/// picture 메타 (Entry objects[].sprite.pictures[] 형식).
#[derive(Debug, Clone)]
pub struct PictureMeta {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub fileurl: String,
    pub thumbname: String,
    pub thumburl: String,
    pub ext: String,
    pub width: u32,
    pub height: u32,
}

/// 사운드.
#[derive(Debug)]
pub struct SoundEntry {
    pub meta: SoundMeta,
    pub data: Vec<u8>,
}

/// 사운드 메타.
#[derive(Debug, Clone)]
pub struct SoundMeta {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub fileurl: String,
    pub ext: String,
    pub duration: f64,
}

fn ext_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn stem_of(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

fn is_image(ext: &str) -> bool {
    matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp")
}

fn is_sound(ext: &str) -> bool {
    matches!(ext, "mp3" | "wav" | "ogg" | "m4a")
}

fn read_file(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(Into::into)
}

/// 스프라이트 디렉토리 -> SpriteEntry 목록.
///
/// 레이아웃:
/// ```text
/// dir/
///   <sprite_a>/
///     foo.png            <- picture 원본 (image/foo.png)
///     bar.svg            <- picture 원본
///     thumb/foo.png      <- foo.png의 썸네일
///     sound/baz.mp3      <- 사운드
///   <sprite_b>/
///     ...
/// ```
///
/// thumb 매칭은 base name(stem) 기준. 원본이 `foo.png`이면 썸네일은
/// `thumb/foo.png` (또는 `foo.jpg` 등 확장자 무관).
pub fn collect_sprites(dir: &Path) -> Result<Vec<SpriteEntry>> {
    let mut sprites = Vec::new();
    let entries = std::fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        sprites.push(collect_one(&path, name)?);
    }
    Ok(sprites)
}

fn collect_one(sprite_dir: &Path, name: String) -> Result<SpriteEntry> {
    // 1) 원본 picture 수집 (sprite_dir 직접 자식 중 image)
    // 2) thumb/<stem>.* 수집
    // 3) sound/* 수집
    let mut pictures: Vec<(String, String, PathBuf, Option<(String, PathBuf)>)> = Vec::new();
    // (stem, ext, image_path, thumb(stem, path)?)
    let mut sounds: Vec<(String, String, PathBuf)> = Vec::new();

    for ent in std::fs::read_dir(sprite_dir)?.flatten() {
        let p = ent.path();
        if p.is_file() {
            let ext = ext_of(&p);
            let stem = stem_of(&p);
            if is_image(&ext) {
                pictures.push((stem.clone(), ext.clone(), p.clone(), None));
            }
            // 루트 사운드도 허용
            if is_sound(&ext) {
                sounds.push((stem, ext, p));
            }
        }
    }

    // thumb/, sound/ 서브폴더 2차 패스
    for ent in std::fs::read_dir(sprite_dir)?.flatten() {
        let p = ent.path();
        if !p.is_dir() {
            continue;
        }
        let dir_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if dir_name == "thumb" {
            for tent in std::fs::read_dir(&p)?.flatten() {
                let tp = tent.path();
                if !tp.is_file() {
                    continue;
                }
                let ext = ext_of(&tp);
                if !is_image(&ext) {
                    continue;
                }
                let stem = stem_of(&tp);
                for pic in pictures.iter_mut() {
                    if pic.0 == stem && pic.3.is_none() {
                        pic.3 = Some((ext.clone(), tp.clone()));
                        break;
                    }
                }
            }
        } else if dir_name == "sound" {
            for sent in std::fs::read_dir(&p)?.flatten() {
                let sp = sent.path();
                if !sp.is_file() {
                    continue;
                }
                let ext = ext_of(&sp);
                if !is_sound(&ext) {
                    continue;
                }
                let stem = stem_of(&sp);
                sounds.push((stem, ext, sp));
            }
        }
    }

    // picture -> PictureEntry
    let mut pic_entries = Vec::new();
    for (stem, ext, img_path, thumb) in pictures {
        let img_data = read_file(&img_path)?;
        let (width, height) = png_dim(&img_data).unwrap_or((100, 100));

        let filename = hash_filename();
        let fileurl = format!("image/{filename}.{ext}");

        let (thumbname, thumburl, thumb_data) = match thumb {
            Some((text, tpath)) => {
                let tdata = read_file(&tpath)?;
                let tname = hash_filename();
                let turl = format!("thumb/{tname}.{text}");
                (tname, turl, Some(tdata))
            }
            None => (String::new(), String::new(), None),
        };

        pic_entries.push(PictureEntry {
            meta: PictureMeta {
                id: hash_id(),
                name: stem,
                filename,
                fileurl,
                thumbname,
                thumburl,
                ext,
                width,
                height,
            },
            image: img_data,
            thumb: thumb_data,
        });
    }

    // sound -> SoundEntry
    let mut sound_entries = Vec::new();
    for (stem, ext, s_path) in sounds {
        let data = read_file(&s_path)?;
        let filename = hash_filename();
        let fileurl = format!("sound/{filename}.{ext}");
        sound_entries.push(SoundEntry {
            meta: SoundMeta {
                id: hash_id(),
                name: stem,
                filename,
                fileurl,
                ext,
                duration: 0.0,
            },
            data,
        });
    }

    Ok(SpriteEntry {
        name,
        pictures: pic_entries,
        sounds: sound_entries,
    })
}