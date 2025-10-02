// https://uefi.org/specs/UEFI/2.11/37_Secure_Technologies.html#efi-rng-protocol

use chacha20::{ChaCha20, cipher::KeyIvInit, cipher::StreamCipher};
use core::mem;
use uefi::proto::unsafe_protocol;
use uefi::{Guid, Status, guid};

#[repr(C)]
#[unsafe_protocol("3152bca5-eade-433d-862e-c01cdc291f44")]
pub struct RngProtocol {
    pub get_info: unsafe extern "efiapi" fn(
        this: *const RngProtocol,
        rng_algorithm_list_size: *mut usize,
        rng_algorithm_list: *mut Guid,
    ) -> Status,

    pub get_rng: unsafe extern "efiapi" fn(
        this: *const RngProtocol,
        rng_algorithm: *const Guid,
        rng_value_length: usize,
        rng_value: *mut u8,
    ) -> Status,

    chacha20: ChaCha20,
}

impl RngProtocol {
    pub const ALGORITHM_RAW: Guid = guid!("e43176d7-b6e8-4827-b784-7ffdc4b68561");

    pub fn new() -> Self {
        let key = Self::collect_entropy();
        let iv = [0u8; 12];

        let chacha20 = ChaCha20::new(&key.into(), &iv.into());

        Self {
            get_info: Self::get_info,
            get_rng: Self::get_rng,
            chacha20,
        }
    }

    unsafe extern "efiapi" fn get_info(
        _: *const RngProtocol,
        rng_algorithm_list_size: *mut usize,
        rng_algorithm_list: *mut Guid,
    ) -> Status {
        let size = match unsafe { rng_algorithm_list_size.as_mut() } {
            Some(size) => size,
            None => return Status::INVALID_PARAMETER,
        };

        let required_size = mem::size_of::<Guid>();

        if rng_algorithm_list.is_null() {
            *size = required_size;
            return Status::BUFFER_TOO_SMALL;
        }

        if *size < required_size {
            *size = required_size;
            return Status::BUFFER_TOO_SMALL;
        }

        let list = unsafe { core::slice::from_raw_parts_mut(rng_algorithm_list, 1) };
        list[0] = Self::ALGORITHM_RAW;

        *size = required_size;

        Status::SUCCESS
    }

    unsafe extern "efiapi" fn get_rng(
        this: *const RngProtocol,
        rng_algorithm: *const Guid,
        rng_value_length: usize,
        rng_value: *mut u8,
    ) -> Status {
        if rng_value.is_null() || rng_value_length == 0 {
            return Status::INVALID_PARAMETER;
        }

        let this = match unsafe { (this as *mut RngProtocol).as_mut() } {
            Some(protocol) => protocol,
            None => return Status::INVALID_PARAMETER,
        };

        if !rng_algorithm.is_null() {
            let algorithm = unsafe { *rng_algorithm };
            if algorithm != Self::ALGORITHM_RAW {
                return Status::UNSUPPORTED;
            }
        }

        let value = unsafe { core::slice::from_raw_parts_mut(rng_value, rng_value_length) };

        value.fill(0);
        this.chacha20.apply_keystream(value);

        Status::SUCCESS
    }

    fn collect_entropy() -> [u8; 32] {
        let mut entropy = [0u8; 32];

        for i in 0..4 {
            let tsc: u64;

            unsafe {
                core::arch::asm!("mrs {}, CNTVCT_EL0", out(reg) tsc, options(nomem, nostack));
            }

            entropy[i * 8..(i + 1) * 8].copy_from_slice(&tsc.to_le_bytes());

            uefi::boot::stall(50_000);
        }

        entropy
    }
}
