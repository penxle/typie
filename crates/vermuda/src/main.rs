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
use clap::Parser;
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

#[derive(Parser, Debug)]
#[command(name = "vermuda")]
struct Args {
    #[arg(long)]
    iso: Option<PathBuf>,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    let mtm = MainThreadMarker::new().expect("Application must start on the main thread");

    let (app_exit_tx, app_exit_rx) = oneshot::channel();
    let (delegate, app) = setup_appkit(mtm)?;

    let _delegate_guard = delegate;

    VermudaAppDelegate::set_exit_sender(app_exit_tx);
    let runtime_handle = spawn_runtime_thread(app_exit_rx, args);

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

    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    app.activate();

    Ok((delegate, app))
}

fn spawn_runtime_thread(
    app_exit_rx: oneshot::Receiver<()>,
    args: Args,
) -> thread::JoinHandle<Result<()>> {
    thread::spawn(move || {
        let runtime = RuntimeBuilder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .thread_name("vermuda-runtime")
            .build()
            .context("Failed to create Tokio runtime")?;

        let result = runtime.block_on(run_runtime_tasks(app_exit_rx, args));

        if let Err(ref err) = result {
            error!("Runtime task failed: {}", err);
        }

        result
    })
}

async fn run_runtime_tasks(app_exit_rx: oneshot::Receiver<()>, args: Args) -> Result<()> {
    info!("Initializing VM environment");

    let initialize = async move {
        let vm_home = config::get_vm_home()?;
        info!("VM_HOME: {}", vm_home.display());

        std::fs::create_dir_all(&vm_home).with_context(|| {
            format!("Failed to create VM_HOME directory: {}", vm_home.display())
        })?;

        let config_path = config::get_config_path()?;
        info!("Loading VM configuration from {}", config_path.display());
        let mut config = VmConfig::load()?;

        if let Some(iso) = args.iso {
            info!("ISO path overridden from CLI: {}", iso.display());
            config = config.with_iso_override(Some(iso));
        }

        if let Some(root) = config.root() {
            let disk_path = root.get_path()?;
            let disk = DiskImage::new(&disk_path, root.size);
            disk.ensure_exists()?;
        }

        Ok::<VmConfig, anyhow::Error>(config)
    };

    let config = match initialize.await {
        Ok(config) => config,
        Err(err) => {
            error!("Initialization failed: {}", err);
            process::exit(1);
        }
    };

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
                app.replyToApplicationShouldTerminate(true);
            })
            .await;
        }
        ShutdownTrigger::CtrlC
        | ShutdownTrigger::GuestStop
        | ShutdownTrigger::SignalStreamClosed => {
            run_on_main(|mtm| {
                if !VermudaAppDelegate::set_terminating() {
                    let app = NSApplication::sharedApplication(mtm);
                    app.terminate(None);
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
    let app_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(APP_NAME));

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
