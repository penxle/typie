use crate::error::{Result, VermudaError};
use log::info;
use nix::sys::socket::{AddressFamily, SockFlag, SockType, socketpair};
use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_foundation::NSFileHandle;
use std::io;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use vmnet::{Interface, Options};

const MTU: usize = 1500;
const PACKET_BUFFER_SIZE: usize = MTU + 128;
const MAX_ENOBUFS_RETRIES: usize = 100;

pub struct VmnetInterface {
    interface: Arc<std::sync::Mutex<Interface>>,
}

impl std::fmt::Debug for VmnetInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VmnetInterface").finish()
    }
}

unsafe impl Send for VmnetInterface {}
unsafe impl Sync for VmnetInterface {}

impl VmnetInterface {
    pub fn new(mode: vmnet::mode::Mode) -> Result<Self> {
        info!("Initializing vmnet interface");
        let interface = Interface::new(mode, Options::default()).map_err(|e| {
            VermudaError::operation_failed(format!("Failed to create vmnet interface: {:?}", e))
        })?;

        Ok(Self {
            interface: Arc::new(std::sync::Mutex::new(interface)),
        })
    }

    pub fn read_packet(&self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut interface = self.interface.lock().unwrap();

        match interface.read(buffer) {
            Ok(size) => Ok(size),
            Err(vmnet::Error::VmnetReadNothing) => Ok(0),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, format!("{:?}", e))),
        }
    }

    pub fn write_packet(&self, buffer: &[u8]) -> io::Result<usize> {
        let mut interface = self.interface.lock().unwrap();

        match interface.write(buffer) {
            Ok(size) => Ok(size),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, format!("{:?}", e))),
        }
    }
}

pub struct VmnetBridge {
    vmnet_interface: Arc<VmnetInterface>,
    _vz_socket_owned: OwnedFd,
    host_socket_fd: OwnedFd,
    shutdown: Arc<AtomicBool>,
    threads: Vec<thread::JoinHandle<()>>,
}

impl std::fmt::Debug for VmnetBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VmnetBridge")
            .field("shutdown", &self.shutdown.load(Ordering::Relaxed))
            .finish()
    }
}

impl VmnetBridge {
    pub fn new(mode: vmnet::mode::Mode) -> Result<(Self, Retained<NSFileHandle>)> {
        let vmnet_interface = Arc::new(VmnetInterface::new(mode)?);

        let (host_fd, vz_fd) = socketpair(
            AddressFamily::Unix,
            SockType::Datagram,
            None,
            SockFlag::empty(),
        )
        .map_err(|e| {
            VermudaError::operation_failed(format!("Failed to create socketpair: {}", e))
        })?;

        let host_fd_raw = host_fd.as_raw_fd();
        let vz_fd_raw = vz_fd.as_raw_fd();

        let sndbuf_size = 1 * 1024 * 1024;
        let rcvbuf_size = 4 * 1024 * 1024;
        unsafe {
            libc::setsockopt(
                host_fd_raw,
                libc::SOL_SOCKET,
                libc::SO_SNDBUF,
                &sndbuf_size as *const _ as *const libc::c_void,
                std::mem::size_of::<i32>() as libc::socklen_t,
            );
            libc::setsockopt(
                vz_fd_raw,
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &rcvbuf_size as *const _ as *const libc::c_void,
                std::mem::size_of::<i32>() as libc::socklen_t,
            );
            libc::setsockopt(
                host_fd_raw,
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &rcvbuf_size as *const _ as *const libc::c_void,
                std::mem::size_of::<i32>() as libc::socklen_t,
            );

            let flags = libc::fcntl(host_fd_raw, libc::F_GETFL, 0);
            libc::fcntl(host_fd_raw, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }

        let vz_fd_for_nshandle = unsafe { libc::dup(vz_fd_raw) };
        if vz_fd_for_nshandle < 0 {
            return Err(VermudaError::operation_failed("Failed to dup vz_fd"));
        }

        let vz_ns_handle =
            NSFileHandle::initWithFileDescriptor(NSFileHandle::alloc(), vz_fd_for_nshandle);

        let bridge = Self {
            vmnet_interface,
            _vz_socket_owned: vz_fd,
            host_socket_fd: host_fd,
            shutdown: Arc::new(AtomicBool::new(false)),
            threads: Vec::new(),
        };

        Ok((bridge, vz_ns_handle))
    }

    pub fn start(mut self) -> Result<Self> {
        info!("Starting vmnet bridge");
        let vmnet_to_vz = self.spawn_vmnet_to_vz_forwarder()?;
        self.threads.push(vmnet_to_vz);

        let vz_to_vmnet = self.spawn_vz_to_vmnet_forwarder()?;
        self.threads.push(vz_to_vmnet);

        Ok(self)
    }

    fn spawn_vmnet_to_vz_forwarder(&self) -> Result<thread::JoinHandle<()>> {
        let vmnet = self.vmnet_interface.clone();
        let host_fd = self.host_socket_fd.as_raw_fd();
        let shutdown = self.shutdown.clone();

        let handle = thread::spawn(move || {
            let mut buffer = vec![0u8; PACKET_BUFFER_SIZE];

            loop {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                match vmnet.read_packet(&mut buffer) {
                    Ok(size) if size > 0 => {
                        let mut retries = 0;
                        while let Err(e) = Self::write_to_socket(host_fd, &buffer[..size]) {
                            if e.raw_os_error() != Some(libc::ENOBUFS) {
                                break;
                            }

                            retries += 1;
                            if retries > MAX_ENOBUFS_RETRIES {
                                break;
                            }

                            thread::yield_now();
                        }
                    }
                    Ok(_) => thread::yield_now(),
                    Err(_) => thread::yield_now(),
                }
            }
        });

        Ok(handle)
    }

    fn spawn_vz_to_vmnet_forwarder(&self) -> Result<thread::JoinHandle<()>> {
        let vmnet = self.vmnet_interface.clone();
        let host_fd = self.host_socket_fd.as_raw_fd();
        let shutdown = self.shutdown.clone();

        let handle = thread::spawn(move || {
            let mut buffer = vec![0u8; PACKET_BUFFER_SIZE];

            loop {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                match Self::read_from_socket(host_fd, &mut buffer) {
                    Ok(size) if size > 0 => {
                        let _ = vmnet.write_packet(&buffer[..size]);
                    }
                    Ok(_) => thread::yield_now(),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::yield_now();
                    }
                    Err(_) => thread::yield_now(),
                }
            }
        });

        Ok(handle)
    }

    fn read_from_socket(fd: RawFd, buffer: &mut [u8]) -> io::Result<usize> {
        let result = unsafe {
            libc::recv(
                fd,
                buffer.as_mut_ptr() as *mut libc::c_void,
                buffer.len(),
                0,
            )
        };

        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(result as usize)
        }
    }

    fn write_to_socket(fd: RawFd, buffer: &[u8]) -> io::Result<usize> {
        let result =
            unsafe { libc::send(fd, buffer.as_ptr() as *const libc::c_void, buffer.len(), 0) };

        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(result as usize)
        }
    }
}

impl Drop for VmnetBridge {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.join_threads();
    }
}

impl VmnetBridge {
    fn join_threads(&mut self) {
        let threads = std::mem::take(&mut self.threads);
        for thread in threads {
            let _ = thread.join();
        }
    }
}

#[derive(Debug)]
pub struct VmnetAttachment {
    _bridge: VmnetBridge,
    filehandle: Retained<NSFileHandle>,
}

unsafe impl Send for VmnetAttachment {}

impl VmnetAttachment {
    pub fn new(mode: vmnet::mode::Mode) -> Result<Self> {
        let (bridge, filehandle) = VmnetBridge::new(mode)?;
        let bridge = bridge.start()?;

        Ok(Self {
            _bridge: bridge,
            filehandle,
        })
    }

    pub fn filehandle(&self) -> &Retained<NSFileHandle> {
        &self.filehandle
    }
}
