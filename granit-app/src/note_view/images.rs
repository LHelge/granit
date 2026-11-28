//! Images in notes: `![alt](src)` sources resolved against the note's
//! directory, decoded off the UI thread and cached for the editor.
//!
//! The editor asks its image provider for a decoded image while it lays
//! out, so the provider only reads the cache; [`ImageCache::request`]
//! starts the decoding of sources it has not seen, and the note view
//! repaints the editor when one finishes.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui::{Image, ImageFormat, RenderImage, SvgRenderer};
use granit_core::markdown::cave_image_path;

/// Where a note's relative image sources resolve from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageBase {
    /// The cave root.
    pub cave: PathBuf,
    /// The note's directory, cave-relative with forward slashes (`""`: the
    /// root).
    pub note_dir: String,
}

impl ImageBase {
    /// The file `src` (as written between the parentheses of `![alt](src)`)
    /// points at, or `None` for a remote or out-of-cave source.
    pub fn resolve(&self, src: &str) -> Option<PathBuf> {
        let relative = cave_image_path(&self.note_dir, link_destination(src))?;
        Some(self.cave.join(relative))
    }
}

/// The link destination of a raw `(src)`: `<a b.png>` without the angle
/// brackets, or the part before an optional `"title"`.
fn link_destination(src: &str) -> &str {
    let src = src.trim();
    if let Some(inner) = src.strip_prefix('<') {
        return inner.split_once('>').map_or(inner, |(dest, _)| dest);
    }
    src.split_once([' ', '\t']).map_or(src, |(dest, _)| dest)
}

/// The raw `src` of every `![alt](src)` in `text`, the way the editor
/// passes it to the image provider: the text between `](` and the next `)`,
/// trimmed.
pub fn image_sources(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut rest = line;
        while let Some(at) = rest.find("![") {
            rest = &rest[at + 2..];
            let Some(close_alt) = rest.find("](") else {
                break;
            };
            let after = &rest[close_alt + 2..];
            let Some(close) = after.find(')') else {
                break;
            };
            let src = after[..close].trim();
            if !src.is_empty() {
                out.push(src);
            }
            rest = &after[close + 1..];
        }
    }
    out
}

/// The format of an image file, from its extension.
fn image_format(path: &Path) -> Option<ImageFormat> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => ImageFormat::Png,
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        "gif" => ImageFormat::Gif,
        "webp" => ImageFormat::Webp,
        "svg" => ImageFormat::Svg,
        "bmp" => ImageFormat::Bmp,
        "tif" | "tiff" => ImageFormat::Tiff,
        "ico" => ImageFormat::Ico,
        _ => return None,
    })
}

/// Read and decode the image at `path`, refusing files outside `cave` (a
/// symlink pointing out of it). Blocking; runs on the background executor.
pub fn load_image(cave: &Path, path: &Path, svg: SvgRenderer) -> Result<Arc<RenderImage>, String> {
    let format = image_format(path).ok_or("not an image file")?;
    let real = path.canonicalize().map_err(|err| err.to_string())?;
    let root = cave.canonicalize().map_err(|err| err.to_string())?;
    if !real.starts_with(&root) {
        return Err("outside the cave".into());
    }
    let bytes = std::fs::read(&real).map_err(|err| err.to_string())?;
    Image::from_bytes(format, bytes)
        .to_image_data(svg)
        .map_err(|err| err.to_string())
}

enum Entry {
    Loading,
    Ready(Arc<RenderImage>),
    Failed,
}

/// Decoded images by file, shared by every editor of the note view.
#[derive(Default)]
pub struct ImageCache {
    entries: RefCell<HashMap<PathBuf, Entry>>,
}

impl ImageCache {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The decoded image at `path`, once it has loaded.
    pub fn get(&self, path: &Path) -> Option<Arc<RenderImage>> {
        match self.entries.borrow().get(path) {
            Some(Entry::Ready(image)) => Some(image.clone()),
            _ => None,
        }
    }

    /// The files of `sources` that have not been requested yet, now marked
    /// as loading. The caller decodes them and reports with [`Self::finish`].
    pub fn request(&self, base: &ImageBase, sources: &[&str]) -> Vec<PathBuf> {
        let mut entries = self.entries.borrow_mut();
        let mut out = Vec::new();
        for path in sources.iter().filter_map(|src| base.resolve(src)) {
            if !entries.contains_key(&path) {
                entries.insert(path.clone(), Entry::Loading);
                out.push(path);
            }
        }
        out
    }

    /// Record the result of decoding `path`.
    pub fn finish(&self, path: PathBuf, result: Result<Arc<RenderImage>, String>) {
        let entry = match result {
            Ok(image) => Entry::Ready(image),
            Err(err) => {
                log::debug!("Could not load the image {}: {err}", path.display());
                Entry::Failed
            }
        };
        self.entries.borrow_mut().insert(path, entry);
    }

    /// Forget everything, so changed files are read again (on a cave
    /// change).
    pub fn clear(&self) {
        self.entries.borrow_mut().clear();
    }

    /// The editor's image provider for a note at `base`.
    pub fn provider(
        self: &Rc<Self>,
        base: ImageBase,
    ) -> impl Fn(&str) -> Option<Arc<RenderImage>> + 'static {
        let cache = self.clone();
        move |src| cache.get(&base.resolve(src)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(note_dir: &str) -> ImageBase {
        ImageBase {
            cave: PathBuf::from("/cave"),
            note_dir: note_dir.into(),
        }
    }

    #[test]
    fn sources_resolve_against_the_note_directory() {
        let sub = base("projects/alpha");
        assert_eq!(
            sub.resolve("img/a.png"),
            Some(PathBuf::from("/cave/projects/alpha/img/a.png"))
        );
        assert_eq!(
            sub.resolve("../b.png"),
            Some(PathBuf::from("/cave/projects/b.png"))
        );
        assert_eq!(
            sub.resolve("/top.png"),
            Some(PathBuf::from("/cave/top.png"))
        );
        assert_eq!(
            sub.resolve("<my image.png>"),
            Some(PathBuf::from("/cave/projects/alpha/my image.png"))
        );
        assert_eq!(
            sub.resolve("my%20image.png \"Title\""),
            Some(PathBuf::from("/cave/projects/alpha/my image.png"))
        );
        assert_eq!(base("").resolve("../escape.png"), None);
        assert_eq!(sub.resolve("https://example.com/a.png"), None);
        assert_eq!(sub.resolve("data:image/png;base64,AA"), None);
    }

    #[test]
    fn sources_are_found_on_every_line() {
        let text = "![a](one.png)\nText ![b](two.png \"t\") and ![](three.svg){width=40}\n\
                    ![no close](x\n`![code](four.png)` [not](image.png)\n";
        assert_eq!(
            image_sources(text),
            ["one.png", "two.png \"t\"", "three.svg", "four.png"]
        );
    }

    #[test]
    fn formats_come_from_the_extension() {
        assert_eq!(image_format(Path::new("a.PNG")), Some(ImageFormat::Png));
        assert_eq!(image_format(Path::new("a.jpeg")), Some(ImageFormat::Jpeg));
        assert_eq!(image_format(Path::new("a.svg")), Some(ImageFormat::Svg));
        assert_eq!(image_format(Path::new("a.pdf")), None);
        assert_eq!(image_format(Path::new("a")), None);
    }

    #[test]
    fn requests_are_made_once_per_file() {
        let cache = ImageCache::new();
        let note = base("sub");
        assert_eq!(
            cache.request(&note, &["a.png", "./a.png", "b.png"]),
            [
                PathBuf::from("/cave/sub/a.png"),
                PathBuf::from("/cave/sub/b.png")
            ]
        );
        assert!(cache.request(&note, &["a.png"]).is_empty());
        cache.finish(PathBuf::from("/cave/sub/a.png"), Err("broken".into()));
        assert!(cache.get(Path::new("/cave/sub/a.png")).is_none());
        cache.clear();
        assert_eq!(cache.request(&note, &["a.png"]).len(), 1);
    }
}
