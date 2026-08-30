use std::{io::Read, time::Duration};

use smithay::{
    backend::renderer::{
        element::{
            memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
            surface::WaylandSurfaceRenderElement,
            AsRenderElements,
        },
        ImportAll, ImportMem, Renderer, Texture,
    },
    input::pointer::CursorImageStatus,
    render_elements,
};
use xcursor::{
    parser::{parse_xcursor, Image},
    CursorTheme,
};

// FIXIFIIFI
static FALLBACK_CURSOR_DATA: &[u8] = include_bytes!("../resources/cursor.rgba");

pub struct Cursor {
    icons: Vec<Image>,
    size: u32,
}

impl Cursor {
    pub fn load() -> Cursor {
        let name = std::env::var("XCURSOR_THEME").ok().unwrap_or_else(|| "default".into());
        let size = std::env::var("XCURSOR_SIZE").ok().and_then(|s| s.parse().ok()).unwrap_or(24);

        let theme = CursorTheme::load(&name);
        let icons = load_icon(&theme).unwrap_or_else(|| {
            vec![Image {
                size: 32,
                width: 64,
                height: 64,
                xhot: 1,
                yhot: 1,
                delay: 1,
                pixels_rgba: Vec::from(FALLBACK_CURSOR_DATA),
                pixels_argb: vec![], //unused
            }]
        });

        Cursor {
            icons,
            size,
        }
    }

    pub fn get_image(&self, scale: u32, time: Duration) -> Image {
        let size = self.size * scale;
        frame(time.as_millis() as u32, size, &self.icons)
    }
}

fn nearest_images(size: u32, images: &[Image]) -> impl Iterator<Item = &Image> {
    // Follow the nominal size of the cursor to choose the nearest
    let nearest_image = images.iter().min_by_key(|image| (size as i32 - image.size as i32).abs()).unwrap();

    images.iter().filter(move |image| image.width == nearest_image.width && image.height == nearest_image.height)
}

fn frame(mut millis: u32, size: u32, images: &[Image]) -> Image {
    let total = nearest_images(size, images).fold(0, |acc, image| acc + image.delay);
    if total == 0 {
        return nearest_images(size, images).next().unwrap().clone();
    }
    millis %= total;

    for img in nearest_images(size, images) {
        if millis < img.delay {
            return img.clone();
        }
        millis -= img.delay;
    }

    unreachable!()
}

fn load_icon(theme: &CursorTheme) -> Option<Vec<Image>> {
    let mut cursor_file = theme.load_icon("default").and_then(|path| std::fs::File::open(path).ok())?;

    let mut cursor_data = Vec::new();
    cursor_file.read_to_end(&mut cursor_data).ok()?;

    parse_xcursor(&cursor_data)
}

// Pointer Element used for drawing a cursor. Only useful for udev backend

pub struct PointerElement {
    buffer: Option<MemoryRenderBuffer>,
    status: CursorImageStatus,
}

render_elements! {
    pub PointerRenderElement<R> where R: ImportAll + ImportMem;
        Surface=WaylandSurfaceRenderElement<R>,
        Memory=MemoryRenderBufferRenderElement<R>,
}

impl Default for PointerElement {
    fn default() -> Self {
        Self {
            buffer: Default::default(),
            status: CursorImageStatus::default_named(),
        }
    }
}

impl PointerElement {
    pub fn set_status(&mut self, status: CursorImageStatus) {
        self.status = status;
    }

    pub fn set_buffer(&mut self, buffer: MemoryRenderBuffer) {
        self.buffer = Some(buffer);
    }
}

impl<T: Texture + Clone + Send + 'static, R> AsRenderElements<R> for PointerElement
where
    R: Renderer<TextureId = T> + ImportAll + ImportMem,
{
    type RenderElement = PointerRenderElement<R>;

    fn render_elements<C: From<Self::RenderElement>>(&self, renderer: &mut R, location: smithay::utils::Point<i32, smithay::utils::Physical>, scale: smithay::utils::Scale<f64>, alpha: f32) -> Vec<C> {
        match &self.status {
            CursorImageStatus::Hidden => vec![],
            CursorImageStatus::Named(_) => {
                if let Some(buffer) = self.buffer.as_ref() {
                    vec![
                        PointerRenderElement::<R>::from(
                            MemoryRenderBufferRenderElement::from_buffer(renderer, location.to_f64(), buffer, None, None, None, smithay::backend::renderer::element::Kind::Cursor)
                                .expect("Lost system pointer buffer"),
                        )
                        .into(),
                    ]
                } else {
                    vec![]
                }
            }
            CursorImageStatus::Surface(surface) => {
                let elements: Vec<PointerRenderElement<R>> =
                    smithay::backend::renderer::element::surface::render_elements_from_surface_tree(renderer, surface, location, scale, alpha, smithay::backend::renderer::element::Kind::Cursor);
                elements.into_iter().map(C::from).collect()
            }
        }
    }
}
