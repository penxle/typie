use crate::error::{Result, VermudaError};
use crate::window_delegate::{VermudaWindowDelegate, WindowDelegateHandle, WindowEvent};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSWindow, NSWindowCollectionBehavior,
    NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use objc2_virtualization::{VZVirtualMachine, VZVirtualMachineView};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, mpsc};

pub struct DisplayWindow {
    window: Retained<NSWindow>,
    _vm_view: Retained<VZVirtualMachineView>,
    is_visible: Arc<AtomicBool>,
    window_delegate_handle: Arc<Mutex<WindowDelegateHandle>>,
}

unsafe impl Send for DisplayWindow {}
unsafe impl Sync for DisplayWindow {}

impl DisplayWindow {
    pub fn new(vm: &VZVirtualMachine, width: f64, height: f64) -> Result<Self> {
        let frame = NSRect::new(NSPoint::new(100.0, 100.0), NSSize::new(width, height));
        let vm_view = Self::build_vm_view(vm, frame)?;
        let window = Self::build_window(frame, &vm_view)?;

        let (delegate_handle, event_tx) = WindowDelegateHandle::new();
        Self::attach_delegate(&window, event_tx)?;

        Ok(Self {
            window: window.clone(),
            _vm_view: vm_view,
            is_visible: Arc::new(AtomicBool::new(false)),
            window_delegate_handle: Arc::new(Mutex::new(delegate_handle)),
        })
    }

    pub fn show(&self) {
        if let Some(mtm) = MainThreadMarker::new() {
            let app = NSApplication::sharedApplication(mtm);
            app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
            app.activate();
        }

        self.window.makeKeyAndOrderFront(None);
        self.window.makeKeyWindow();
        self.window.makeMainWindow();
        self.window.setIsVisible(true);
        self.window.orderFrontRegardless();
        self.window.setLevel(NS_NORMAL_WINDOW_LEVEL);

        self.is_visible.store(true, Ordering::SeqCst);
    }

    pub fn close(&self) {
        self.window.close();
        self.is_visible.store(false, Ordering::SeqCst);
    }

    pub fn get_window(&self) -> Retained<NSWindow> {
        self.window.clone()
    }

    pub fn spawn_event_handler(&self) {
        let delegate_handle = self.window_delegate_handle.clone();
        let is_visible = self.is_visible.clone();

        tokio::spawn(async move {
            let mut handle = delegate_handle.lock().await;

            while let Some(event) = handle.wait_for_event().await {
                match event {
                    WindowEvent::WillClose => {
                        is_visible.store(false, Ordering::SeqCst);
                    }
                    _ => {}
                }
            }
        });
    }

    fn build_vm_view(
        vm: &VZVirtualMachine,
        frame: NSRect,
    ) -> Result<Retained<VZVirtualMachineView>> {
        let mtm = main_thread_marker()?;
        unsafe {
            let view =
                VZVirtualMachineView::initWithFrame(mtm.alloc::<VZVirtualMachineView>(), frame);
            view.setVirtualMachine(Some(vm));
            view.setCapturesSystemKeys(true);
            Ok(view)
        }
    }

    fn build_window(
        frame: NSRect,
        vm_view: &Retained<VZVirtualMachineView>,
    ) -> Result<Retained<NSWindow>> {
        let mtm = main_thread_marker()?;
        unsafe {
            let style = NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Miniaturizable
                | NSWindowStyleMask::Resizable;

            let window = NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc::<NSWindow>(),
                frame,
                style,
                objc2_app_kit::NSBackingStoreType::Buffered,
                false,
            );

            window.setTitle(&NSString::from_str("Vermuda VM"));
            window.setContentView(Some(vm_view));
            window.center();
            window.setReleasedWhenClosed(false);
            window.setLevel(NS_FLOATING_WINDOW_LEVEL);
            window.setCollectionBehavior(
                NSWindowCollectionBehavior::Managed
                    | NSWindowCollectionBehavior::ParticipatesInCycle
                    | NSWindowCollectionBehavior::FullScreenPrimary,
            );

            Ok(window)
        }
    }

    fn attach_delegate(
        window: &Retained<NSWindow>,
        event_tx: mpsc::UnboundedSender<WindowEvent>,
    ) -> Result<()> {
        let mtm = main_thread_marker()?;
        let delegate = VermudaWindowDelegate::new(mtm, event_tx)?;

        let protocol_object = ProtocolObject::from_retained(delegate);
        window.setDelegate(Some(&protocol_object));

        Ok(())
    }
}

const NS_FLOATING_WINDOW_LEVEL: isize = 3; // NSFloatingWindowLevel
const NS_NORMAL_WINDOW_LEVEL: isize = 0; // NSNormalWindowLevel

fn main_thread_marker() -> Result<MainThreadMarker> {
    MainThreadMarker::new().ok_or_else(|| {
        VermudaError::operation_failed("Display window must be created on main thread")
    })
}
