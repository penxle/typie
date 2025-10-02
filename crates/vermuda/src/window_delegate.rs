use crate::error::Result;
use objc2::rc::Retained;
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSWindow, NSWindowDelegate};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol};
use std::cell::RefCell;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum WindowEvent {
    WillClose,
    WillMiniaturize,
    DidMiniaturize,
    DidDeminiaturize,
    DidBecomeKey,
    DidResignKey,
    DidResize,
}

thread_local! {
    static EVENT_SENDER: RefCell<Option<mpsc::UnboundedSender<WindowEvent>>> = RefCell::new(None);
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "VermudaWindowDelegate"]
    pub struct VermudaWindowDelegate;

    unsafe impl NSObjectProtocol for VermudaWindowDelegate {}

    unsafe impl NSWindowDelegate for VermudaWindowDelegate {
        #[unsafe(method(windowShouldClose:))]
        fn window_should_close(&self, _window: &NSWindow) -> bool {
            true
        }

        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &NSObject) {
            Self::send_event(WindowEvent::WillClose);

            if let Some(mtm) = MainThreadMarker::new() {
                let app = NSApplication::sharedApplication(mtm);
                app.hide(None);
            }
        }

        #[unsafe(method(windowWillMiniaturize:))]
        fn window_will_miniaturize(&self, _notification: &NSObject) {
            Self::send_event(WindowEvent::WillMiniaturize);
        }

        #[unsafe(method(windowDidMiniaturize:))]
        fn window_did_miniaturize(&self, _notification: &NSObject) {
            Self::send_event(WindowEvent::DidMiniaturize);
        }

        #[unsafe(method(windowDidDeminiaturize:))]
        fn window_did_deminiaturize(&self, _notification: &NSObject) {
            Self::send_event(WindowEvent::DidDeminiaturize);
        }

        #[unsafe(method(windowDidBecomeKey:))]
        fn window_did_become_key(&self, _notification: &NSObject) {
            Self::send_event(WindowEvent::DidBecomeKey);
        }

        #[unsafe(method(windowDidResignKey:))]
        fn window_did_resign_key(&self, _notification: &NSObject) {
            Self::send_event(WindowEvent::DidResignKey);
        }

        #[unsafe(method(windowDidResize:))]
        fn window_did_resize(&self, _notification: &NSObject) {
            Self::send_event(WindowEvent::DidResize);
        }
    }
);

impl VermudaWindowDelegate {
    pub fn new(
        mtm: MainThreadMarker,
        event_tx: mpsc::UnboundedSender<WindowEvent>,
    ) -> Result<Retained<Self>> {
        EVENT_SENDER.with(|sender| {
            *sender.borrow_mut() = Some(event_tx);
        });

        let delegate: Retained<Self> =
            unsafe { msg_send![VermudaWindowDelegate::alloc(mtm), init] };
        Ok(delegate)
    }
}

pub struct WindowDelegateHandle {
    event_rx: mpsc::UnboundedReceiver<WindowEvent>,
}

impl WindowDelegateHandle {
    pub fn new() -> (Self, mpsc::UnboundedSender<WindowEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();

        (Self { event_rx: rx }, tx)
    }

    pub async fn wait_for_event(&mut self) -> Option<WindowEvent> {
        self.event_rx.recv().await
    }
}

impl VermudaWindowDelegate {
    fn send_event(event: WindowEvent) {
        EVENT_SENDER.with(|sender| {
            if let Some(tx) = sender.borrow().as_ref() {
                let _ = tx.send(event);
            }
        });
    }
}
