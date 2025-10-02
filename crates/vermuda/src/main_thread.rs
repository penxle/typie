use dispatch2::DispatchQueue;
use objc2_foundation::MainThreadMarker;
use tokio::sync::oneshot;

pub async fn run_on_main<F, R>(operation: F) -> R
where
    F: FnOnce(MainThreadMarker) -> R + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = oneshot::channel();

    DispatchQueue::main().exec_async(move || {
        let mtm =
            MainThreadMarker::new().expect("Dispatch main queue executed off the main thread");
        let result = operation(mtm);
        let _ = tx.send(result);
    });

    rx.await.expect("Main thread task was cancelled")
}
