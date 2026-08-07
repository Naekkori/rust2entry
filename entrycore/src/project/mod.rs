//! .ent zip 패키징.

pub mod pack;
pub mod sprite;

use crate::{Result, project::pack::ZipBuilder};
use std::path::Path;

pub use sprite::{PictureEntry, PictureMeta, SoundEntry, SoundMeta, SpriteEntry};

/// 스프라이트 디렉토리 -> SpriteEntry 목록.
pub fn collect_sprites(_dir: &Path) -> Result<Vec<SpriteEntry>> {
    sprite::collect_sprites(_dir)
}

/// project.json + 스프라이트 -> .ent zip 바이트스트림.
pub fn build(project: serde_json::Value, sprites: Option<&Path>) -> Result<Vec<u8>> {
    let sprites = match sprites {
        Some(p) => collect_sprites(p)?,
        None => Vec::new(),
    };

    let mut zip = ZipBuilder::new();

    let json = serde_json::to_vec(&project)?;
    zip.add_file("project.json", &json)?;

    for sprite in &sprites {
        for pic in &sprite.pictures {
            zip.add_file(&pic.meta.fileurl, &pic.image)?;

            if let Some(thumb) = &pic.thumb {
                zip.add_file(&pic.meta.thumburl, thumb)?;
            }
        }

        for snd in &sprite.sounds {
            zip.add_file(&snd.meta.fileurl, &snd.data)?;
        }
    }

    zip.finish()
}
