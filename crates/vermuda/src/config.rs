use crate::error::{Result, VermudaError};
use log::{error, info};
use objc2::{AnyThread, rc::Retained};
use objc2_foundation::{NSArray, NSFileHandle, NSString, NSUInteger, NSURL};
use objc2_virtualization::*;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::os::unix::io::IntoRawFd;
use std::path::{Path, PathBuf};

const EFI_VARIABLE_STORE_NAME: &str = "efivars.bin";

#[derive(Debug, Default)]
pub struct VmContext {
    pub vmnet: Option<crate::vmnet::VmnetAttachment>,
}

unsafe impl Sync for VmContext {}

impl VmContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_vmnet(mut self, attachment: crate::vmnet::VmnetAttachment) -> Self {
        self.vmnet = Some(attachment);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    cpu: CpuConfig,
    memory: MemoryConfig,
    boot: Option<BootConfig>,
    root: Option<RootConfig>,
    disks: Vec<DiskConfig>,
    iso: Option<IsoConfig>,
    network: Option<NetworkConfig>,
    display: Option<DisplayConfig>,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            cpu: CpuConfig::default(),
            memory: MemoryConfig::default(),
            boot: None,
            root: None,
            disks: Vec::new(),
            iso: None,
            network: None,
            display: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuConfig {
    count: u32,
}

impl Default for CpuConfig {
    fn default() -> Self {
        Self { count: 2 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    size_gb: f64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self { size_gb: 2.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootConfig {
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootConfig {
    path: PathBuf,
    size_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsoConfig {
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub interface: String,
    pub mac_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub width: u32,
    pub height: u32,
    pub ppi: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskConfig {
    pub path: PathBuf,
}

#[derive(Debug, Default)]
pub struct VmConfigBuilder {
    config: VmConfig,
}

impl VmConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn cpu_count(mut self, count: u32) -> Self {
        self.config.cpu.count = count;
        self
    }

    #[must_use]
    pub fn memory_gb(mut self, size: f64) -> Self {
        self.config.memory.size_gb = size;
        self
    }

    #[must_use]
    pub fn with_boot(mut self, path: PathBuf) -> Self {
        self.config.boot = Some(BootConfig { path });
        self
    }

    #[must_use]
    pub fn with_root(mut self, path: PathBuf, size_gb: f64) -> Self {
        self.config.root = Some(RootConfig { path, size_gb });
        self
    }

    #[must_use]
    pub fn with_iso(mut self, path: PathBuf) -> Self {
        self.config.iso = Some(IsoConfig { path });
        self
    }

    #[must_use]
    pub fn with_network(mut self, interface: String, mac_address: String) -> Self {
        self.config.network = Some(NetworkConfig {
            interface,
            mac_address,
        });
        self
    }

    #[must_use]
    pub fn with_display(mut self, width: u32, height: u32, ppi: u32) -> Self {
        self.config.display = Some(DisplayConfig { width, height, ppi });
        self
    }

    #[must_use]
    pub fn with_disk(mut self, path: PathBuf) -> Self {
        self.config.disks.push(DiskConfig { path });
        self
    }

    pub fn build(self) -> Result<VmConfig> {
        Ok(self.config)
    }
}

impl VmConfig {
    pub fn builder() -> VmConfigBuilder {
        VmConfigBuilder::new()
    }

    pub fn display(&self) -> Option<&DisplayConfig> {
        self.display.as_ref()
    }

    pub fn network(&self) -> Option<&NetworkConfig> {
        self.network.as_ref()
    }

    pub fn to_platform_config(
        &self,
        context: &VmContext,
    ) -> Result<Retained<VZVirtualMachineConfiguration>> {
        let config = unsafe { VZVirtualMachineConfiguration::new() };

        self.configure_platform(&config);

        let variable_store = self.create_or_load_variable_store()?;
        self.configure_bootloader(&config, &variable_store);
        self.configure_entropy(&config);

        self.configure_cpu_and_memory(&config);

        self.configure_storages(&config)?;
        self.configure_network(&config, context)?;

        self.configure_input_devices(&config);
        self.configure_graphics(&config);

        Self::validate(&config)?;

        Ok(config)
    }

    fn configure_cpu_and_memory(&self, config: &VZVirtualMachineConfiguration) {
        unsafe {
            config.setCPUCount(self.cpu.count as NSUInteger);
            let memory_bytes = gigabytes_to_bytes(self.memory.size_gb);
            config.setMemorySize(memory_bytes);
        }
    }

    fn configure_platform(&self, config: &VZVirtualMachineConfiguration) {
        unsafe {
            let platform = VZGenericPlatformConfiguration::new();
            let machine_id = VZGenericMachineIdentifier::new();
            platform.setMachineIdentifier(&machine_id);
            config.setPlatform(&platform);
        }
    }

    fn configure_entropy(&self, config: &VZVirtualMachineConfiguration) {
        unsafe {
            let entropy = VZVirtioEntropyDeviceConfiguration::new();
            let devices = NSArray::from_retained_slice(&[Retained::into_super(entropy)]);
            config.setEntropyDevices(&devices);
        }
    }

    fn create_or_load_variable_store(&self) -> Result<Retained<VZEFIVariableStore>> {
        let store_path = PathBuf::from(EFI_VARIABLE_STORE_NAME);
        let url = path_to_url(&store_path);

        unsafe {
            if store_path.exists() {
                Ok(VZEFIVariableStore::initWithURL(
                    VZEFIVariableStore::alloc(),
                    &url,
                ))
            } else {
                info!("Creating new EFI variable store");
                VZEFIVariableStore::initCreatingVariableStoreAtURL_options_error(
                    VZEFIVariableStore::alloc(),
                    &url,
                    VZEFIVariableStoreInitializationOptions::empty(),
                )
                .map_err(|error| {
                    VermudaError::validation_failed(format!(
                        "Failed to create EFI variable store: {:?}",
                        error
                    ))
                })
            }
        }
    }

    fn configure_bootloader(
        &self,
        config: &VZVirtualMachineConfiguration,
        variable_store: &Retained<VZEFIVariableStore>,
    ) {
        unsafe {
            let bootloader = VZEFIBootLoader::new();
            bootloader.setVariableStore(Some(variable_store));
            config.setBootLoader(Some(&Retained::into_super(bootloader)));
        }
    }

    fn configure_storages(&self, config: &VZVirtualMachineConfiguration) -> Result<()> {
        let mut devices: Vec<Retained<VZStorageDeviceConfiguration>> = Vec::new();

        if let Some(root) = &self.root {
            let attachment = self.disk_attachment(&root.path, false)?;
            let block = unsafe {
                VZVirtioBlockDeviceConfiguration::initWithAttachment(
                    VZVirtioBlockDeviceConfiguration::alloc(),
                    &attachment,
                )
            };
            devices.push(Retained::into_super(block));
        }

        if let Some(iso) = &self.iso {
            let attachment = self.disk_attachment(&iso.path, true)?;
            let usb = unsafe {
                VZUSBMassStorageDeviceConfiguration::initWithAttachment(
                    VZUSBMassStorageDeviceConfiguration::alloc(),
                    &attachment,
                )
            };
            devices.push(Retained::into_super(usb));
        }

        if let Some(boot) = &self.boot {
            let attachment = self.disk_attachment(&boot.path, true)?;
            let block = unsafe {
                VZVirtioBlockDeviceConfiguration::initWithAttachment(
                    VZVirtioBlockDeviceConfiguration::alloc(),
                    &attachment,
                )
            };
            devices.push(Retained::into_super(block));
        }

        for disk in &self.disks {
            let attachment = self.device_attachment(&disk.path)?;
            let block = unsafe {
                VZNVMExpressControllerDeviceConfiguration::initWithAttachment(
                    VZNVMExpressControllerDeviceConfiguration::alloc(),
                    &attachment,
                )
            };
            devices.push(Retained::into_super(block));
        }

        if devices.is_empty() {
            return Ok(());
        }

        unsafe {
            let array = NSArray::from_retained_slice(&devices);
            config.setStorageDevices(&array);
        }

        Ok(())
    }

    fn disk_attachment(
        &self,
        path: &Path,
        read_only: bool,
    ) -> Result<Retained<VZDiskImageStorageDeviceAttachment>> {
        let url = path_to_url(path);
        unsafe {
            VZDiskImageStorageDeviceAttachment::initWithURL_readOnly_cachingMode_synchronizationMode_error(
                VZDiskImageStorageDeviceAttachment::alloc(),
                &url,
                read_only,
                VZDiskImageCachingMode::Cached,
                VZDiskImageSynchronizationMode::Full,
            )
            .map_err(|error| {
                VermudaError::validation_failed(format!(
                    "Failed to create disk attachment: {:?}",
                    error
                ))
            })
        }
    }

    fn device_attachment(
        &self,
        path: &Path,
    ) -> Result<Retained<VZDiskBlockDeviceStorageDeviceAttachment>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| {
                VermudaError::validation_failed(format!(
                    "Failed to open block device {}: {}",
                    path.display(),
                    e
                ))
            })?;

        let fd = file.into_raw_fd();
        let file_handle =
            NSFileHandle::initWithFileDescriptor_closeOnDealloc(NSFileHandle::alloc(), fd, true);

        unsafe {
            VZDiskBlockDeviceStorageDeviceAttachment::initWithFileHandle_readOnly_synchronizationMode_error(
                VZDiskBlockDeviceStorageDeviceAttachment::alloc(),
                &file_handle,
                false,
                VZDiskSynchronizationMode::Full,
            )
            .map_err(|error| {
                VermudaError::validation_failed(format!(
                    "Failed to create block device attachment: {:?}",
                    error
                ))
            })
        }
    }

    fn configure_network(
        &self,
        config: &VZVirtualMachineConfiguration,
        context: &VmContext,
    ) -> Result<()> {
        let filehandle = match &context.vmnet {
            Some(vmnet) => vmnet.filehandle(),
            None => return Ok(()),
        };

        let network_config = self.network.as_ref().ok_or_else(|| {
            VermudaError::validation_failed(
                "Network attachment provided without network configuration",
            )
        })?;

        unsafe {
            let network_device = VZVirtioNetworkDeviceConfiguration::new();

            let mac_string = NSString::from_str(&network_config.mac_address);
            let mac = objc2_virtualization::VZMACAddress::initWithString(
                objc2_virtualization::VZMACAddress::alloc(),
                &mac_string,
            )
            .ok_or_else(|| {
                VermudaError::validation_failed(format!(
                    "Invalid MAC address format: '{}'",
                    network_config.mac_address
                ))
            })?;
            network_device.setMACAddress(&mac);

            let attachment = VZFileHandleNetworkDeviceAttachment::initWithFileHandle(
                VZFileHandleNetworkDeviceAttachment::alloc(),
                filehandle,
            );
            network_device.setAttachment(Some(&attachment.into_super()));
            let device: Retained<VZNetworkDeviceConfiguration> =
                Retained::into_super(network_device);
            let devices = NSArray::from_retained_slice(&[device]);
            config.setNetworkDevices(&devices);
        }

        Ok(())
    }

    fn configure_input_devices(&self, config: &VZVirtualMachineConfiguration) {
        unsafe {
            let keyboard = VZUSBKeyboardConfiguration::new();
            let keyboard: Retained<VZKeyboardConfiguration> = Retained::into_super(keyboard);
            let keyboards = NSArray::from_retained_slice(&[keyboard]);
            config.setKeyboards(&keyboards);
        }
    }

    fn configure_graphics(&self, config: &VZVirtualMachineConfiguration) {
        let Some(display) = &self.display else {
            return;
        };

        unsafe {
            let graphics = VZVirtioGraphicsDeviceConfiguration::new();
            let scanout =
                VZVirtioGraphicsScanoutConfiguration::initWithWidthInPixels_heightInPixels(
                    VZVirtioGraphicsScanoutConfiguration::alloc(),
                    display.width as isize,
                    display.height as isize,
                );

            let scanouts = NSArray::from_retained_slice(&[scanout]);
            graphics.setScanouts(&scanouts);
            let graphics_device: Retained<VZGraphicsDeviceConfiguration> =
                Retained::into_super(graphics);
            let graphics_devices = NSArray::from_retained_slice(&[graphics_device]);
            config.setGraphicsDevices(&graphics_devices);
        }
    }

    fn validate(config: &VZVirtualMachineConfiguration) -> Result<()> {
        unsafe {
            config.validateWithError().map_err(|error| {
                error!("VM configuration validation failed");

                let description = error.localizedDescription().to_string();
                let domain = error.domain().to_string();
                let code = error.code();

                error!("Error domain: {}", domain);
                error!("Error code: {}", code);
                error!("Error description: {}", description);

                VermudaError::validation_failed(format!(
                    "Configuration validation failed: {}",
                    description
                ))
            })?;
        }

        info!("VM configuration validated successfully");

        Ok(())
    }
}

fn path_to_url(path: &Path) -> Retained<NSURL> {
    let path_string = path.to_string_lossy();
    NSURL::fileURLWithPath(&NSString::from_str(&path_string))
}

fn gigabytes_to_bytes(size_gb: f64) -> u64 {
    (size_gb * 1024.0 * 1024.0 * 1024.0) as u64
}
