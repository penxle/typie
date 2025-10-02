use crate::app_delegate::VermudaAppDelegate;
use crate::config::{VmConfig, VmContext};
use crate::display::DisplayWindow;
use crate::error::{Result, VermudaError};
use crate::main_thread::run_on_main;
use crate::vm_delegate::{DelegateHandle, VmDelegate, VmEvent};
use block2::RcBlock;
use log::{error, info, warn};
use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSError;
use objc2_virtualization::VZVirtualMachine;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmState {
    Created,
    Running,
    Stopped,
    Error,
}

#[repr(transparent)]
struct MainThreadRetained<T>(Retained<T>);

unsafe impl<T> Send for MainThreadRetained<T> {}

impl<T> MainThreadRetained<T> {
    fn new(value: Retained<T>) -> Self {
        Self(value)
    }

    fn into_inner(self) -> Retained<T> {
        self.0
    }
}

impl<T> std::ops::Deref for MainThreadRetained<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct VmInstance {
    vm: Option<Retained<VZVirtualMachine>>,
    state: Arc<Mutex<VmState>>,
    delegate_handle: Arc<Mutex<DelegateHandle>>,
    display_window: Option<DisplayWindow>,
    shutdown_tx: broadcast::Sender<()>,
    config: VmConfig,
    _context: Arc<VmContext>,
}

impl VmInstance {
    pub async fn new(config: VmConfig) -> Result<Self> {
        let context = Arc::new(Self::prepare_context(&config)?);
        let vm = Self::create_virtual_machine(config.clone(), context.clone()).await?;

        let (delegate_handle, event_tx) = DelegateHandle::new();
        let (shutdown_tx, _) = broadcast::channel(8);

        let mut instance = Self {
            vm: Some(vm),
            state: Arc::new(Mutex::new(VmState::Created)),
            delegate_handle: Arc::new(Mutex::new(delegate_handle)),
            display_window: None,
            shutdown_tx,
            config,
            _context: context,
        };

        instance.setup_delegate(event_tx).await?;
        instance.spawn_event_handler();

        info!("VM instance created successfully");
        Ok(instance)
    }

    async fn setup_delegate(&mut self, event_tx: mpsc::UnboundedSender<VmEvent>) -> Result<()> {
        let delegate = VmDelegate::new(event_tx)?;

        if let Some(vm) = &self.vm {
            let vm_for_delegate = MainThreadRetained::new(vm.clone());
            let delegate_for_vm = MainThreadRetained::new(delegate.clone());

            run_on_main(move |_| -> Result<()> {
                let vm = vm_for_delegate.into_inner();
                let delegate = delegate_for_vm.into_inner();

                unsafe {
                    let protocol_object = ProtocolObject::from_retained(delegate.clone());
                    vm.setDelegate(Some(&protocol_object));
                }

                Ok(())
            })
            .await?;
        }

        self.delegate_handle.lock().await.set_delegate(delegate);

        Ok(())
    }

    fn spawn_event_handler(&self) {
        let state = self.state.clone();
        let delegate_handle = self.delegate_handle.clone();
        let shutdown_tx = self.shutdown_tx.clone();

        tokio::spawn(async move {
            let mut handle = delegate_handle.lock().await;

            while let Some(event) = handle.wait_for_event().await {
                match event {
                    VmEvent::GuestDidStop => {
                        info!("VM stopped gracefully");
                        *state.lock().await = VmState::Stopped;
                        let _ = shutdown_tx.send(());
                    }
                    VmEvent::DidStopWithError(err) => {
                        error!("VM error: {}", err);
                        *state.lock().await = VmState::Error;
                        let _ = shutdown_tx.send(());
                    }
                    VmEvent::NetworkDisconnected(reason) => {
                        warn!("Network attachment disconnected: {}", reason);
                    }
                }
            }
        });
    }

    pub fn subscribe_guest_shutdown(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    pub async fn start(&mut self) -> Result<()> {
        let current = *self.state.lock().await;
        if current != VmState::Created && current != VmState::Stopped {
            return Ok(());
        }

        let vm = self
            .vm
            .as_ref()
            .ok_or_else(|| VermudaError::resource_unavailable("VM not available"))?;

        let (tx, rx) = oneshot::channel();
        let sender = Arc::new(StdMutex::new(Some(tx)));
        let sender_for_block = Arc::clone(&sender);
        let vm_for_start = MainThreadRetained::new(vm.clone());

        run_on_main(move |_| -> Result<()> {
            let vm = vm_for_start.into_inner();

            unsafe {
                if !vm.canStart() {
                    return Err(VermudaError::validation_failed(
                        "VM cannot start with current configuration",
                    ));
                }
            }

            let callback = RcBlock::new(move |error: *mut NSError| {
                let result = if error.is_null() {
                    Ok(())
                } else {
                    let err = unsafe { &*error };
                    let code = err.code();
                    let domain = err.domain().to_string();
                    let msg = err.localizedDescription().to_string();

                    error!("VM start error:");
                    error!("  Domain: {}", domain);
                    error!("  Code: {}", code);
                    error!("  Message: {}", msg);

                    if domain == "VZErrorDomain" {
                        match code {
                            1 => error!("  Hint: Invalid configuration - check VM settings"),
                            2 => error!("  Hint: Internal virtualization error"),
                            3 => error!("  Hint: No virtualization support available"),
                            _ => {}
                        }
                    }

                    Err(VermudaError::virtualization(msg))
                };

                match sender_for_block.lock() {
                    Ok(mut guard) => {
                        if let Some(tx) = guard.take() {
                            let _ = tx.send(result);
                        }
                    }
                    Err(_) => {
                        error!("Failed to lock VM start completion sender");
                    }
                }
            });

            unsafe {
                vm.startWithCompletionHandler(&callback);
            }

            Ok(())
        })
        .await?;

        match rx.await {
            Ok(result) => result?,
            Err(_) => {
                return Err(VermudaError::operation_failed(
                    "VM start channel closed unexpectedly",
                ));
            }
        }

        *self.state.lock().await = VmState::Running;
        info!("VM is now running");

        if let Some(display) = self.config.display() {
            if let Some(vm_ref) = self.vm.as_ref() {
                let vm_for_window = MainThreadRetained::new(vm_ref.clone());
                let width = display.width as f64;
                let height = display.height as f64;

                if let Ok(window) = run_on_main(move |_| {
                    let vm = vm_for_window.into_inner();
                    let window = DisplayWindow::new(&vm, width, height)?;
                    window.show();
                    VermudaAppDelegate::with_global(|delegate| {
                        delegate.set_main_window(window.get_window());
                    });
                    Ok::<_, VermudaError>(window)
                })
                .await
                {
                    window.spawn_event_handler();
                    self.display_window = Some(window);
                }
            }
        }

        Ok(())
    }

    pub async fn request_shutdown(&mut self) -> Result<()> {
        let current = *self.state.lock().await;
        if current == VmState::Stopped {
            return Ok(());
        }

        let vm = self
            .vm
            .as_ref()
            .ok_or_else(|| VermudaError::resource_unavailable("VM not available"))?
            .clone();
        let vm = MainThreadRetained::new(vm);

        run_on_main(move |_| unsafe {
            let vm = vm.into_inner();
            if vm.canRequestStop() {
                vm.requestStopWithError().map_err(|e| {
                    VermudaError::virtualization(format!("Failed to request shutdown: {:?}", e))
                })
            } else {
                Err(VermudaError::operation_failed(
                    "VM cannot accept shutdown request in current state",
                ))
            }
        })
        .await?;

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        match shutdown_rx.recv().await {
            Ok(()) => Ok(()),
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                Err(VermudaError::operation_failed("Shutdown channel closed"))
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => Ok(()),
        }
    }

    pub async fn force_stop(&mut self) -> Result<()> {
        if let Some(window) = self.display_window.take() {
            run_on_main(move |_| {
                window.close();
            })
            .await;
        }

        let current = *self.state.lock().await;
        if current == VmState::Stopped {
            return Ok(());
        }

        let vm = self
            .vm
            .as_ref()
            .ok_or_else(|| VermudaError::resource_unavailable("VM not available"))?;

        let vm_for_stop = MainThreadRetained::new(vm.clone());
        let (tx, rx) = oneshot::channel();
        let sender = Arc::new(StdMutex::new(Some(tx)));
        let sender_for_block = Arc::clone(&sender);

        run_on_main(move |_| {
            let vm = vm_for_stop.into_inner();

            let callback = RcBlock::new(move |error: *mut NSError| {
                let result = if error.is_null() {
                    Ok(())
                } else {
                    let msg = unsafe { &*error }.localizedDescription().to_string();
                    Err(VermudaError::virtualization(msg))
                };

                match sender_for_block.lock() {
                    Ok(mut guard) => {
                        if let Some(tx) = guard.take() {
                            let _ = tx.send(result);
                        }
                    }
                    Err(_) => {
                        error!("Failed to lock VM stop completion sender");
                    }
                }
            });

            unsafe {
                vm.stopWithCompletionHandler(&callback);
            }

            Ok::<_, VermudaError>(())
        })
        .await?;

        match rx.await {
            Ok(Ok(())) => {
                *self.state.lock().await = VmState::Stopped;
                Ok(())
            }
            Ok(Err(e)) => {
                *self.state.lock().await = VmState::Error;
                Err(e)
            }
            Err(_) => Err(VermudaError::operation_failed("Failed to stop VM")),
        }
    }
}

impl VmInstance {
    fn prepare_context(config: &VmConfig) -> Result<VmContext> {
        let mut context = VmContext::new();

        if let Some(network) = config.network() {
            use vmnet::mode::{self, Mode};
            let vmnet_mode = Mode::Bridged(mode::bridged::Bridged {
                shared_interface_name: network.interface.clone(),
            });

            let attachment = crate::vmnet::VmnetAttachment::new(vmnet_mode)?;
            context = context.with_vmnet(attachment);
        }

        Ok(context)
    }

    async fn create_virtual_machine(
        config: VmConfig,
        context: Arc<VmContext>,
    ) -> Result<Retained<VZVirtualMachine>> {
        let vm = run_on_main(move |_| -> Result<MainThreadRetained<VZVirtualMachine>> {
            let platform_config = config.to_platform_config(&context)?;

            let vm = unsafe {
                let vm = VZVirtualMachine::initWithConfiguration(
                    VZVirtualMachine::alloc(),
                    &platform_config,
                );

                if !vm.canStart() {
                    return Err(VermudaError::virtualization(
                        "VM cannot be started with current configuration",
                    ));
                }

                vm
            };

            Ok(MainThreadRetained::new(vm))
        })
        .await?
        .into_inner();

        Ok(vm)
    }
}
