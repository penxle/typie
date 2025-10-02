use crate::error::{Result, VermudaError};
use log::{error, info};
use objc2::{AnyThread, rc::Retained};
use objc2_foundation::{NSArray, NSFileHandle, NSString, NSUInteger, NSURL};
use objc2_virtualization::*;
use serde::{Deserialize, Deserializer, Serialize};
use std::env;
use std::fs::{self, OpenOptions};
use std::os::unix::io::IntoRawFd;
use std::path::{Path, PathBuf};

const EFI_VARIABLE_STORE_NAME: &str = "efivars.bin";

fn parse_size_mb<'de, D>(deserializer: D) -> std::result::Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let s = String::deserialize(deserializer)?;
    let s = s.trim();

    if let Ok(num) = s.parse::<f64>() {
        return Ok((num * 1024.0) as u64);
    }

    let (num_str, unit) = if s.len() >= 2 {
        let split_pos = s
            .chars()
            .position(|c| c.is_alphabetic())
            .ok_or_else(|| D::Error::custom(format!("Invalid size format: {}", s)))?;
        let (num, unit) = s.split_at(split_pos);
        (num.trim(), unit.trim())
    } else {
        return Err(D::Error::custom(format!("Invalid size format: {}", s)));
    };

    let num: f64 = num_str
        .parse()
        .map_err(|_| D::Error::custom(format!("Invalid number: {}", num_str)))?;

    let mb = match unit {
        "Gi" => num * 1024.0,
        "Mi" => num,
        "Ti" => num * 1024.0 * 1024.0,
        _ => {
            return Err(D::Error::custom(format!(
                "Unknown size unit: {}. Supported units: Mi, Gi, Ti",
                unit
            )));
        }
    };

    Ok(mb as u64)
}

pub fn get_vm_home() -> Result<PathBuf> {
    if let Ok(vm_home) = env::var("VM_HOME") {
        Ok(PathBuf::from(vm_home))
    } else {
        let home = env::var("HOME")
            .map_err(|_| VermudaError::validation_failed("HOME environment variable not found"))?;
        Ok(PathBuf::from(home).join(".vm"))
    }
}

pub fn get_config_path() -> Result<PathBuf> {
    Ok(get_vm_home()?.join("config.toml"))
}

pub fn get_efi_store_path() -> Result<PathBuf> {
    Ok(get_vm_home()?.join(EFI_VARIABLE_STORE_NAME))
}

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
    #[serde(default)]
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
            boot: Some(BootConfig { path: None }),
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
    #[serde(deserialize_with = "parse_size_mb")]
    size: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self { size: 2048 }
    }
}

impl MemoryConfig {
    pub fn size_bytes(&self) -> u64 {
        self.size * 1024 * 1024
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootConfig {
    #[serde(default)]
    path: Option<PathBuf>,
}

impl BootConfig {
    pub fn get_path(&self) -> Result<PathBuf> {
        if let Some(ref path) = self.path {
            Ok(path.clone())
        } else {
            Ok(get_vm_home()?.join("boot.img"))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootConfig {
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(deserialize_with = "parse_size_mb")]
    pub size: u64,
}

impl RootConfig {
    pub fn get_path(&self) -> Result<PathBuf> {
        if let Some(ref path) = self.path {
            Ok(path.clone())
        } else {
            Ok(get_vm_home()?.join("disk.img"))
        }
    }
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
    #[serde(default = "default_display_width")]
    pub width: u32,
    #[serde(default = "default_display_height")]
    pub height: u32,
    #[serde(default = "default_display_ppi")]
    pub ppi: u32,
}

fn default_display_width() -> u32 {
    1024
}

fn default_display_height() -> u32 {
    768
}

fn default_display_ppi() -> u32 {
    96
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskConfig {
    pub path: PathBuf,
}

impl VmConfig {
    pub fn load() -> Result<Self> {
        let config_path = get_config_path()?;
        Self::from_toml_file(&config_path)
    }

    pub fn from_toml_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|e| {
            VermudaError::validation_failed(format!(
                "Failed to read config file ({}): {}",
                path.display(),
                e
            ))
        })?;

        toml::from_str(&content).map_err(|e| {
            VermudaError::validation_failed(format!("Failed to parse config file: {}", e))
        })
    }

    pub fn with_iso_override(mut self, iso_path: Option<PathBuf>) -> Self {
        if let Some(path) = iso_path {
            self.iso = Some(IsoConfig { path });
        }
        self
    }

    pub fn display(&self) -> Option<&DisplayConfig> {
        self.display.as_ref()
    }

    pub fn network(&self) -> Option<&NetworkConfig> {
        self.network.as_ref()
    }

    pub fn root(&self) -> Option<&RootConfig> {
        self.root.as_ref()
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
            config.setMemorySize(self.memory.size_bytes());
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
        let store_path = get_efi_store_path()?;
        let url = path_to_url(&store_path);

        unsafe {
            if store_path.exists() {
                Ok(VZEFIVariableStore::initWithURL(
                    VZEFIVariableStore::alloc(),
                    &url,
                ))
            } else {
                info!(
                    "Creating new EFI variable store at {}",
                    store_path.display()
                );
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

        if let Some(boot) = &self.boot {
            let path = boot.get_path()?;
            if path.exists() {
                let attachment = self.disk_attachment(&path, true)?;
                let block = unsafe {
                    VZVirtioBlockDeviceConfiguration::initWithAttachment(
                        VZVirtioBlockDeviceConfiguration::alloc(),
                        &attachment,
                    )
                };
                devices.push(Retained::into_super(block));
            }
        }

        if let Some(root) = &self.root {
            let path = root.get_path()?;
            let attachment = self.disk_attachment(&path, false)?;
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
