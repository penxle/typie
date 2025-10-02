use crate::error::Result;
use objc2::rc::Retained;
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSApplicationDelegate, NSWindow};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::oneshot;

#[derive(Default)]
struct AppState {
    window: Option<Retained<NSWindow>>,
    delegate: Option<Retained<VermudaAppDelegate>>,
    exit_tx: Option<oneshot::Sender<()>>,
}

thread_local! {
    static STATE: RefCell<AppState> = RefCell::new(AppState::default());
}

static TERMINATING: AtomicBool = AtomicBool::new(false);

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "VermudaAppDelegate"]
    pub struct VermudaAppDelegate;

    unsafe impl NSObjectProtocol for VermudaAppDelegate {}

    unsafe impl NSApplicationDelegate for VermudaAppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn application_did_finish_launching(&self, _notification: &NSObject) {}

        #[unsafe(method(applicationDidBecomeActive:))]
        fn application_did_become_active(&self, _notification: &NSObject) {
            Self::with_window(Self::promote_window);
        }

        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn application_should_handle_reopen(
            &self,
            _app: &NSApplication,
            has_visible_windows: bool,
        ) -> bool {
            Self::with_window(|window| {
                if !has_visible_windows {
                    Self::promote_window(window);
                    return;
                }

                if window.isMiniaturized() {
                    unsafe {
                        window.deminiaturize(None);
                    }
                }

                Self::promote_window(window);
            });

            true
        }

        #[unsafe(method(applicationWillTerminate:))]
        fn application_will_terminate(&self, _notification: &NSObject) {}

        #[unsafe(method(applicationShouldTerminate:))]
        fn application_should_terminate(
            &self,
            _sender: &NSApplication,
        ) -> objc2_foundation::NSInteger {
            let was_terminating = Self::set_terminating();

            if !was_terminating {
                Self::send_exit_signal();
                2 // NSTerminateLater
            } else {
                1 // NSTerminateNow
            }
        }

        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn application_should_terminate_after_last_window_closed(
            &self,
            _sender: &NSApplication,
        ) -> bool {
            false
        }
    }
);

impl VermudaAppDelegate {
    pub fn new(mtm: MainThreadMarker) -> Result<Retained<Self>> {
        let delegate: Retained<Self> = unsafe { msg_send![VermudaAppDelegate::alloc(mtm), init] };
        Ok(delegate)
    }

    pub fn register_global(delegate: &Retained<Self>) {
        STATE.with(|state| {
            state.borrow_mut().delegate = Some(delegate.clone());
        });
    }

    pub fn with_global<F, R>(f: F) -> Option<R>
    where
        F: FnOnce(&Self) -> R,
    {
        STATE.with(|state| {
            let delegate = state.borrow().delegate.clone();
            delegate.map(|delegate| f(delegate.as_ref()))
        })
    }

    pub fn set_main_window(&self, window: Retained<NSWindow>) {
        STATE.with(|state| {
            state.borrow_mut().window = Some(window);
        });
    }

    pub fn set_exit_sender(tx: oneshot::Sender<()>) {
        STATE.with(|state| {
            state.borrow_mut().exit_tx = Some(tx);
        });
    }

    pub fn set_terminating() -> bool {
        TERMINATING.swap(true, Ordering::AcqRel)
    }

    fn send_exit_signal() {
        STATE.with(|state| {
            if let Some(tx) = state.borrow_mut().exit_tx.take() {
                let _ = tx.send(());
            }
        });
    }

    fn with_window<F>(f: F)
    where
        F: FnOnce(&NSWindow),
    {
        STATE.with(|state| {
            let window = state.borrow().window.clone();
            if let Some(window) = window {
                f(window.as_ref());
            }
        });
    }

    fn promote_window(window: &NSWindow) {
        unsafe {
            window.makeKeyAndOrderFront(None);
            window.setIsVisible(true);
            window.orderFrontRegardless();
        }
    }
}
