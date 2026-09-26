mod frame;
mod raster;
mod state;

use std::sync::{Arc, Mutex, MutexGuard};

use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::editor::Editor;
use crate::prelude::*;
use state::ViewportState;

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ViewportRequest {
    pub scroll_x: f64,
    pub scroll_y: f64,
    pub width: f64,
    pub height: f64,
    pub occlusion_top: f64,
    pub occlusion_bottom: f64,
    pub device_scale: f64,
    pub header_height: f64,
    pub time_ms: f64,
    pub debug: bool,
    pub destination: Option<f64>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FrameGeometry {
    pub id: u64,
    pub revision: editor_core::Revision,
    pub tick: Option<editor_core::TickResult>,
    pub layout: FrameLayout,
    pub zoom: f64,
    pub raster_scale: f64,
    pub content_width: f64,
    pub content_height: f64,
    pub page_count: u32,
    pub body: FrameBody,
    pub pages: Vec<FramePage>,
    pub tiles: Vec<FrameTile>,
    pub position: Option<FramePosition>,
    pub debug: Option<FrameDebug>,
    pub needs_next_frame: bool,
    pub fill_remaining: bool,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrameLayout {
    #[serde(rename_all = "snake_case")]
    Paginated {
        margin_top: f64,
        margin_bottom: f64,
        margin_left: f64,
        margin_right: f64,
    },
    Continuous,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FrameBody {
    pub pages_top: f64,
    pub top_spacer: f64,
    pub pages_bottom: f64,
    pub bottom_padding: f64,
    pub minimum_body_bottom: f64,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FramePage {
    pub index: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FramePxRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl From<editor_viewport::PxRect> for FramePxRect {
    fn from(rect: editor_viewport::PxRect) -> Self {
        Self {
            x0: rect.x0,
            y0: rect.y0,
            x1: rect.x1,
            y1: rect.y1,
        }
    }
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrameTile {
    #[serde(rename_all = "snake_case")]
    Set {
        page: u32,
        bounds: FramePxRect,
        version: u64,
        pixels: u32,
    },
    #[serde(rename_all = "snake_case")]
    Drop { page: u32, bounds: FramePxRect },
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FramePosition {
    pub page: u32,
    pub pages: u32,
    pub percent: u32,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FrameDebug {
    pub pending: Vec<FrameTileRef>,
    pub invalidated: Vec<FrameTileRef>,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FrameTileRef {
    pub page: u32,
    pub bounds: FramePxRect,
}

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct EditorViewport {
    editor: Owned<Editor>,
    state: Mutex<ViewportState>,
}

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct EditorFrame {
    geometry: FrameGeometry,
    pixels: Vec<Arc<[u8]>>,
}

#[cfg_attr(feature = "uniffi", editor_macros::ffi_export(uniffi))]
impl EditorViewport {
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    pub fn new(editor: Owned<Editor>) -> EditorResult<Owned<Self>> {
        Ok(into_owned(Self {
            editor,
            state: Mutex::new(ViewportState::new()),
        }))
    }

    pub fn frame(&self, request: Complex<ViewportRequest>) -> EditorResult<Owned<EditorFrame>> {
        let request: ViewportRequest = request.from_ffi()?;
        let mut state = self.lock_state()?;
        let frame = self
            .editor
            .with_viewport_core(|core| frame::frame(core, &mut state, request))?;
        Ok(into_owned(frame))
    }

    pub fn fill(&self, budget_ms: f64) -> EditorResult<Option<Owned<EditorFrame>>> {
        let mut state = self.lock_state()?;
        let frame = self
            .editor
            .with_viewport_core(|core| Ok(frame::fill(core, &mut state, budget_ms)))?;
        Ok(frame.map(into_owned))
    }

    pub fn presented(&self, frame_id: u64) -> EditorResult<()> {
        self.lock_state()?.presented = Some(frame_id);
        Ok(())
    }
}

impl EditorViewport {
    fn lock_state(&self) -> EditorResult<MutexGuard<'_, ViewportState>> {
        self.state.lock().map_err(|_| FfiError::LockPoisoned.into())
    }

    #[cfg(test)]
    pub(crate) fn editor(&self) -> &Editor {
        &self.editor
    }
}

#[cfg_attr(feature = "uniffi", editor_macros::ffi_export(uniffi))]
impl EditorFrame {
    pub fn geometry(&self) -> EditorResult<Complex<FrameGeometry>> {
        Ok(self.geometry.clone().into_ffi()?)
    }

    pub fn pixel_count(&self) -> EditorResult<u32> {
        Ok(self.pixels.len() as u32)
    }

    pub fn pixel_address(&self, index: u32) -> EditorResult<u64> {
        Ok(self.pixel(index)?.as_ptr() as u64)
    }

    pub fn pixel_length(&self, index: u32) -> EditorResult<u64> {
        Ok(self.pixel(index)?.len() as u64)
    }
}

impl EditorFrame {
    fn pixel(&self, index: u32) -> EditorResult<&Arc<[u8]>> {
        self.pixels
            .get(index as usize)
            .ok_or_else(|| EditorError::General {
                msg: format!("frame pixel index {index} is out of range"),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::OnceLock;

    use editor_core::{EditorEvent, InsertionOp, Message, SelectionOp, SystemEvent};
    use editor_model::{
        LayoutMode, Modifier, ModifierType, PlainDoc, PlainNode, PlainNodeEntry,
        PlainParagraphNode, PlainRootNode, PlainTextNode,
    };
    use editor_renderer::damage::IRect;
    use editor_renderer::display_list::DisplayList;
    use editor_resource::{FontFamily, FontFamilySource, FontManifest, FontWeight};
    use editor_viewport::PxRect;

    use super::*;
    use crate::host::EditorHost;

    const FONT: &[u8] = include_bytes!("../../../../assets/Pretendard-Regular.ttf");
    const PANGRAM: &str = "the quick brown fox jumps over the lazy dog";
    const PAGE: LayoutMode = LayoutMode::Paginated {
        page_width: 400,
        page_height: 600,
        page_margin_top: 20,
        page_margin_bottom: 20,
        page_margin_left: 20,
        page_margin_right: 20,
    };
    const WIDE_PAGE: LayoutMode = LayoutMode::Paginated {
        page_width: 800,
        page_height: 1200,
        page_margin_top: 40,
        page_margin_bottom: 40,
        page_margin_left: 40,
        page_margin_right: 40,
    };

    type TileKey = (u32, [i32; 4]);

    impl FramePxRect {
        fn edges(self) -> [i32; 4] {
            [self.x0, self.y0, self.x1, self.y1]
        }
    }

    fn compressed_font() -> &'static [u8] {
        static COMPRESSED: OnceLock<Vec<u8>> = OnceLock::new();
        COMPRESSED.get_or_init(|| editor_resource::compress_zstd(FONT))
    }

    fn font_host(with_font_data: bool) -> EditorHost {
        let host = EditorHost::new_test();
        host.set_fonts(vec![FontFamily {
            name: "test".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "test-400".into(),
            }],
        }])
        .unwrap();
        if with_font_data {
            let manifest = FontManifest::from_coverages(&[vec![0x0000, 0x00FF]]);
            host.add_font_manifest(
                "test".into(),
                400,
                editor_resource::compress_zstd(&manifest.to_bytes()),
            )
            .unwrap();
            host.add_font_base("test".into(), 400, compressed_font().to_vec())
                .unwrap();
        }
        host
    }

    fn document(layout_mode: LayoutMode, text: &str, paragraphs: usize) -> PlainDoc {
        let paragraph = PlainNodeEntry {
            node: PlainNode::Paragraph(PlainParagraphNode {}),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children: vec![PlainNodeEntry {
                node: PlainNode::Text(PlainTextNode { text: text.into() }),
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children: vec![],
            }],
        };
        PlainDoc {
            root: PlainNodeEntry {
                node: PlainNode::Root(PlainRootNode { layout_mode }),
                modifiers: BTreeMap::from([
                    (
                        ModifierType::FontFamily,
                        Modifier::FontFamily {
                            value: "test".into(),
                        },
                    ),
                    (
                        ModifierType::FontWeight,
                        Modifier::FontWeight { value: 400 },
                    ),
                ]),
                carry: Vec::new(),
                children: vec![paragraph; paragraphs],
            },
        }
    }

    fn open(doc: PlainDoc, width: f64, height: f64, scale: f64) -> EditorViewport {
        let editor = font_host(true)
            .create_editor_from_doc(
                doc,
                editor_view::Viewport::new(width as f32, height as f32, scale),
            )
            .unwrap();
        editor
            .enqueue_request(vec![Message::System {
                event: SystemEvent::Initialize,
            }])
            .unwrap();
        editor.tick().unwrap();
        EditorViewport::new(editor).unwrap()
    }

    fn request(scroll_y: f64, width: f64, height: f64, scale: f64) -> ViewportRequest {
        ViewportRequest {
            scroll_x: 0.0,
            scroll_y,
            width,
            height,
            occlusion_top: 0.0,
            occlusion_bottom: 0.0,
            device_scale: scale,
            header_height: 0.0,
            time_ms: 0.0,
            debug: false,
            destination: None,
        }
    }

    fn sets(frame: &EditorFrame) -> BTreeMap<TileKey, u64> {
        frame
            .geometry
            .tiles
            .iter()
            .filter_map(|tile| match tile {
                FrameTile::Set {
                    page,
                    bounds,
                    version,
                    ..
                } => Some(((*page, bounds.edges()), *version)),
                FrameTile::Drop { .. } => None,
            })
            .collect()
    }

    fn drops(frame: &EditorFrame) -> BTreeSet<TileKey> {
        frame
            .geometry
            .tiles
            .iter()
            .filter_map(|tile| match tile {
                FrameTile::Drop { page, bounds } => Some((*page, bounds.edges())),
                FrameTile::Set { .. } => None,
            })
            .collect()
    }

    fn shown(viewport: &EditorViewport) -> BTreeMap<TileKey, u64> {
        viewport
            .state
            .lock()
            .unwrap()
            .emitted
            .iter()
            .map(|(&(page, bounds), &version)| {
                ((page as u32, FramePxRect::from(bounds).edges()), version)
            })
            .collect()
    }

    fn drain(
        viewport: &EditorViewport,
        budget_ms: f64,
        mut each: impl FnMut(&EditorFrame),
    ) -> usize {
        let limit = {
            let state = viewport.state.lock().unwrap();
            let plan = state.plan.as_ref().unwrap();
            plan.pages
                .values()
                .map(|page| page.required().len())
                .sum::<usize>()
                + plan.pages.len()
                + 1
        };
        let mut calls = 0;
        while let Some(frame) = viewport.fill(budget_ms).unwrap() {
            calls += 1;
            assert!(calls <= limit, "fill did not settle within {limit} calls");
            each(&frame);
        }
        calls
    }

    fn fill_all(viewport: &EditorViewport, scale: f64) -> BTreeMap<TileKey, u64> {
        let mut filled = BTreeMap::new();
        drain(viewport, f64::INFINITY, |frame| {
            assert_pixels_match(viewport, frame, scale);
            filled.extend(sets(frame));
        });
        filled
    }

    fn decelerating(
        scroll_y: f64,
        destination: f64,
        width: f64,
        height: f64,
        scale: f64,
    ) -> ViewportRequest {
        ViewportRequest {
            destination: Some(destination),
            ..request(scroll_y, width, height, scale)
        }
    }

    fn tile_bounds((_, edges): TileKey) -> PxRect {
        PxRect {
            x0: edges[0],
            y0: edges[1],
            x1: edges[2],
            y1: edges[3],
        }
    }

    fn tile_key(page: usize, bounds: PxRect) -> TileKey {
        (page as u32, FramePxRect::from(bounds).edges())
    }

    fn reference(viewport: &EditorViewport, key: TileKey, scale: f64) -> Option<Vec<u8>> {
        viewport
            .editor()
            .with_viewport_core(|core| {
                let (list, _) = core.build_display_list(key.0, scale as f32).unwrap();
                Ok(
                    raster::rasterize(&list.primitives, tile_bounds(key), &mut raster::scratch())
                        .map(|pixels| pixels.to_vec()),
                )
            })
            .unwrap()
    }

    fn rastered(viewport: &EditorViewport) -> BTreeSet<TileKey> {
        let state = viewport.state.lock().unwrap();
        state
            .pages
            .iter()
            .flat_map(|(&page, raster)| {
                raster
                    .tiles
                    .keys()
                    .map(move |&bounds| tile_key(page, bounds))
            })
            .collect()
    }

    fn planned_tiles(
        viewport: &EditorViewport,
        area: impl Fn(&state::PagePlan) -> BTreeSet<PxRect>,
    ) -> BTreeSet<TileKey> {
        let state = viewport.state.lock().unwrap();
        state
            .plan
            .as_ref()
            .unwrap()
            .pages
            .iter()
            .flat_map(|(&page, plan)| {
                area(plan)
                    .into_iter()
                    .map(move |bounds| tile_key(page, bounds))
            })
            .collect()
    }

    fn assert_rows_whole(viewport: &EditorViewport, scale: f64) {
        let mut rows: BTreeMap<(u32, i32), Vec<TileKey>> = BTreeMap::new();
        for key in planned_tiles(viewport, state::PagePlan::required) {
            rows.entry((key.0, key.1[1])).or_default().push(key);
        }
        let held = shown(viewport);
        for key in held.keys() {
            assert!(
                rows.get(&(key.0, key.1[1]))
                    .is_some_and(|row| row.contains(key)),
                "held tile {key:?} is outside the required rows"
            );
        }
        for ((page, y0), row) in rows {
            if row.iter().all(|key| !held.contains_key(key)) {
                continue;
            }
            for key in row.iter().filter(|key| !held.contains_key(key)) {
                assert!(
                    reference(viewport, *key, scale).is_none(),
                    "page {page} row {y0} is shown without its painted tile {key:?}"
                );
            }
        }
    }

    fn shown_rows(viewport: &EditorViewport) -> BTreeSet<(u32, i32)> {
        shown(viewport)
            .into_keys()
            .map(|key| (key.0, key.1[1]))
            .collect()
    }

    fn display_list(viewport: &EditorViewport, page: u32, scale: f64) -> (DisplayList, IRect) {
        viewport
            .editor()
            .with_viewport_core(|core| Ok(core.build_display_list(page, scale as f32).unwrap()))
            .unwrap()
    }

    fn assert_shown_rows_kept(
        viewport: &EditorViewport,
        rows_before: &BTreeSet<(u32, i32)>,
        scale: f64,
    ) {
        let rows_after = shown_rows(viewport);
        for key in planned_tiles(viewport, |plan| plan.visible.clone()) {
            let row = (key.0, key.1[1]);
            if rows_before.contains(&row) && !rows_after.contains(&row) {
                assert!(
                    reference(viewport, key, scale).is_none(),
                    "shown row {row:?} emptied while visible and painted"
                );
            }
        }
    }

    fn assert_visible_shown(viewport: &EditorViewport, scale: f64) {
        let held = shown(viewport);
        let visible = planned_tiles(viewport, |plan| plan.visible.clone());
        assert!(!visible.is_empty());
        for key in visible {
            assert!(
                held.contains_key(&key) || reference(viewport, key, scale).is_none(),
                "visible painted tile {key:?} is not shown"
            );
        }
    }

    fn read_pixels(frame: &EditorFrame, index: u32) -> Vec<u8> {
        let address = frame.pixel_address(index).unwrap() as usize as *const u8;
        let length = frame.pixel_length(index).unwrap() as usize;
        unsafe { std::slice::from_raw_parts(address, length) }.to_vec()
    }

    fn assert_pixels_match(viewport: &EditorViewport, frame: &EditorFrame, scale: f64) {
        for tile in &frame.geometry.tiles {
            if let FrameTile::Set {
                page,
                bounds,
                pixels,
                ..
            } = tile
            {
                raster::assert_same_pixels(
                    Some(&read_pixels(frame, *pixels)),
                    reference(viewport, (*page, bounds.edges()), scale).as_deref(),
                    format_args!("page {page} tile {bounds:?}"),
                );
            }
        }
    }

    fn assert_held_pixels_match(viewport: &EditorViewport, scale: f64) {
        assert!(held_pixels_match(viewport, scale) > 0);
    }

    fn held_pixels_match(viewport: &EditorViewport, scale: f64) -> usize {
        let held: Vec<(TileKey, Vec<u8>)> = {
            let state = viewport.state.lock().unwrap();
            state
                .emitted
                .iter()
                .map(|(&(page, bounds), &version)| {
                    let tile = &state.pages[&page].tiles[&bounds];
                    assert_eq!(tile.version, version);
                    let pixels = tile.pixels.as_deref().expect("a shown tile has pixels");
                    (
                        (page as u32, FramePxRect::from(bounds).edges()),
                        pixels.to_vec(),
                    )
                })
                .collect()
        };
        let count = held.len();
        for (key, pixels) in held {
            raster::assert_same_pixels(
                Some(&pixels),
                reference(viewport, key, scale).as_deref(),
                format_args!("held tile {key:?}"),
            );
        }
        count
    }

    fn fill_calls(viewport: &EditorViewport, budget_ms: f64, scale: f64) -> Vec<Vec<TileKey>> {
        let mut calls = Vec::new();
        drain(viewport, budget_ms, |frame| {
            assert_pixels_match(viewport, frame, scale);
            calls.push(sets(frame).into_keys().collect());
        });
        calls
    }

    fn scrolled_without_fill() -> (EditorViewport, BTreeMap<TileKey, u64>) {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 400.0, 2.0);
        viewport.frame(request(0.0, 400.0, 400.0, 2.0)).unwrap();
        let scrolled = viewport.frame(request(300.0, 400.0, 400.0, 2.0)).unwrap();
        assert!(sets(&scrolled).is_empty());
        assert!(drops(&scrolled).is_empty());
        let before = shown(&viewport);
        assert_eq!(
            before.keys().copied().collect::<Vec<_>>(),
            vec![(0, [0, 0, 512, 512]), (0, [0, 512, 512, 1024])]
        );
        (viewport, before)
    }

    fn edit_rows(viewport: &EditorViewport, points: &[(f32, f32)]) {
        edit_page_rows(viewport, 0, points);
    }

    fn edit_page_rows(viewport: &EditorViewport, page: usize, points: &[(f32, f32)]) {
        let mut messages = Vec::new();
        for &(x, y) in points {
            messages.push(Message::Selection {
                op: SelectionOp::SetAt { page, x, y },
            });
            messages.push(Message::Insertion {
                op: InsertionOp::Text { text: "x".into() },
            });
        }
        viewport.editor().enqueue_request(messages).unwrap();
    }

    #[test]
    fn first_frame_sets_exactly_the_visible_painted_tiles() {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 600.0, 2.0);
        let frame = viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        let geometry = frame.geometry().unwrap();

        assert_eq!(geometry.id, 1);
        assert_eq!((geometry.zoom, geometry.raster_scale), (1.0, 2.0));
        assert!(geometry.tick.is_none());
        assert_eq!(
            geometry
                .pages
                .iter()
                .map(|page| page.index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            sets(&frame).keys().copied().collect::<Vec<_>>(),
            vec![
                (0, [0, 0, 512, 512]),
                (0, [0, 512, 512, 1024]),
                (0, [0, 1024, 512, 1200]),
            ]
        );
        for blank in [
            [512, 0, 800, 512],
            [512, 512, 800, 1024],
            [512, 1024, 800, 1200],
        ] {
            raster::assert_same_pixels(
                reference(&viewport, (0, blank), 2.0).as_deref(),
                None,
                format_args!("blank tile {blank:?}"),
            );
        }
        assert!(drops(&frame).is_empty());
        assert_pixels_match(&viewport, &frame, 2.0);
        assert_eq!(
            geometry.position,
            Some(FramePosition {
                page: 1,
                pages: geometry.page_count,
                percent: 0,
            })
        );
        assert!(geometry.fill_remaining);
        assert!(!geometry.needs_next_frame);
        assert!(geometry.debug.is_none());

        viewport.presented(geometry.id).unwrap();
        assert_eq!(viewport.state.lock().unwrap().presented, Some(1));
    }

    #[test]
    fn scroll_frames_raster_nothing_and_fill_follows_the_fill_order() {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 600.0, 2.0);
        viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        fill_all(&viewport, 2.0);
        let before = shown(&viewport);

        let scrolled = viewport.frame(request(700.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(sets(&scrolled).is_empty());
        assert_eq!(drops(&scrolled), BTreeSet::from([(0, [0, 0, 512, 512])]));
        assert!(scrolled.geometry.fill_remaining);
        let kept = shown(&viewport);
        assert!(
            kept.iter()
                .all(|(key, version)| before.get(key) == Some(version))
        );

        let mut calls = Vec::new();
        drain(&viewport, 0.0, |frame| {
            assert!(drops(frame).is_empty());
            assert_pixels_match(&viewport, frame, 2.0);
            calls.push(sets(frame).into_keys().collect::<Vec<_>>());
        });
        assert_eq!(
            calls,
            vec![
                vec![],
                vec![(1, [0, 1024, 512, 1200])],
                vec![],
                vec![(2, [0, 0, 512, 512])],
                vec![],
                vec![(2, [0, 512, 512, 1024])],
            ]
        );

        let farther = viewport.frame(request(1400.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(sets(&farther).is_empty());
        let frame = viewport.fill(f64::INFINITY).unwrap().unwrap();
        assert!(!sets(&frame).is_empty());
        assert!(!frame.geometry.fill_remaining);
        assert!(viewport.fill(f64::INFINITY).unwrap().is_none());
    }

    #[test]
    fn edit_frames_repaint_only_visible_damaged_tiles() {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 400.0, 2.0);
        let first = viewport.frame(request(100.0, 400.0, 400.0, 2.0)).unwrap();
        fill_all(&viewport, 2.0);
        let before = shown(&viewport);
        assert_eq!(
            before.keys().copied().collect::<Vec<_>>(),
            vec![
                (0, [0, 0, 512, 512]),
                (0, [0, 512, 512, 1024]),
                (0, [0, 1024, 512, 1200]),
                (1, [0, 0, 512, 512]),
            ]
        );

        edit_rows(&viewport, &[(300.0, 40.0), (300.0, 560.0)]);
        let edit = viewport
            .frame(ViewportRequest {
                debug: true,
                ..request(100.0, 400.0, 400.0, 2.0)
            })
            .unwrap();
        assert!(edit.geometry.revision > first.geometry.revision);
        let repainted = sets(&edit);
        assert_eq!(
            repainted.keys().copied().collect::<Vec<_>>(),
            vec![(0, [0, 0, 512, 512])]
        );
        assert!(repainted[&(0, [0, 0, 512, 512])] > before[&(0, [0, 0, 512, 512])]);
        assert_eq!(
            drops(&edit),
            BTreeSet::from([(0, [0, 1024, 512, 1200]), (1, [0, 0, 512, 512])])
        );
        assert_eq!(
            shown(&viewport)[&(0, [0, 512, 512, 1024])],
            before[&(0, [0, 512, 512, 1024])]
        );
        assert_pixels_match(&viewport, &edit, 2.0);
        assert_held_pixels_match(&viewport, 2.0);
        let debug = edit.geometry.debug.clone().unwrap();
        assert_eq!(
            debug
                .invalidated
                .iter()
                .map(|tile| (tile.page, tile.bounds.edges()))
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([(0, [0, 1024, 512, 1200]), (1, [0, 0, 512, 512])])
        );
        assert!(debug.pending.is_empty());
        assert!(edit.geometry.fill_remaining);

        let filled = fill_all(&viewport, 2.0);
        assert_eq!(
            filled.keys().copied().collect::<Vec<_>>(),
            vec![(0, [0, 1024, 512, 1200]), (1, [0, 0, 512, 512])]
        );
        assert!(filled[&(0, [0, 1024, 512, 1200])] > before[&(0, [0, 1024, 512, 1200])]);
        assert_eq!(
            filled[&(1, [0, 0, 512, 512])],
            before[&(1, [0, 0, 512, 512])]
        );
    }

    #[test]
    fn pages_leaving_the_page_view_set_are_released() {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 600.0, 2.0);
        viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        fill_all(&viewport, 2.0);
        let before = shown(&viewport);
        assert!(before.keys().any(|(page, _)| *page == 1));

        let far = viewport.frame(request(3000.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(sets(&far).is_empty());
        assert_eq!(drops(&far), before.keys().copied().collect());
        let pages: Vec<u32> = far.geometry.pages.iter().map(|page| page.index).collect();
        assert_eq!(pages, vec![3, 4, 5, 6]);
        assert!(
            viewport
                .state
                .lock()
                .unwrap()
                .pages
                .keys()
                .all(|page| (3..=6).contains(page))
        );

        let back = viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(back.geometry.tiles.is_empty());
        assert!(back.geometry.fill_remaining);
    }

    #[test]
    fn tile_pixels_are_addressable_with_a_one_pixel_gutter() {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 600.0, 2.0);
        let frame = viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        let count = frame.pixel_count().unwrap();
        assert_eq!(count as usize, sets(&frame).len());
        let mut addresses = BTreeSet::new();
        for tile in &frame.geometry.tiles {
            let FrameTile::Set { bounds, pixels, .. } = tile else {
                continue;
            };
            let expected = (bounds.x1 - bounds.x0 + 2) * (bounds.y1 - bounds.y0 + 2) * 4;
            assert_eq!(frame.pixel_length(*pixels).unwrap(), expected as u64);
            assert!(addresses.insert(frame.pixel_address(*pixels).unwrap()));
            assert!(
                read_pixels(&frame, *pixels)
                    .chunks(4)
                    .any(|pixel| pixel[3] > 0)
            );
        }
        assert!(frame.pixel_address(count).is_err());
        assert!(frame.pixel_length(count).is_err());
    }

    #[test]
    fn continuous_width_change_reflows_in_the_same_frame() {
        let text = [PANGRAM, PANGRAM].join(" ");
        let continuous = LayoutMode::Continuous { max_width: 600 };
        let viewport = open(document(continuous, &text, 40), 400.0, 600.0, 2.0);
        let wide = viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(wide.geometry.tick.is_none());
        assert_eq!(wide.geometry.pages[0].width, 400.0);

        let narrow = viewport.frame(request(0.0, 300.0, 600.0, 2.0)).unwrap();
        let tick = narrow.geometry.tick.clone().expect("resize tick");
        assert_eq!(tick.revision, narrow.geometry.revision);
        assert!(tick.events.iter().any(|event| matches!(
            event,
            EditorEvent::StateChanged { fields } if fields.contains(&editor_core::StateField::PageSizes)
        )));
        assert_eq!(narrow.geometry.pages[0].width, 300.0);
        assert!(narrow.geometry.content_height > wide.geometry.content_height);
        assert!(!sets(&narrow).is_empty());
        assert!(sets(&narrow).keys().all(|(_, edges)| edges[2] <= 600));
        let wide_only: BTreeSet<TileKey> = sets(&wide)
            .into_keys()
            .filter(|(_, edges)| edges[2] > 600)
            .collect();
        assert!(!wide_only.is_empty());
        assert!(wide_only.is_subset(&drops(&narrow)));
        assert_pixels_match(&viewport, &narrow, 2.0);
        assert_held_pixels_match(&viewport, 2.0);

        let again = viewport.frame(request(0.0, 300.0, 600.0, 2.0)).unwrap();
        assert!(again.geometry.tick.is_none());
        assert_eq!(again.geometry.revision, narrow.geometry.revision);

        let collapsed = viewport.frame(request(0.0, 0.0, 600.0, 2.0)).unwrap();
        assert!(collapsed.geometry.tick.is_none());
        assert_eq!(collapsed.geometry.revision, narrow.geometry.revision);
    }

    #[test]
    fn frames_carry_the_tick_events() {
        let editor = font_host(false)
            .create_editor_from_doc(
                document(PAGE, "alpha", 1),
                editor_view::Viewport::new(400.0, 600.0, 2.0),
            )
            .unwrap();
        let viewport = EditorViewport::new(editor).unwrap();
        viewport
            .editor()
            .enqueue_request(vec![Message::System {
                event: SystemEvent::Initialize,
            }])
            .unwrap();

        let frame = viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        let tick = frame.geometry.tick.clone().expect("initialize tick");
        assert_eq!(tick.revision, frame.geometry.revision);
        assert!(tick.events.iter().any(|event| matches!(
            event,
            EditorEvent::FontDataMissing { family, weight: 400, .. } if family == "test"
        )));
        let next = viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(next.geometry.tick.is_none());
    }

    #[test]
    fn failed_editors_fail_frames() {
        let viewport = open(document(PAGE, "alpha", 1), 400.0, 600.0, 2.0);
        viewport
            .editor()
            .enqueue_request(vec![Message::Node {
                op: editor_core::NodeOp::SetAttr {
                    id: editor_crdt::Dot::ROOT,
                    attr: editor_model::NodeAttr::Image {
                        attr: editor_model::ImageNodeAttr::Proportion(50),
                    },
                },
            }])
            .unwrap();

        let Err(error) = viewport.frame(request(0.0, 400.0, 600.0, 2.0)) else {
            panic!("a failing tick must fail the frame");
        };
        assert!(matches!(error, EditorError::Core(_)));
        assert!(matches!(
            viewport.frame(request(0.0, 400.0, 600.0, 2.0)),
            Err(EditorError::Ffi(FfiError::EditorFailed))
        ));
        assert!(matches!(
            viewport.fill(4.0),
            Err(EditorError::Ffi(FfiError::EditorFailed))
        ));
        assert!(viewport.presented(1).is_ok());
    }

    #[test]
    fn frames_carry_only_the_page_view_set() {
        let short = LayoutMode::Paginated {
            page_width: 400,
            page_height: 200,
            page_margin_top: 20,
            page_margin_bottom: 20,
            page_margin_left: 20,
            page_margin_right: 20,
        };
        let viewport = open(document(short, "alpha", 600), 400.0, 600.0, 2.0);
        let frame = viewport.frame(request(5000.0, 400.0, 600.0, 2.0)).unwrap();
        let geometry = frame.geometry().unwrap();

        assert!(geometry.page_count >= 100);
        assert!(!geometry.pages.is_empty() && geometry.pages.len() <= 10);
        assert!(
            geometry
                .pages
                .iter()
                .all(|page| { page.y < 5600.0 + 600.0 && page.y + page.height > 5000.0 - 600.0 })
        );
        let indices: BTreeSet<u32> = geometry.pages.iter().map(|page| page.index).collect();
        assert!(geometry.tiles.iter().all(|tile| match tile {
            FrameTile::Set { page, .. } | FrameTile::Drop { page, .. } => indices.contains(page),
        }));
        assert!(serde_json::to_string(&geometry).unwrap().len() < 8192);

        let sets_in_page_set = |frame: &EditorFrame| {
            let indices: BTreeSet<u32> =
                frame.geometry.pages.iter().map(|page| page.index).collect();
            let set = sets(frame);
            assert!(set.keys().all(|(page, _)| indices.contains(page)));
            set.len()
        };
        let fill_in_page_set = || {
            let mut filled = 0;
            drain(&viewport, f64::INFINITY, |fill| {
                filled += sets_in_page_set(fill);
            });
            filled
        };
        assert!(fill_in_page_set() > 0);

        let visible = geometry
            .pages
            .iter()
            .find(|page| page.y >= 5000.0 && page.y + page.height <= 5600.0)
            .unwrap()
            .index;
        edit_page_rows(&viewport, visible as usize, &[(300.0, 40.0)]);
        let edit = viewport.frame(request(5000.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(edit.geometry.revision > geometry.revision);
        assert!(sets_in_page_set(&edit) > 0);
        fill_in_page_set();

        edit_page_rows(&viewport, visible as usize, &[(300.0, 40.0)]);
        let scrolled = viewport.frame(request(5400.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(scrolled.geometry.revision > edit.geometry.revision);
        sets_in_page_set(&scrolled);
        assert!(fill_in_page_set() > 0);
    }

    #[test]
    fn fill_waits_for_a_frame_after_the_revision_changes() {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 600.0, 2.0);
        let first = viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(first.geometry.fill_remaining);
        edit_rows(&viewport, &[(300.0, 40.0)]);
        viewport.editor().tick().unwrap();

        assert!(viewport.fill(f64::INFINITY).unwrap().is_none());
        let next = viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(next.geometry.revision > first.geometry.revision);
        assert!(next.geometry.tick.is_none());
        assert_eq!(
            sets(&next).keys().copied().collect::<Vec<_>>(),
            vec![(0, [0, 0, 512, 512])]
        );
        assert!(viewport.fill(f64::INFINITY).unwrap().is_some());
    }

    #[test]
    fn position_follows_the_clamped_scroll_offset() {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 600.0, 2.0);
        let before = viewport
            .frame(request(-1_000.0, 400.0, 600.0, 2.0))
            .unwrap();
        let pages = before.geometry.page_count;
        assert!(pages > 1);
        assert_eq!(
            before.geometry.position,
            Some(FramePosition {
                page: 1,
                pages,
                percent: 0,
            })
        );

        let past = viewport
            .frame(request(1_000_000.0, 400.0, 600.0, 2.0))
            .unwrap();
        assert_eq!(
            past.geometry.position,
            Some(FramePosition {
                page: pages,
                pages,
                percent: 100,
            })
        );
    }

    #[test]
    fn edit_frames_leave_scroll_exposed_tiles_to_fill() {
        let (viewport, before) = scrolled_without_fill();
        let exposed = [(0, [0, 1024, 512, 1200]), (1, [0, 0, 512, 512])];

        edit_rows(&viewport, &[(300.0, 300.0)]);
        let edit = viewport
            .frame(ViewportRequest {
                debug: true,
                ..request(300.0, 400.0, 400.0, 2.0)
            })
            .unwrap();
        let repainted = sets(&edit);
        assert_eq!(
            repainted.keys().copied().collect::<Vec<_>>(),
            vec![(0, [0, 512, 512, 1024])]
        );
        assert!(repainted[&(0, [0, 512, 512, 1024])] > before[&(0, [0, 512, 512, 1024])]);
        assert_pixels_match(&viewport, &edit, 2.0);
        assert_held_pixels_match(&viewport, 2.0);
        let pending: BTreeSet<TileKey> = edit
            .geometry
            .debug
            .clone()
            .unwrap()
            .pending
            .iter()
            .map(|tile| (tile.page, tile.bounds.edges()))
            .collect();
        assert!(exposed.iter().all(|key| pending.contains(key)));
        assert!(edit.geometry.fill_remaining);

        let calls = fill_calls(&viewport, 0.0, 2.0);
        assert_eq!(
            calls[..4],
            [vec![], vec![exposed[0]], vec![], vec![exposed[1]]]
        );
        assert!(calls.iter().all(|call| call.len() <= 1));
    }

    #[test]
    fn edit_frames_show_damaged_rows_whole_and_leave_untouched_exposed_rows_to_fill() {
        let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 400.0, 2.0);
        viewport.frame(request(0.0, 400.0, 400.0, 2.0)).unwrap();
        let scrolled = viewport.frame(request(300.0, 400.0, 400.0, 2.0)).unwrap();
        assert!(sets(&scrolled).is_empty() && drops(&scrolled).is_empty());
        let before = shown(&viewport);
        let damaged = [(0, [0, 1024, 512, 1200]), (0, [512, 1024, 800, 1200])];
        let untouched = [(1, [0, 0, 512, 512]), (1, [512, 0, 800, 512])];
        let visible = planned_tiles(&viewport, |plan| plan.visible.clone());
        for key in damaged.iter().chain(&untouched) {
            assert!(visible.contains(key));
            assert!(reference(&viewport, *key, 2.0).is_some());
        }

        edit_rows(&viewport, &[(380.0, 540.0)]);
        let edit = viewport.frame(request(300.0, 400.0, 400.0, 2.0)).unwrap();
        assert!(drops(&edit).is_empty());
        assert_eq!(
            sets(&edit).into_keys().collect::<Vec<_>>(),
            damaged.to_vec()
        );
        assert_pixels_match(&viewport, &edit, 2.0);
        assert_rows_whole(&viewport, 2.0);
        let drawn = rastered(&viewport);
        assert!(untouched.iter().all(|key| !drawn.contains(key)));
        let after = shown(&viewport);
        assert!(
            before
                .iter()
                .all(|(key, version)| after.get(key) == Some(version))
        );

        let mut calls = Vec::new();
        drain(&viewport, 0.000_001, |fill| {
            assert_pixels_match(&viewport, fill, 2.0);
            assert_rows_whole(&viewport, 2.0);
            calls.push(sets(fill));
        });
        assert!(
            calls
                .iter()
                .all(|call| damaged.iter().all(|key| !call.contains_key(key)))
        );
        let revealing = calls
            .iter()
            .position(|call| call.contains_key(&untouched[0]))
            .unwrap();
        assert!(
            untouched
                .iter()
                .all(|key| calls[revealing].contains_key(key))
        );
        assert!(calls[..revealing].iter().all(|call| call.is_empty()));
        assert_held_pixels_match(&viewport, 2.0);
    }

    #[test]
    fn growing_edits_show_every_damaged_visible_row_in_the_edit_frame() {
        for scale in [2.0, 3.0] {
            let continuous = LayoutMode::Continuous { max_width: 600 };
            let viewport = open(document(continuous, "alpha", 20), 400.0, 600.0, scale);
            let mut frame = viewport
                .frame(request(1_000_000.0, 400.0, 600.0, scale))
                .unwrap();
            fill_all(&viewport, scale);
            let mut new_rows = 0;
            for _ in 0..30 {
                let previous: BTreeMap<u32, DisplayList> =
                    planned_tiles(&viewport, |plan| plan.visible.clone())
                        .iter()
                        .map(|key| key.0)
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .map(|page| (page, display_list(&viewport, page, scale).0))
                        .collect();
                let rows_before = shown_rows(&viewport);
                let last = *frame.geometry.pages.last().unwrap();
                viewport
                    .editor()
                    .enqueue_request(vec![
                        Message::Selection {
                            op: SelectionOp::SetAt {
                                page: last.index as usize,
                                x: 390.0,
                                y: (last.height - 30.0) as f32,
                            },
                        },
                        Message::Insertion {
                            op: InsertionOp::Break {
                                kind: editor_core::Break::Paragraph,
                            },
                        },
                        Message::Insertion {
                            op: InsertionOp::Text { text: "x".into() },
                        },
                    ])
                    .unwrap();
                frame = viewport
                    .frame(request(1_000_000.0, 400.0, 600.0, scale))
                    .unwrap();
                assert_pixels_match(&viewport, &frame, scale);
                assert_rows_whole(&viewport, scale);

                let visible = planned_tiles(&viewport, |plan| plan.visible.clone());
                let next: BTreeMap<u32, (DisplayList, IRect)> = previous
                    .keys()
                    .filter(|page| visible.iter().any(|key| key.0 == **page))
                    .map(|&page| (page, display_list(&viewport, page, scale)))
                    .collect();
                let damaged_rows: BTreeSet<(u32, i32)> = visible
                    .iter()
                    .filter(|key| {
                        next.get(&key.0).is_some_and(|(list, bounds)| {
                            let damage =
                                editor_renderer::diff::diff(&previous[&key.0], list, *bounds);
                            raster::is_damaged(tile_bounds(**key), &damage)
                        })
                    })
                    .map(|key| (key.0, key.1[1]))
                    .collect();
                let held = shown(&viewport);
                for key in planned_tiles(&viewport, state::PagePlan::required) {
                    if damaged_rows.contains(&(key.0, key.1[1]))
                        && reference(&viewport, key, scale).is_some()
                    {
                        assert!(
                            held.contains_key(&key),
                            "scale {scale}: tile {key:?} of a damaged row is withheld"
                        );
                    }
                }
                new_rows += damaged_rows.difference(&rows_before).count();
                fill_all(&viewport, scale);
            }
            assert!(new_rows > 0, "scale {scale}");
        }
    }

    #[test]
    fn edit_frames_redraw_rekeyed_tiles_the_host_was_showing() {
        let continuous = LayoutMode::Continuous { max_width: 600 };
        let viewport = open(document(continuous, "alpha", 40), 400.0, 600.0, 2.0);
        let first = viewport
            .frame(request(1_000_000.0, 400.0, 600.0, 2.0))
            .unwrap();
        let height = first.geometry.pages[0].height;
        let bottom = ((height * 2.0) as i32 - 1) / 512 * 512;
        let old = (0, [0, bottom, 512, (height * 2.0) as i32]);
        assert!(shown(&viewport).contains_key(&old));

        viewport
            .editor()
            .enqueue_request(vec![
                Message::Selection {
                    op: SelectionOp::SetAt {
                        page: 0,
                        x: 300.0,
                        y: (height - 30.0) as f32,
                    },
                },
                Message::Insertion {
                    op: InsertionOp::Break {
                        kind: editor_core::Break::Paragraph,
                    },
                },
            ])
            .unwrap();
        let edit = viewport
            .frame(request(1_000_000.0, 400.0, 600.0, 2.0))
            .unwrap();
        let grown = edit.geometry.pages[0].height;
        assert!(grown > height);
        let rekeyed = (
            0,
            [0, bottom, 512, ((grown * 2.0) as i32).min(bottom + 512)],
        );
        assert!(drops(&edit).contains(&old));
        assert!(sets(&edit).contains_key(&rekeyed));
        assert_pixels_match(&viewport, &edit, 2.0);
        assert_held_pixels_match(&viewport, 2.0);
    }

    #[test]
    fn width_changes_keep_the_visible_rows_the_host_was_showing() {
        let continuous = LayoutMode::Continuous { max_width: 600 };
        let viewport = open(document(continuous, "alpha", 40), 400.0, 600.0, 2.0);
        viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        fill_all(&viewport, 2.0);
        let row = |key: &TileKey| (key.0, key.1[1]);
        let before: BTreeSet<(u32, i32)> = shown(&viewport).keys().map(row).collect();

        let narrow = viewport.frame(request(0.0, 300.0, 600.0, 2.0)).unwrap();
        assert!(narrow.geometry.tick.is_some());
        assert_eq!(narrow.geometry.pages[0].width, 300.0);
        let visible = planned_tiles(&viewport, |plan| plan.visible.clone());
        let kept: BTreeSet<(u32, i32)> = visible
            .iter()
            .map(row)
            .filter(|row| before.contains(row))
            .collect();
        assert!(!kept.is_empty());
        let after: BTreeSet<(u32, i32)> = shown(&viewport).keys().map(row).collect();
        assert!(kept.is_subset(&after));
        assert!(visible.iter().all(|key| !drops(&narrow).contains(key)));
        assert_pixels_match(&viewport, &narrow, 2.0);
        assert_visible_shown(&viewport, 2.0);
        assert_rows_whole(&viewport, 2.0);
        assert_held_pixels_match(&viewport, 2.0);
    }

    #[test]
    fn edit_frames_redraw_the_visible_tiles_after_a_scale_change() {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 600.0, 2.0);
        viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();

        edit_rows(&viewport, &[(300.0, 40.0)]);
        let edit = viewport.frame(request(0.0, 400.0, 600.0, 3.0)).unwrap();
        assert_eq!(edit.geometry.raster_scale, 3.0);
        assert_eq!(
            sets(&edit).keys().copied().collect::<Vec<_>>(),
            vec![
                (0, [0, 0, 512, 512]),
                (0, [0, 512, 512, 1024]),
                (0, [0, 1024, 512, 1536]),
                (0, [0, 1536, 512, 1800]),
            ]
        );
        assert_pixels_match(&viewport, &edit, 3.0);
        assert_held_pixels_match(&viewport, 3.0);
    }

    #[test]
    fn zero_width_frames_do_not_fix_the_zoom() {
        let viewport = open(document(WIDE_PAGE, "alpha", 200), 400.0, 300.0, 2.0);
        let collapsed = viewport.frame(request(0.0, 0.0, 300.0, 2.0)).unwrap();
        assert_eq!(collapsed.geometry.zoom, 1.0);

        let sized = viewport.frame(request(0.0, 400.0, 300.0, 2.0)).unwrap();
        let expected = editor_viewport::initial_zoom(
            &editor_viewport::LayoutMode::Paginated {
                page_width: 800.0,
                page_height: 1200.0,
                margin_top: 40.0,
                margin_bottom: 40.0,
                margin_left: 40.0,
                margin_right: 40.0,
            },
            400.0,
        );
        assert_eq!(expected, 0.5);
        assert_eq!(sized.geometry.zoom, expected);
    }

    #[test]
    fn half_zoom_frames_raster_and_fill_in_device_pixels() {
        let viewport = open(document(WIDE_PAGE, "alpha", 200), 400.0, 300.0, 2.0);
        let frame = viewport.frame(request(300.0, 400.0, 300.0, 2.0)).unwrap();
        assert_eq!(
            (frame.geometry.zoom, frame.geometry.raster_scale),
            (0.5, 1.0)
        );
        {
            let state = viewport.state.lock().unwrap();
            let plan = state.plan.as_ref().unwrap();
            let visible = |page: usize| {
                plan.pages[&page]
                    .visible
                    .iter()
                    .map(|bounds| FramePxRect::from(*bounds).edges())
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                visible(0),
                vec![
                    [0, 512, 512, 1024],
                    [512, 512, 800, 1024],
                    [0, 1024, 512, 1200],
                    [512, 1024, 800, 1200],
                ]
            );
            assert!(visible(1).is_empty());
        }
        assert_eq!(
            sets(&frame).keys().copied().collect::<Vec<_>>(),
            vec![(0, [0, 512, 512, 1024]), (0, [0, 1024, 512, 1200])]
        );
        assert_pixels_match(&viewport, &frame, 1.0);

        assert_eq!(
            fill_calls(&viewport, 0.0, 1.0),
            vec![
                vec![],
                vec![(1, [0, 0, 512, 512])],
                vec![],
                vec![(0, [0, 0, 512, 512])],
            ]
        );
        assert_held_pixels_match(&viewport, 1.0);
    }

    #[test]
    fn stationary_fills_reveal_the_visible_rows_top_down_one_row_at_a_time() {
        let scale = 2.0;
        let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, scale);
        viewport.frame(request(0.0, 400.0, 600.0, scale)).unwrap();
        let scrolled = viewport.frame(request(700.0, 400.0, 600.0, scale)).unwrap();
        assert!(sets(&scrolled).is_empty());
        let row = |key: &TileKey| (key.0, key.1[1]);
        let visible_rows: Vec<(u32, i32)> = planned_tiles(&viewport, |plan| plan.visible.clone())
            .iter()
            .map(row)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        assert_eq!(visible_rows, vec![(1, 0), (1, 512), (1, 1024), (2, 0)]);

        let mut drawn = rastered(&viewport);
        let mut revealed = Vec::new();
        drain(&viewport, 0.000_001, |fill| {
            assert_pixels_match(&viewport, fill, scale);
            assert_rows_whole(&viewport, scale);
            let now = rastered(&viewport);
            assert_eq!(now.difference(&drawn).count(), 1);
            drawn = now;
            let rows: BTreeSet<(u32, i32)> = sets(fill).keys().map(row).collect();
            assert!(rows.len() <= 1);
            revealed.extend(rows);
        });
        assert_eq!(revealed[..visible_rows.len()], visible_rows);
        assert_visible_shown(&viewport, scale);
    }

    #[test]
    fn fill_paces_margin_tiles_by_the_budget() {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 600.0, 2.0);
        viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        assert_eq!(
            fill_calls(&viewport, 0.000_001, 2.0),
            vec![
                vec![],
                vec![(1, [0, 0, 512, 512])],
                vec![],
                vec![(1, [0, 512, 512, 1024])],
            ]
        );
    }

    #[test]
    fn fill_verifies_visible_pages_before_drawing_visible_rows() {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 400.0, 2.0);
        viewport.frame(request(100.0, 400.0, 400.0, 2.0)).unwrap();
        fill_all(&viewport, 2.0);
        let before = shown(&viewport);
        edit_rows(&viewport, &[(300.0, 40.0), (300.0, 560.0)]);
        let edit = viewport.frame(request(100.0, 400.0, 400.0, 2.0)).unwrap();
        assert!(drops(&edit).contains(&(1, [0, 0, 512, 512])));

        let scrolled = viewport.frame(request(400.0, 400.0, 400.0, 2.0)).unwrap();
        assert!(sets(&scrolled).is_empty());
        assert!(!shown(&viewport).contains_key(&(1, [0, 0, 512, 512])));

        let verified = viewport.fill(0.000_001).unwrap().unwrap();
        assert_eq!(
            sets(&verified),
            BTreeMap::from([((1, [0, 0, 512, 512]), before[&(1, [0, 0, 512, 512])])])
        );
        assert!(verified.geometry.fill_remaining);
        assert_pixels_match(&viewport, &verified, 2.0);
        let redrawn = viewport.fill(0.000_001).unwrap().unwrap();
        let revealed = sets(&redrawn);
        assert_eq!(
            revealed.keys().copied().collect::<Vec<_>>(),
            vec![(0, [0, 1024, 512, 1200])]
        );
        assert!(revealed[&(0, [0, 1024, 512, 1200])] > before[&(0, [0, 1024, 512, 1200])]);
        assert_pixels_match(&viewport, &redrawn, 2.0);
        assert_held_pixels_match(&viewport, 2.0);
    }

    #[test]
    fn fill_keeps_off_screen_verification_on_the_budget() {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 400.0, 2.0);
        viewport.frame(request(700.0, 400.0, 400.0, 2.0)).unwrap();
        fill_all(&viewport, 2.0);
        let before = shown(&viewport);
        let off_screen = [
            (0, [0, 512, 512, 1024]),
            (0, [0, 1024, 512, 1200]),
            (2, [0, 0, 512, 512]),
        ];

        edit_page_rows(&viewport, 1, &[(300.0, 100.0)]);
        let edit = viewport.frame(request(700.0, 400.0, 400.0, 2.0)).unwrap();
        assert_eq!(drops(&edit), BTreeSet::from(off_screen));
        {
            let state = viewport.state.lock().unwrap();
            let plan = state.plan.as_ref().unwrap();
            assert_eq!(
                plan.pages.keys().copied().collect::<Vec<_>>(),
                vec![0, 1, 2]
            );
            assert!(plan.pages[&0].visible.is_empty() && plan.pages[&2].visible.is_empty());
            assert!(
                plan.pages[&1]
                    .visible
                    .iter()
                    .all(|bounds| state.pages[&1].tiles.contains_key(bounds))
            );
        }

        let calls = fill_calls(&viewport, 0.000_001, 2.0);
        assert_eq!(
            calls,
            vec![vec![off_screen[0], off_screen[1]], vec![off_screen[2]]]
        );
        let after = shown(&viewport);
        assert!(off_screen.iter().all(|key| after[key] == before[key]));
    }

    #[test]
    fn fill_redraws_visible_tiles_that_verification_finds_damaged() {
        let viewport = open(document(PAGE, "alpha", 200), 400.0, 600.0, 2.0);
        viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        fill_all(&viewport, 2.0);
        let before = shown(&viewport);
        let hidden = [(1, [0, 0, 512, 512]), (1, [0, 512, 512, 1024])];
        assert!(hidden.iter().all(|key| before.contains_key(key)));

        edit_page_rows(&viewport, 1, &[(300.0, 40.0), (300.0, 300.0)]);
        let edit = viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(hidden.iter().all(|key| drops(&edit).contains(key)));

        let scrolled = viewport.frame(request(500.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(sets(&scrolled).is_empty());
        {
            let state = viewport.state.lock().unwrap();
            let visible: Vec<[i32; 4]> = state.plan.as_ref().unwrap().pages[&1]
                .visible
                .iter()
                .map(|bounds| FramePxRect::from(*bounds).edges())
                .collect();
            assert_eq!(
                visible,
                vec![
                    [0, 0, 512, 512],
                    [512, 0, 800, 512],
                    [0, 512, 512, 1024],
                    [512, 512, 800, 1024],
                ]
            );
            assert!(
                state.plan.as_ref().unwrap().pages[&1]
                    .visible
                    .iter()
                    .all(|bounds| state.pages[&1].tiles.contains_key(bounds))
            );
        }

        let calls: Vec<BTreeMap<TileKey, u64>> = (0..3)
            .map(|_| {
                let frame = viewport.fill(0.000_001).unwrap().unwrap();
                assert_pixels_match(&viewport, &frame, 2.0);
                assert_rows_whole(&viewport, 2.0);
                sets(&frame)
            })
            .collect();
        assert!(calls[0].is_empty());
        for (call, key) in calls[1..].iter().zip(hidden) {
            assert_eq!(call.keys().copied().collect::<Vec<_>>(), vec![key]);
            assert!(call[&key] > before[&key]);
        }
        assert_held_pixels_match(&viewport, 2.0);
    }

    #[test]
    fn tiny_budget_fills_show_every_row_whole() {
        for scale in [2.0, 3.0] {
            let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, scale);
            viewport.frame(request(0.0, 400.0, 600.0, scale)).unwrap();
            assert_rows_whole(&viewport, scale);
            viewport.frame(request(700.0, 400.0, 600.0, scale)).unwrap();
            assert_rows_whole(&viewport, scale);

            let mut held_back = 0;
            let mut drawn = rastered(&viewport);
            drain(&viewport, 0.000_001, |frame| {
                assert_pixels_match(&viewport, frame, scale);
                assert_rows_whole(&viewport, scale);
                held_pixels_match(&viewport, scale);
                let now = rastered(&viewport);
                if now.len() > drawn.len() && sets(frame).is_empty() {
                    held_back += 1;
                }
                drawn = now;
            });
            assert!(held_back > 0, "scale {scale}");
            let required = planned_tiles(&viewport, state::PagePlan::required);
            assert!(required.is_subset(&drawn));
            let held = shown(&viewport);
            for key in required {
                assert_eq!(
                    held.contains_key(&key),
                    reference(&viewport, key, scale).is_some(),
                    "scale {scale} tile {key:?}"
                );
            }
        }
    }

    #[test]
    fn invalidating_one_tile_of_a_shown_row_drops_the_whole_row() {
        let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 400.0, 2.0);
        viewport.frame(request(0.0, 400.0, 400.0, 2.0)).unwrap();
        fill_all(&viewport, 2.0);
        let left = (0, [0, 1024, 512, 1200]);
        let right = (0, [512, 1024, 800, 1200]);
        assert!(
            planned_tiles(&viewport, |plan| plan.visible.clone())
                .iter()
                .all(|key| key.1[1] < 1024)
        );
        let before = shown(&viewport);
        assert!(before.contains_key(&left) && before.contains_key(&right));

        edit_rows(&viewport, &[(380.0, 540.0)]);
        let edit = viewport.frame(request(0.0, 400.0, 400.0, 2.0)).unwrap();
        assert!(sets(&edit).is_empty());
        assert_eq!(drops(&edit), BTreeSet::from([left, right]));
        {
            let state = viewport.state.lock().unwrap();
            let raster = &state.pages[&0];
            assert_eq!(raster.tiles[&tile_bounds(left)].version, before[&left]);
            assert!(raster.stale.contains(&tile_bounds(right)));
        }
        assert_rows_whole(&viewport, 2.0);

        let filled = fill_all(&viewport, 2.0);
        assert_eq!(
            filled.keys().copied().collect::<Vec<_>>(),
            vec![left, right]
        );
        assert_eq!(filled[&left], before[&left]);
        assert!(filled[&right] > before[&right]);
        assert_held_pixels_match(&viewport, 2.0);
        assert_rows_whole(&viewport, 2.0);
    }

    #[test]
    fn deceleration_fills_visible_rows_then_the_row_ahead_then_destination_rows_in_arrival_order() {
        let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, 2.0);
        let first = viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(first.geometry.page_count > 8);
        let frame = viewport
            .frame(ViewportRequest {
                debug: true,
                ..decelerating(700.0, 4000.0, 400.0, 600.0, 2.0)
            })
            .unwrap();
        assert!(sets(&frame).is_empty());
        assert_eq!(
            frame
                .geometry
                .pages
                .iter()
                .map(|page| (page.index, page.y))
                .collect::<Vec<_>>(),
            vec![
                (0, 0.0),
                (1, 624.0),
                (2, 1248.0),
                (3, 1872.0),
                (5, 3120.0),
                (6, 3744.0),
                (7, 4368.0),
            ]
        );
        assert!(frame.geometry.fill_remaining);
        let destination = planned_tiles(&viewport, |plan| plan.destination.clone());
        let visible = planned_tiles(&viewport, |plan| plan.visible.clone());
        let rows = |page: u32, tops: &[i32]| {
            tops.iter()
                .flat_map(move |&y0| {
                    let y1 = (y0 + 512).min(1200);
                    [(page, [0, y0, 512, y1]), (page, [512, y0, 800, y1])]
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            destination,
            [
                rows(5, &[512, 1024]),
                rows(6, &[0, 512, 1024]),
                rows(7, &[0, 512, 1024]),
            ]
            .concat()
            .into_iter()
            .collect::<BTreeSet<_>>()
        );
        assert!(visible.iter().all(|key| !destination.contains(key)));
        let pending: BTreeSet<TileKey> = frame
            .geometry
            .debug
            .clone()
            .unwrap()
            .pending
            .iter()
            .map(|tile| (tile.page, tile.bounds.edges()))
            .collect();
        assert!(destination.is_subset(&pending));
        let page_tops: BTreeMap<u32, f64> = frame
            .geometry
            .pages
            .iter()
            .map(|page| (page.index, page.y))
            .collect();
        let content_top = |key: &TileKey| page_tops[&key.0] + f64::from(key.1[1]) / 2.0;

        let mut drawn = rastered(&viewport);
        let mut order = Vec::new();
        let mut revealed = Vec::new();
        drain(&viewport, 0.000_001, |fill| {
            assert_pixels_match(&viewport, fill, 2.0);
            assert_rows_whole(&viewport, 2.0);
            let now = rastered(&viewport);
            let added: Vec<TileKey> = now.difference(&drawn).copied().collect();
            assert_eq!(added.len(), 1);
            assert!(drawn.is_subset(&now));
            order.push(added[0]);
            revealed.push(sets(fill).into_keys().collect::<Vec<_>>());
            drawn = now;
        });
        let visible_rows = [
            rows(1, &[0]),
            rows(1, &[512]),
            rows(1, &[1024]),
            rows(2, &[0]),
        ];
        let ahead = rows(2, &[512]);
        let arrival = [
            rows(5, &[512]),
            rows(5, &[1024]),
            rows(6, &[0]),
            rows(6, &[512]),
            rows(6, &[1024]),
            rows(7, &[0]),
            rows(7, &[512]),
            rows(7, &[1024]),
        ];
        assert_eq!(
            visible_rows.concat().into_iter().collect::<BTreeSet<_>>(),
            visible
        );
        assert_eq!(order[..8], visible_rows.concat());
        assert_eq!(order[8..10], ahead);
        assert_eq!(order[10..], arrival.concat());
        assert!(
            order[10..]
                .windows(2)
                .all(|pair| content_top(&pair[0]) <= content_top(&pair[1]))
        );
        assert_eq!(
            revealed,
            visible_rows
                .iter()
                .chain(std::iter::once(&ahead))
                .chain(&arrival)
                .flat_map(|row| [Vec::new(), row.clone()])
                .collect::<Vec<_>>()
        );
        assert!(visible.is_subset(&drawn));
        assert_held_pixels_match(&viewport, 2.0);

        let settled = viewport
            .frame(ViewportRequest {
                debug: true,
                ..decelerating(700.0, 4000.0, 400.0, 600.0, 2.0)
            })
            .unwrap();
        assert!(settled.geometry.tiles.is_empty());
        assert!(!settled.geometry.fill_remaining);
        assert!(viewport.fill(f64::INFINITY).unwrap().is_none());
        let pending: BTreeSet<TileKey> = settled
            .geometry
            .debug
            .clone()
            .unwrap()
            .pending
            .iter()
            .map(|tile| (tile.page, tile.bounds.edges()))
            .collect();
        let required = planned_tiles(&viewport, state::PagePlan::required);
        assert_eq!(
            pending,
            required
                .difference(&drawn)
                .copied()
                .collect::<BTreeSet<_>>()
        );
        assert!(pending.is_empty());
    }

    #[test]
    fn destination_fills_reverify_destination_pages_whose_signature_changed() {
        let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, 2.0);
        viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        let prefill = viewport
            .frame(decelerating(700.0, 4000.0, 400.0, 600.0, 2.0))
            .unwrap();
        drain(&viewport, f64::INFINITY, |_| {});
        let ahead: BTreeMap<TileKey, u64> = shown(&viewport)
            .into_iter()
            .filter(|(key, _)| key.0 >= 5)
            .collect();
        assert_eq!(
            ahead.keys().map(|key| key.0).collect::<BTreeSet<_>>(),
            BTreeSet::from([5, 6, 7])
        );

        edit_page_rows(&viewport, 1, &[(380.0, 300.0)]);
        let edit = viewport
            .frame(decelerating(700.0, 4000.0, 400.0, 600.0, 2.0))
            .unwrap();
        assert!(edit.geometry.revision > prefill.geometry.revision);
        let dropped = drops(&edit);
        assert!(ahead.keys().all(|key| dropped.contains(key)));
        assert!(edit.geometry.fill_remaining);

        let mut verified = Vec::new();
        drain(&viewport, 0.000_001, |fill| {
            assert_pixels_match(&viewport, fill, 2.0);
            assert_rows_whole(&viewport, 2.0);
            let pages: BTreeSet<u32> = sets(fill).keys().map(|key| key.0).collect();
            assert!(pages.len() <= 1);
            verified.extend(pages);
        });
        assert_eq!(verified, vec![5, 6, 7, 0]);
        let after = shown(&viewport);
        assert!(
            ahead
                .iter()
                .all(|(key, version)| after.get(key) == Some(version))
        );
        assert_held_pixels_match(&viewport, 2.0);
    }

    #[test]
    fn destination_prefill_covers_every_screen_on_the_approach() {
        for (start, destination) in [(0.0, 4000.0), (4000.0, 0.0)] {
            let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, 2.0);
            viewport.frame(request(start, 400.0, 600.0, 2.0)).unwrap();
            viewport
                .frame(decelerating(start, destination, 400.0, 600.0, 2.0))
                .unwrap();
            drain(&viewport, f64::INFINITY, |_| {});

            let from: f64 = if destination > start {
                start.max(destination - 600.0)
            } else {
                start.min(destination + 600.0)
            };
            for step in 0..=4 {
                let scroll = from + (destination - from) * f64::from(step) / 4.0;
                let frame = viewport
                    .frame(decelerating(scroll, destination, 400.0, 600.0, 2.0))
                    .unwrap();
                assert_pixels_match(&viewport, &frame, 2.0);
                assert_visible_shown(&viewport, 2.0);
                assert_rows_whole(&viewport, 2.0);
            }
        }
    }

    #[test]
    fn destination_frames_keep_the_departure_screen() {
        let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, 2.0);
        viewport.frame(request(0.0, 400.0, 600.0, 2.0)).unwrap();
        fill_all(&viewport, 2.0);
        let departure: BTreeSet<TileKey> = planned_tiles(&viewport, |plan| plan.visible.clone())
            .into_iter()
            .filter(|key| reference(&viewport, *key, 2.0).is_some())
            .collect();
        let before = shown(&viewport);
        assert!(departure.iter().all(|key| before.contains_key(key)));

        let frame = viewport
            .frame(decelerating(0.0, 4000.0, 400.0, 600.0, 2.0))
            .unwrap();
        assert!(drops(&frame).is_empty());
        drain(&viewport, 0.000_001, |fill| {
            assert!(drops(fill).is_empty());
        });
        let after = shown(&viewport);
        assert!(
            departure
                .iter()
                .all(|key| after.get(key) == before.get(key))
        );
    }

    #[test]
    fn leaving_deceleration_drops_destination_pages_and_fills_the_margin_behind() {
        let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, 2.0);
        viewport.frame(request(1400.0, 400.0, 600.0, 2.0)).unwrap();
        viewport
            .frame(decelerating(700.0, 4000.0, 400.0, 600.0, 2.0))
            .unwrap();
        drain(&viewport, f64::INFINITY, |_| {});
        assert!(rastered(&viewport).iter().all(|key| key.0 != 0));
        let ahead: BTreeSet<TileKey> = shown(&viewport)
            .into_keys()
            .filter(|key| key.0 >= 5)
            .collect();
        assert!(!ahead.is_empty());

        let frame = viewport.frame(request(700.0, 400.0, 600.0, 2.0)).unwrap();
        assert!(
            viewport
                .state
                .lock()
                .unwrap()
                .plan
                .as_ref()
                .unwrap()
                .destination
                .is_none()
        );
        assert_eq!(
            frame
                .geometry
                .pages
                .iter()
                .map(|page| page.index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(drops(&frame), ahead);
        assert!(sets(&frame).is_empty());
        assert!(
            viewport
                .state
                .lock()
                .unwrap()
                .pages
                .keys()
                .all(|page| *page <= 3)
        );

        assert!(frame.geometry.fill_remaining);
        assert_visible_shown(&viewport, 2.0);
        let behind = [
            (0, [0, 1024, 512, 1200]),
            (0, [512, 1024, 800, 1200]),
            (0, [0, 512, 512, 1024]),
            (0, [512, 512, 800, 1024]),
        ];
        let mut drawn = rastered(&viewport);
        let mut order = Vec::new();
        drain(&viewport, 0.000_001, |fill| {
            assert_pixels_match(&viewport, fill, 2.0);
            assert_rows_whole(&viewport, 2.0);
            let now = rastered(&viewport);
            order.extend(now.difference(&drawn).copied());
            drawn = now;
        });
        assert_eq!(order, behind.to_vec());
        assert!(behind.iter().all(|key| shown(&viewport).contains_key(key)));

        for unusable in [f64::NAN, f64::INFINITY] {
            let frame = viewport
                .frame(decelerating(700.0, unusable, 400.0, 600.0, 2.0))
                .unwrap();
            assert!(
                viewport
                    .state
                    .lock()
                    .unwrap()
                    .plan
                    .as_ref()
                    .unwrap()
                    .destination
                    .is_none()
            );
            assert!(frame.geometry.tiles.is_empty());
        }
    }

    #[test]
    fn narrow_views_complete_visible_rows_with_their_margin_tiles() {
        let viewport = open(document(PAGE, PANGRAM, 200), 60.0, 600.0, 8.0);
        let first = viewport.frame(request(0.0, 60.0, 600.0, 8.0)).unwrap();
        assert_eq!(
            (first.geometry.zoom, first.geometry.raster_scale),
            (0.25, 2.0)
        );
        let visible = planned_tiles(&viewport, |plan| plan.visible.clone());
        assert!(!visible.is_empty());
        assert!(visible.iter().all(|key| key.1[0] == 0));
        let row_mates: BTreeSet<TileKey> = visible
            .iter()
            .map(|key| (key.0, [512, key.1[1], 800, key.1[3]]))
            .collect();
        assert!(row_mates.is_subset(&planned_tiles(&viewport, state::PagePlan::required)));
        assert_eq!(
            sets(&first).into_keys().collect::<BTreeSet<_>>(),
            visible.union(&row_mates).copied().collect()
        );
        assert_pixels_match(&viewport, &first, 2.0);
        assert_rows_whole(&viewport, 2.0);

        let scrolled = viewport.frame(request(300.0, 60.0, 600.0, 8.0)).unwrap();
        assert!(sets(&scrolled).is_empty());
        let visible = planned_tiles(&viewport, |plan| plan.visible.clone());
        assert!(visible.iter().all(|key| key.1[0] == 0));
        let mut drawn = rastered(&viewport);
        let rows_first: Vec<TileKey> = visible
            .iter()
            .flat_map(|key| [*key, (key.0, [512, key.1[1], 800, key.1[3]])])
            .filter(|key| !drawn.contains(key))
            .collect();
        assert!(!rows_first.is_empty());
        let mut order = Vec::new();
        drain(&viewport, 0.000_001, |fill| {
            assert_pixels_match(&viewport, fill, 2.0);
            assert_rows_whole(&viewport, 2.0);
            let now = rastered(&viewport);
            order.extend(now.difference(&drawn).copied());
            drawn = now;
        });
        assert_eq!(order[..rows_first.len()], rows_first);
        let shown_now = shown(&viewport);
        for key in &visible {
            assert!(shown_now.contains_key(key), "tile {key:?}");
            assert!(
                shown_now.contains_key(&(key.0, [512, key.1[1], 800, key.1[3]])),
                "row mate of {key:?}"
            );
        }
        assert_rows_whole(&viewport, 2.0);
    }

    #[test]
    fn sideways_moves_on_a_page_wider_than_the_view_keep_the_shown_rows() {
        let scale = 2.0;
        let viewport = open(document(WIDE_PAGE, PANGRAM, 200), 800.0, 600.0, scale);
        let landscape = viewport.frame(request(0.0, 800.0, 600.0, scale)).unwrap();
        assert_eq!(landscape.geometry.zoom, 1.0);
        let at = |scroll_x: f64, destination: Option<f64>| ViewportRequest {
            scroll_x,
            destination,
            ..request(0.0, 400.0, 600.0, scale)
        };
        let portrait = viewport.frame(at(0.0, None)).unwrap();
        assert_eq!(
            (portrait.geometry.zoom, portrait.geometry.content_width),
            (1.0, 800.0)
        );
        fill_all(&viewport, scale);
        assert_visible_shown(&viewport, scale);

        let mut columns = BTreeSet::new();
        let steps = (1..=30)
            .map(|step| (f64::from(step) * 13.0, None))
            .chain((0..=30).map(|step| (390.0 - f64::from(step) * 13.0, Some(4000.0))));
        for (index, (scroll_x, destination)) in steps.enumerate() {
            if index == 30 {
                fill_all(&viewport, scale);
                assert_visible_shown(&viewport, scale);
            }
            let rows_before = shown_rows(&viewport);
            let frame = viewport.frame(at(scroll_x, destination)).unwrap();
            assert_pixels_match(&viewport, &frame, scale);
            assert_rows_whole(&viewport, scale);
            assert_shown_rows_kept(&viewport, &rows_before, scale);
            columns.insert(
                planned_tiles(&viewport, |plan| plan.visible.clone())
                    .iter()
                    .map(|key| key.1[0])
                    .collect::<BTreeSet<_>>(),
            );
            if let Some(fill) = viewport.fill(0.000_001).unwrap() {
                assert_pixels_match(&viewport, &fill, scale);
                assert_rows_whole(&viewport, scale);
            }
        }
        assert!(columns.len() >= 3);
        drain(&viewport, 0.000_001, |fill| {
            assert_pixels_match(&viewport, fill, scale);
            assert_rows_whole(&viewport, scale);
        });
        assert_visible_shown(&viewport, scale);
    }

    #[test]
    fn destination_prefill_rows_span_a_wide_page_and_stay_shown_on_arrival() {
        let scale = 2.0;
        let viewport = open(document(WIDE_PAGE, PANGRAM, 200), 800.0, 600.0, scale);
        viewport.frame(request(0.0, 800.0, 600.0, scale)).unwrap();
        let at = |scroll_y: f64, destination: Option<f64>| ViewportRequest {
            scroll_x: 390.0,
            destination,
            ..request(scroll_y, 400.0, 600.0, scale)
        };
        let portrait = viewport.frame(at(0.0, None)).unwrap();
        assert_eq!(
            (portrait.geometry.zoom, portrait.geometry.content_width),
            (1.0, 800.0)
        );
        fill_all(&viewport, scale);

        viewport.frame(at(0.0, Some(4000.0))).unwrap();
        drain(&viewport, 0.000_001, |fill| {
            assert_pixels_match(&viewport, fill, scale);
            assert_rows_whole(&viewport, scale);
        });
        let destination = planned_tiles(&viewport, |plan| plan.destination.clone());
        assert!(destination.is_subset(&rastered(&viewport)));
        let mut rows: BTreeMap<(u32, i32), BTreeSet<i32>> = BTreeMap::new();
        for key in &destination {
            rows.entry((key.0, key.1[1])).or_default().insert(key.1[0]);
        }
        assert!(rows.len() >= 8);
        for ((page, y0), columns) in &rows {
            assert_eq!(
                columns,
                &BTreeSet::from([0, 512, 1024, 1536]),
                "page {page} row {y0}"
            );
        }

        for scroll_y in [3400.0, 3550.0, 3700.0, 3850.0, 4000.0] {
            let rows_before = shown_rows(&viewport);
            let frame = viewport.frame(at(scroll_y, Some(4000.0))).unwrap();
            assert_pixels_match(&viewport, &frame, scale);
            assert_rows_whole(&viewport, scale);
            assert_shown_rows_kept(&viewport, &rows_before, scale);
            assert_visible_shown(&viewport, scale);
        }
    }

    #[test]
    fn decelerating_fills_visible_rows_then_rows_ahead_then_arrival_rows_in_either_direction() {
        let scale = 3.0;
        for (start, destination, rows) in [(1400.0, 4000.0, 2), (3000.0, 0.0, 1), (3130.0, 0.0, 3)]
        {
            let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, scale);
            viewport.frame(request(0.0, 400.0, 600.0, scale)).unwrap();
            let frame = viewport
                .frame(decelerating(start, destination, 400.0, 600.0, scale))
                .unwrap();
            assert!(frame.geometry.fill_remaining);
            let tops: BTreeMap<u32, f64> = frame
                .geometry
                .pages
                .iter()
                .map(|page| (page.index, page.y))
                .collect();
            let span = |key: &TileKey| {
                (
                    tops[&key.0] + f64::from(key.1[1]) / scale,
                    tops[&key.0] + f64::from(key.1[3]) / scale,
                )
            };
            let down = destination > start;
            let ahead = |key: &TileKey| {
                let (top, bottom) = span(key);
                if down {
                    top >= start + 600.0
                } else {
                    bottom <= start
                }
            };
            let middle = start + 300.0;
            let mut leading: Vec<TileKey> = planned_tiles(&viewport, |plan| plan.margin.clone())
                .into_iter()
                .filter(ahead)
                .collect();
            leading.sort_by(|first, second| {
                let distance = |key: &TileKey| {
                    let (top, bottom) = span(key);
                    ((top + bottom) / 2.0 - middle).abs()
                };
                distance(first).total_cmp(&distance(second)).then_with(|| {
                    (first.0, first.1[1], first.1[0]).cmp(&(second.0, second.1[1], second.1[0]))
                })
            });
            assert_eq!(
                leading
                    .iter()
                    .map(|key| (key.0, key.1[1]))
                    .collect::<BTreeSet<_>>()
                    .len(),
                rows,
                "start {start}"
            );
            let prefill = planned_tiles(&viewport, |plan| plan.destination.clone());
            assert!(leading.iter().all(|key| !prefill.contains(key)));
            let mut drawn = rastered(&viewport);
            let visible_rows: BTreeSet<(u32, i32)> =
                planned_tiles(&viewport, |plan| plan.visible.clone())
                    .iter()
                    .map(|key| (key.0, key.1[1]))
                    .collect();
            let mut reading: Vec<TileKey> = planned_tiles(&viewport, state::PagePlan::required)
                .into_iter()
                .filter(|key| visible_rows.contains(&(key.0, key.1[1])) && !drawn.contains(key))
                .collect();
            reading.sort_by_key(|key| (key.0, key.1[1], key.1[0]));
            assert!(!reading.is_empty());
            let mut arrival: Vec<TileKey> = prefill
                .iter()
                .filter(|key| {
                    !visible_rows.contains(&(key.0, key.1[1]))
                        && !leading.contains(key)
                        && !drawn.contains(key)
                })
                .copied()
                .collect();
            arrival.sort_by_key(|key| {
                if down {
                    (key.0 as i64, i64::from(key.1[1]), key.1[0])
                } else {
                    (-(key.0 as i64), -i64::from(key.1[1]), key.1[0])
                }
            });
            assert!(!arrival.is_empty());

            let mut order = Vec::new();
            drain(&viewport, 0.000_001, |fill| {
                assert_pixels_match(&viewport, fill, scale);
                assert_rows_whole(&viewport, scale);
                let now = rastered(&viewport);
                order.extend(now.difference(&drawn).copied());
                drawn = now;
            });
            let expected: Vec<TileKey> = reading
                .iter()
                .chain(&leading)
                .chain(&arrival)
                .copied()
                .collect();
            assert_eq!(order, expected, "start {start}");
        }
    }

    #[test]
    fn decelerating_slower_than_a_row_per_frame_shows_each_entering_row_on_arrival() {
        let scale = 2.0;
        let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, scale);
        viewport.frame(request(0.0, 400.0, 600.0, scale)).unwrap();
        fill_all(&viewport, scale);
        let row = |key: &TileKey| (key.0, key.1[1]);
        let visible_rows = || -> BTreeSet<(u32, i32)> {
            planned_tiles(&viewport, |plan| plan.visible.clone())
                .iter()
                .map(row)
                .collect()
        };
        let mut entering = 0;
        for step in 1..=40 {
            let scroll = f64::from(step) * 60.0;
            let before = visible_rows();
            let frame = viewport
                .frame(decelerating(scroll, 5500.0, 400.0, 600.0, scale))
                .unwrap();
            assert_pixels_match(&viewport, &frame, scale);
            assert_rows_whole(&viewport, scale);
            let held = shown(&viewport);
            for key in planned_tiles(&viewport, |plan| plan.visible.clone()) {
                if before.contains(&row(&key)) || reference(&viewport, key, scale).is_none() {
                    continue;
                }
                entering += 1;
                assert!(
                    held.contains_key(&key),
                    "tile {key:?} entered blank at scroll {scroll}"
                );
            }
            if let Some(fill) = viewport.fill(0.000_001).unwrap() {
                assert_pixels_match(&viewport, &fill, scale);
                assert_rows_whole(&viewport, scale);
            }
        }
        assert!(entering >= 16);

        for step in 1..=3 {
            let scroll = 2400.0 + f64::from(step) * 700.0;
            let frame = viewport
                .frame(decelerating(scroll, 5500.0, 400.0, 600.0, scale))
                .unwrap();
            assert_pixels_match(&viewport, &frame, scale);
            assert_rows_whole(&viewport, scale);
            if let Some(fill) = viewport.fill(0.000_001).unwrap() {
                assert_pixels_match(&viewport, &fill, scale);
                assert_rows_whole(&viewport, scale);
            }
        }
        held_pixels_match(&viewport, scale);
    }

    #[test]
    fn deceleration_reverifies_pages_ahead_whose_signature_changed() {
        let scale = 2.0;
        let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, scale);
        viewport
            .frame(request(1172.0, 400.0, 600.0, scale))
            .unwrap();
        fill_all(&viewport, scale);
        let prefill = viewport
            .frame(decelerating(1172.0, 4000.0, 400.0, 600.0, scale))
            .unwrap();
        drain(&viewport, f64::INFINITY, |_| {});
        let visible_pages: BTreeSet<u32> = planned_tiles(&viewport, |plan| plan.visible.clone())
            .iter()
            .map(|key| key.0)
            .collect();
        assert_eq!(visible_pages, BTreeSet::from([1, 2]));
        let ahead: BTreeMap<TileKey, u64> = shown(&viewport)
            .into_iter()
            .filter(|(key, _)| key.0 == 3)
            .collect();
        assert!(!ahead.is_empty());

        edit_page_rows(&viewport, 1, &[(380.0, 300.0)]);
        let edit = viewport
            .frame(decelerating(1172.0, 4000.0, 400.0, 600.0, scale))
            .unwrap();
        assert!(edit.geometry.revision > prefill.geometry.revision);
        let dropped = drops(&edit);
        assert!(ahead.keys().all(|key| dropped.contains(key)));

        let fill = viewport.fill(0.000_001).unwrap().unwrap();
        assert_eq!(sets(&fill), ahead);
        assert_pixels_match(&viewport, &fill, scale);
        assert_rows_whole(&viewport, scale);
    }

    #[test]
    fn nearby_stops_fill_the_visible_rows_first_top_down() {
        let scale = 2.0;
        for destination in [2300.0, 2900.0] {
            let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, scale);
            viewport.frame(request(0.0, 400.0, 600.0, scale)).unwrap();
            viewport
                .frame(decelerating(2000.0, destination, 400.0, 600.0, scale))
                .unwrap();
            let row = |key: &TileKey| (key.0, key.1[1]);
            let visible = planned_tiles(&viewport, |plan| plan.visible.clone());
            let visible_rows: BTreeSet<(u32, i32)> = visible.iter().map(row).collect();
            let prefill = planned_tiles(&viewport, |plan| plan.destination.clone());
            assert!(
                prefill.iter().any(|key| visible_rows.contains(&row(key))),
                "destination {destination}"
            );
            let mut drawn = rastered(&viewport);
            let mut reading: Vec<TileKey> = planned_tiles(&viewport, state::PagePlan::required)
                .into_iter()
                .filter(|key| visible_rows.contains(&row(key)))
                .collect();
            reading.sort_by_key(|key| (key.0, key.1[1], key.1[0]));
            assert!(reading.iter().all(|key| !drawn.contains(key)));

            let mut order = Vec::new();
            let mut revealed = Vec::new();
            drain(&viewport, 0.000_001, |fill| {
                assert_pixels_match(&viewport, fill, scale);
                assert_rows_whole(&viewport, scale);
                let now = rastered(&viewport);
                order.extend(now.difference(&drawn).copied());
                drawn = now;
                let rows: BTreeSet<(u32, i32)> = sets(fill).keys().map(row).collect();
                assert!(rows.len() <= 1);
                revealed.extend(rows);
            });
            assert_eq!(order[..reading.len()], reading, "destination {destination}");
            assert_eq!(
                revealed[..visible_rows.len()],
                visible_rows.iter().copied().collect::<Vec<_>>(),
                "destination {destination}"
            );
            assert_visible_shown(&viewport, scale);

            let rows_before = shown_rows(&viewport);
            let stop = viewport
                .frame(request(2000.0, 400.0, 600.0, scale))
                .unwrap();
            assert!(sets(&stop).is_empty());
            assert_shown_rows_kept(&viewport, &rows_before, scale);
            assert_visible_shown(&viewport, scale);
            assert_rows_whole(&viewport, scale);
        }
    }

    #[test]
    fn resting_deceleration_orders_the_fill_like_the_normal_mode() {
        let scale = 2.0;
        let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, scale);
        viewport.frame(request(0.0, 400.0, 600.0, scale)).unwrap();
        let order = |request: ViewportRequest| {
            viewport.frame(request).unwrap();
            let pages: Vec<usize> = viewport
                .state
                .lock()
                .unwrap()
                .plan
                .as_ref()
                .unwrap()
                .pages
                .keys()
                .copied()
                .collect();
            let signatures: BTreeMap<usize, u64> = viewport
                .editor()
                .with_viewport_core(|core| {
                    Ok(pages
                        .iter()
                        .map(|&page| (page, core.page_render_signature(page as u32)))
                        .collect())
                })
                .unwrap();
            viewport
                .state
                .lock()
                .unwrap()
                .fill_tasks(&signatures)
                .iter()
                .map(|task| (task.page, task.bounds))
                .collect::<Vec<_>>()
        };
        let normal = order(request(2000.0, 400.0, 600.0, scale));
        let resting = order(decelerating(2000.0, 2000.0, 400.0, 600.0, scale));
        assert!(!normal.is_empty());
        assert_eq!(resting, normal);
    }

    #[test]
    fn deceleration_toward_the_current_position_has_no_rows_ahead() {
        let scale = 2.0;
        let viewport = open(document(PAGE, PANGRAM, 200), 400.0, 600.0, scale);
        viewport.frame(request(0.0, 400.0, 600.0, scale)).unwrap();
        let tasks = |scroll: f64, destination: f64| {
            let frame = viewport
                .frame(decelerating(scroll, destination, 400.0, 600.0, scale))
                .unwrap();
            let tops: BTreeMap<usize, f64> = frame
                .geometry
                .pages
                .iter()
                .map(|page| (page.index as usize, page.y))
                .collect();
            let signatures: BTreeMap<usize, u64> = viewport
                .editor()
                .with_viewport_core(|core| {
                    Ok(tops
                        .keys()
                        .map(|&page| (page, core.page_render_signature(page as u32)))
                        .collect())
                })
                .unwrap();
            let state = viewport.state.lock().unwrap();
            (state.fill_tasks(&signatures), tops)
        };
        let classes = |tasks: &[editor_viewport::FillTask]| {
            tasks.iter().map(|task| task.class).collect::<BTreeSet<_>>()
        };
        let still = BTreeSet::from([
            editor_viewport::FillClass::VisibleMissing,
            editor_viewport::FillClass::Destination,
        ]);
        assert!(
            classes(&tasks(700.0, 4000.0).0).contains(&editor_viewport::FillClass::MarginMissing)
        );
        assert_eq!(classes(&tasks(700.0, 700.0).0), still);

        let (resting, tops) = tasks(2000.0, 2000.0);
        assert_eq!(classes(&resting), still);
        let centers: Vec<f64> = resting
            .iter()
            .filter(|task| task.class == editor_viewport::FillClass::Destination)
            .map(|task| {
                tops[&task.page] + f64::from(task.bounds.y0 + task.bounds.y1) / (2.0 * scale)
            })
            .collect();
        assert!(centers.iter().any(|center| *center < 2000.0));
        assert!(centers.iter().any(|center| *center > 2600.0));
        assert!(centers.windows(2).any(|pair| pair[0] > pair[1]));
        assert!(
            centers
                .windows(2)
                .all(|pair| (pair[0] - 2300.0).abs() <= (pair[1] - 2300.0).abs())
        );
    }

    #[derive(Clone, Debug)]
    enum Step {
        Frame {
            scroll_x: f64,
            scroll: f64,
            destination: Option<f64>,
        },
        Fill {
            budget: f64,
        },
        Edit {
            page: usize,
            x: f32,
            y: f32,
        },
    }

    fn step() -> impl proptest::strategy::Strategy<Value = Step> {
        use proptest::prelude::*;
        prop_oneof![
            3 => (0.0..450.0f64, 0.0..6000.0f64, proptest::option::of(-500.0..7000.0f64))
                .prop_map(|(scroll_x, scroll, destination)| Step::Frame { scroll_x, scroll, destination }),
            3 => proptest::sample::select(vec![0.0, 0.000_001, 1.0, f64::INFINITY])
                .prop_map(|budget| Step::Fill { budget }),
            1 => (0usize..4, proptest::sample::select(vec![21.0f32, 200.0, 380.0]), 30.0f32..570.0)
                .prop_map(|(page, x, y)| Step::Edit { page, x, y }),
        ]
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(24))]

        #[test]
        fn host_rows_stay_whole_and_current(
            scale in proptest::sample::select(vec![2.0, 3.0]),
            wide in proptest::bool::ANY,
            steps in proptest::collection::vec(step(), 1..10),
        ) {
            let (layout, landscape) = if wide { (WIDE_PAGE, 800.0) } else { (PAGE, 400.0) };
            let viewport = open(document(layout, PANGRAM, 200), landscape, 600.0, scale);
            viewport.frame(request(0.0, landscape, 600.0, scale)).unwrap();
            viewport.frame(request(0.0, 400.0, 600.0, scale)).unwrap();
            assert_rows_whole(&viewport, scale);
            for step in steps {
                match step {
                    Step::Frame { scroll_x, scroll, destination } => {
                        let rows_before = shown_rows(&viewport);
                        let frame = viewport
                            .frame(ViewportRequest {
                                scroll_x,
                                destination,
                                ..request(scroll, 400.0, 600.0, scale)
                            })
                            .unwrap();
                        assert_pixels_match(&viewport, &frame, scale);
                        assert_shown_rows_kept(&viewport, &rows_before, scale);
                    }
                    Step::Fill { budget } => {
                        let Some(frame) = viewport.fill(budget).unwrap() else {
                            continue;
                        };
                        assert_pixels_match(&viewport, &frame, scale);
                    }
                    Step::Edit { page, x, y } => {
                        edit_page_rows(&viewport, page, &[(x, y)]);
                        continue;
                    }
                }
                assert_rows_whole(&viewport, scale);
                held_pixels_match(&viewport, scale);
            }
        }
    }
}
