use std::ffi::c_void;
use std::ptr::NonNull;

use amml_msvc::Optional;
use levels_core::structs::draw::CameraView;

use crate::game::{BaseActorRenderer, FontReader, NameTagCollection, ScreenContext, TextObjectArgs};
use crate::services::{LabelRing, LabelText, NameTagList, Overlay};
use crate::types::{DrawOutcome, DrawSetup, LevelsEntry, LevelsFeature, WidthSource};

// appends the overlay's labels to the game's name-tag list. After the game made its tags, the drawer checks the list
// layout, copies the game's tags into a ring slot, adds one tag object per label made by the game's own
// `_extractRenderTextObject`, and repoints the list at the slot. It never writes to the list when the layout is not
// the expected one or when there are no labels
pub struct LabelDrawer {
    ring: LabelRing,
    text: LabelText,
    // the address of the renderer that owns the materials; made once and never freed
    renderer: Option<usize>,
    renderer_failed: bool,
    // the list layout was wrong: no more drawing for the session
    broken: bool,
}

impl LabelDrawer {
    pub fn new(allocate: unsafe extern "C" fn(usize) -> *mut u8) -> Self {
        Self {
            ring: LabelRing::new(allocate),
            text: LabelText::new(),
            renderer: None,
            renderer_failed: false,
            broken: false,
        }
    }

    // the bytes the ring and the texts hold, for telemetry
    pub fn resident_bytes(&self) -> u64 {
        self.ring.resident_bytes()
    }

    pub fn is_broken(&self) -> bool {
        self.broken
    }

    pub fn text(&self) -> &LabelText {
        &self.text
    }

    /// appends the labels of this frame to the list in `tags`
    ///
    /// # Safety
    /// `tags` must be a live `NameTagRenderObjectCollection` that the game just made, `setup.screen` the live
    /// ScreenContext of the same call, and the heap functions the game's. The call must be on the game thread
    #[allow(unsafe_code)]
    pub unsafe fn append(
        &mut self,
        setup: &DrawSetup<'_>,
        overlay: &mut Overlay,
        camera: Option<CameraView>,
        tags: *mut c_void,
    ) -> DrawOutcome {
        if self.broken {
            return DrawOutcome::Skipped;
        }
        // SAFETY: the caller guarantees a live collection
        let Some(collection) = (unsafe { NameTagCollection::from_raw(tags, setup.entries) }) else {
            return DrawOutcome::Skipped;
        };
        let mut list = match NameTagList::open(collection) {
            Ok(list) => list,
            Err(fault) => {
                self.broken = true;
                curia::error!("@alaydriem/levels stops drawing: the name-tag list layout is not the one this build expects", {
                    reason: fault.to_string()
                });
                return DrawOutcome::Skipped;
            }
        };
        let view = overlay.draw(list.len(), camera);
        if view.labels.is_empty() {
            return DrawOutcome::NoLabels;
        }
        let Some(client) = setup.client else { return DrawOutcome::Skipped };
        if !self.prepare_text(setup, client) {
            return DrawOutcome::Skipped;
        }
        // SAFETY: the caller guarantees the heap functions and the thread
        let Some(renderer_address) = (unsafe { self.renderer(setup) }) else { return DrawOutcome::Skipped };
        // SAFETY: the address came from `create` for this game build
        let Some(renderer) = (unsafe { BaseActorRenderer::from_address(renderer_address) }) else {
            return DrawOutcome::Skipped;
        };
        // SAFETY: the caller guarantees a live ScreenContext
        let tessellator = unsafe { ScreenContext::tessellator(setup.screen, setup.entries) };
        let Some(tessellator) = tessellator else {
            return DrawOutcome::Skipped;
        };
        let extract_ready =
            setup.entries.get(LevelsEntry::EXTRACT_RENDER_TEXT_OBJECT).is_some_and(|address| address != 0);
        if !extract_ready {
            return DrawOutcome::Skipped;
        }
        // a slot is counted by `move_to` before it is built, so every label must have its text first
        if !self.text.covers(view.labels.iter().map(|label| label.value)) {
            return DrawOutcome::Skipped;
        }
        let count = view.labels.len();
        let element_size = list.element_size();
        let Some(bytes) = (list.len() + count).checked_mul(element_size) else { return DrawOutcome::Skipped };
        let Some(storage) = self.ring.next(bytes) else { return DrawOutcome::Skipped };
        // SAFETY: the ring slot holds `bytes` bytes, is apart from the game's storage, and is never freed
        let first_new = unsafe { list.move_to(storage, count) };
        let rotation = Optional::some(view.rotation);
        for (index, label) in view.labels.iter().enumerate() {
            // `covers` held for every label above, and the texts have not changed since
            let (Some(text), Some(widths)) = (self.text.text(label.value), self.text.widths(label.value)) else {
                continue;
            };
            let args = TextObjectArgs {
                tessellator,
                text,
                widths,
                position: [label.x, label.y, label.z],
                text_color: label.color,
                tag_color: [0.0; 4],
                scale: view.scale,
                rotation: &rotation,
            };
            // SAFETY: slot `index` of the new storage is room for one object; the arguments outlive the call
            unsafe {
                renderer.extract_render_text_object(setup.entries, first_new.wrapping_add(index * element_size), &args)
            };
        }
        DrawOutcome::Drew(count)
    }

    // the sixteen texts and widths, made on the first frame with labels. True when they are ready
    #[allow(unsafe_code)]
    fn prepare_text(&mut self, setup: &DrawSetup<'_>, client: NonNull<c_void>) -> bool {
        if self.text.is_ready() {
            return true;
        }
        let font_on = LevelsFeature::is_on(setup.entries, LevelsFeature::FONT);
        // SAFETY: the heap functions are the game's; the client is live for this call
        let source = unsafe {
            self.text.prepare(setup.heap, |texts| {
                if font_on { FontReader::measure(setup.entries, client.as_ptr(), texts) } else { None }
            })
        };
        match source {
            Some(WidthSource::FixedTable) => {
                curia::warn!("label widths come from a fixed table, not the game font", { feature: LevelsFeature::FONT });
                true
            }
            Some(WidthSource::Font) => true,
            None => {
                curia::error!("the label texts could not be made; no labels are drawn");
                false
            }
        }
    }

    /// the address of the renderer that owns the depth-tested materials, made on the first frame that needs it,
    /// from the name-tag hook or the render-level hook, both under the state lock. A failed make is tried once
    ///
    /// # Safety
    /// The heap functions of `setup` must be the game's
    #[allow(unsafe_code)]
    pub unsafe fn renderer(&mut self, setup: &DrawSetup<'_>) -> Option<usize> {
        if let Some(address) = self.renderer {
            return Some(address);
        }
        if self.renderer_failed {
            return None;
        }
        // SAFETY: the caller guarantees the heap allocator and the thread
        let made = unsafe { BaseActorRenderer::create(setup.entries, setup.allocate) };
        match made {
            Some(renderer) => {
                self.renderer = Some(renderer.as_address());
                self.renderer
            }
            None => {
                self.renderer_failed = true;
                curia::error!("the name-tag renderer could not be made; no labels are drawn");
                None
            }
        }
    }
}
