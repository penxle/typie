mod app_delegate;
mod config;
mod disk;
mod display;
mod error;
mod main_thread;
mod vm;
mod vm_delegate;
mod vmnet;
mod window_delegate;

use anyhow::{Context, Result, anyhow};
use app_delegate::VermudaAppDelegate;
use config::VmConfig;
use disk::DiskImage;
use error::VermudaError;
use log::{error, info, warn};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadOnly, sel};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSProcessInfo, NSString};
use std::path::PathBuf;
use std::process;
use std::thread;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::oneshot;
use vm::VmInstance;

use crate::main_thread::run_on_main;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mtm = MainThreadMarker::new().expect("Application must start on the main thread");

    let (app_exit_tx, app_exit_rx) = oneshot::channel();
    let (delegate, app) = setup_appkit(mtm)?;

    let _delegate_guard = delegate;

    VermudaAppDelegate::set_exit_sender(app_exit_tx);
    let runtime_handle = spawn_runtime_thread(app_exit_rx);

    app.run();

    match runtime_handle.join() {
        Ok(result) => result?,
        Err(panic) => {
            error!("Runtime thread panicked!");
            if let Some(msg) = panic.downcast_ref::<&str>() {
                return Err(anyhow!("Tokio runtime panicked: {}", msg));
            }
            if let Some(msg) = panic.downcast_ref::<String>() {
                return Err(anyhow!("Tokio runtime panicked: {}", msg));
            }
            Err(anyhow!("Tokio runtime panicked with unknown payload"))?
        }
    }

    Ok(())
}

fn setup_appkit(
    mtm: MainThreadMarker,
) -> Result<(Retained<VermudaAppDelegate>, Retained<NSApplication>)> {
    set_process_name();

    let delegate = VermudaAppDelegate::new(mtm)?;
    VermudaAppDelegate::register_global(&delegate);

    let app = NSApplication::sharedApplication(mtm);
    configure_menu(&app, mtm);
    attach_delegate(&app, &delegate);

    unsafe {
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        app.activate();
    }

    Ok((delegate, app))
}

fn spawn_runtime_thread(app_exit_rx: oneshot::Receiver<()>) -> thread::JoinHandle<Result<()>> {
    thread::spawn(move || {
        let runtime = RuntimeBuilder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .thread_name("vermuda-runtime")
            .build()
            .context("Failed to create Tokio runtime")?;

        let result = runtime.block_on(run_runtime_tasks(app_exit_rx));

        if let Err(ref err) = result {
            error!("Runtime task failed: {}", err);
        }

        result
    })
}

async fn run_runtime_tasks(app_exit_rx: oneshot::Receiver<()>) -> Result<()> {
    info!("Initializing VM environment");

    let disk_path = PathBuf::from("disk.img");
    let disk = DiskImage::new(&disk_path, 10.0);
    disk.ensure_exists()?;

    let iso_path = PathBuf::from("metal-arm64.iso");
    if !iso_path.exists() {
        return Err(anyhow!("Boot ISO not found: {}", iso_path.display()));
    }

    let boot_path = PathBuf::from("../vermuda-boot/vermuda-boot.img");
    if !boot_path.exists() {
        return Err(anyhow!("Boot image not found: {}", boot_path.display()));
    }

    info!("Building VM configuration (4 CPUs, 4GB RAM)");
    let builder = VmConfig::builder()
        .cpu_count(4)
        .memory_gb(4.0)
        .with_boot(boot_path)
        .with_root(disk.path().to_path_buf(), disk.size_gb())
        .with_iso(iso_path)
        .with_network("en0".into(), "52:54:00:12:34:56".into())
        .with_display(1920, 1080, 144);

    let config = builder.build()?;

    info!("Creating VM instance");
    let mut vm = VmInstance::new(config).await?;
    let mut shutdown_events = vm.subscribe_guest_shutdown();

    info!("Starting VM");
    vm.start().await?;

    #[derive(Debug)]
    enum ShutdownTrigger {
        CtrlC,
        AppExit,
        GuestStop,
        SignalStreamClosed,
    }

    let mut sigint_stream =
        signal(SignalKind::interrupt()).context("Failed to create SIGINT stream")?;

    let mut app_exit_rx = Some(app_exit_rx);

    let trigger = tokio::select! {
        _ = async { app_exit_rx.as_mut().unwrap().await }, if app_exit_rx.is_some() => {
            ShutdownTrigger::AppExit
        },
        shutdown = shutdown_events.recv() => match shutdown {
            Ok(()) => ShutdownTrigger::GuestStop,
            Err(_) => ShutdownTrigger::SignalStreamClosed,
        },
        signal = sigint_stream.recv() => match signal {
            Some(_) => ShutdownTrigger::CtrlC,
            None => ShutdownTrigger::SignalStreamClosed,
        }
    };

    info!("Shutting down (trigger: {:?})", trigger);

    match trigger {
        ShutdownTrigger::CtrlC => {
            info!("Press Ctrl-C again to force quit");
        }
        _ => {}
    }

    let shutdown_result = match trigger {
        ShutdownTrigger::GuestStop => Ok(()),
        ShutdownTrigger::AppExit => vm.force_stop().await,
        _ => {
            tokio::select! {
                result = vm.request_shutdown() => result,
                signal = sigint_stream.recv() => {
                    if signal.is_some() {
                        warn!("Force shutdown initiated");
                        Err(VermudaError::operation_failed("User forced shutdown"))
                    } else {
                        Err(VermudaError::operation_failed("Signal stream closed"))
                    }
                }
            }
        }
    };

    if let Err(e) = shutdown_result {
        if matches!(e, VermudaError::OperationFailed(ref msg) if msg == "User forced shutdown") {
            process::exit(130);
        }

        error!("Graceful shutdown failed: {}", e);
        if let Err(force_err) = vm.force_stop().await {
            error!("Force stop also failed: {}", force_err);
        }
    }

    match trigger {
        ShutdownTrigger::AppExit => {
            run_on_main(|mtm| {
                let app = NSApplication::sharedApplication(mtm);
                unsafe {
                    app.replyToApplicationShouldTerminate(true);
                }
            })
            .await;
        }
        ShutdownTrigger::CtrlC
        | ShutdownTrigger::GuestStop
        | ShutdownTrigger::SignalStreamClosed => {
            run_on_main(|mtm| {
                if !VermudaAppDelegate::set_terminating() {
                    let app = NSApplication::sharedApplication(mtm);
                    unsafe {
                        app.terminate(None);
                    }
                }
            })
            .await;
        }
    }

    Ok(())
}

const APP_NAME: &str = "Vermuda";

fn set_process_name() {
    let process_info = NSProcessInfo::processInfo();
    process_info.setProcessName(&NSString::from_str(APP_NAME));
}

fn configure_menu(app: &Retained<NSApplication>, mtm: MainThreadMarker) {
    let menu_bar = NSMenu::new(mtm);
    let app_menu_item = NSMenuItem::new(mtm);
    let app_menu =
        unsafe { NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(APP_NAME)) };

    let quit_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str("Quit"),
            Some(sel!(terminate:)),
            &NSString::from_str("q"),
        )
    };
    app_menu.addItem(&quit_item);

    app_menu_item.setSubmenu(Some(&app_menu));
    menu_bar.addItem(&app_menu_item);

    app.setMainMenu(Some(&menu_bar));
}

fn attach_delegate(app: &Retained<NSApplication>, delegate: &Retained<VermudaAppDelegate>) {
    let protocol_object = ProtocolObject::from_retained(delegate.clone());
    app.setDelegate(Some(&protocol_object));
}
