use crate::error::Result;
use log::error;
use objc2::rc::Retained;
use objc2::{AllocAnyThread, define_class, msg_send};
use objc2_foundation::{NSError, NSObject, NSObjectProtocol};
use objc2_virtualization::{VZNetworkDevice, VZVirtualMachine, VZVirtualMachineDelegate};
use std::sync::Mutex;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum VmEvent {
    GuestDidStop,
    DidStopWithError(String),
    NetworkDisconnected(String),
}

static EVENT_SENDER: Mutex<Option<mpsc::UnboundedSender<VmEvent>>> = Mutex::new(None);

define_class!(
    #[unsafe(super = NSObject)]
    #[name = "VmDelegate"]
    pub struct VmDelegate;

    unsafe impl NSObjectProtocol for VmDelegate {}

    unsafe impl VZVirtualMachineDelegate for VmDelegate {
        #[unsafe(method(guestDidStopVirtualMachine:))]
        fn guest_did_stop_virtual_machine(&self, _vm: &VZVirtualMachine) {
            dispatch_event(VmEvent::GuestDidStop);
        }

        #[unsafe(method(virtualMachine:didStopWithError:))]
        fn virtual_machine_did_stop_with_error(&self, _vm: &VZVirtualMachine, err: &NSError) {
            let error_msg = err.localizedDescription().to_string();
            error!("VM stopped with error: {}", error_msg);

            dispatch_event(VmEvent::DidStopWithError(error_msg));
        }

        #[unsafe(method(virtualMachine:networkDevice:attachmentWasDisconnectedWithError:))]
        fn virtual_machine_network_device_attachment_was_disconnected_with_error(
            &self,
            _vm: &VZVirtualMachine,
            _network_device: &VZNetworkDevice,
            err: &NSError,
        ) {
            let error_msg = err.localizedDescription().to_string();
            dispatch_event(VmEvent::NetworkDisconnected(error_msg));
        }
    }
);

impl VmDelegate {
    pub fn new(event_tx: mpsc::UnboundedSender<VmEvent>) -> Result<Retained<Self>> {
        install_sender(event_tx)?;
        let delegate: Retained<Self> = unsafe { msg_send![VmDelegate::alloc(), init] };
        Ok(delegate)
    }
}

fn install_sender(sender: mpsc::UnboundedSender<VmEvent>) -> Result<()> {
    if let Ok(mut slot) = EVENT_SENDER.lock() {
        *slot = Some(sender);
        Ok(())
    } else {
        Err(crate::error::VermudaError::operation_failed(
            "Failed to initialise VM delegate sender",
        ))
    }
}

fn dispatch_event(event: VmEvent) {
    let sender = match EVENT_SENDER.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };

    if let Some(tx) = sender {
        let _ = tx.send(event);
    }
}

pub struct DelegateHandle {
    delegate: Option<Retained<VmDelegate>>,
    event_rx: mpsc::UnboundedReceiver<VmEvent>,
}

impl DelegateHandle {
    pub fn new() -> (Self, mpsc::UnboundedSender<VmEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();

        (
            Self {
                delegate: None,
                event_rx: rx,
            },
            tx,
        )
    }

    pub fn set_delegate(&mut self, delegate: Retained<VmDelegate>) {
        self.delegate = Some(delegate);
    }

    pub async fn wait_for_event(&mut self) -> Option<VmEvent> {
        self.event_rx.recv().await
    }
}
