#![no_std]
#![no_main]

use core::ffi::c_void;
use core::mem;
use uefi::Identify;
use uefi::prelude::*;

mod rng_protocol;
use rng_protocol::RngProtocol;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let allocation = uefi::boot::allocate_pool(
        uefi::boot::MemoryType::RUNTIME_SERVICES_DATA,
        mem::size_of::<RngProtocol>(),
    )
    .unwrap();

    let storage = allocation.as_ptr().cast::<RngProtocol>();

    unsafe {
        storage.write(RngProtocol::new());
        uefi::boot::install_protocol_interface(None, &RngProtocol::GUID, storage.cast::<c_void>())
            .unwrap();
    }

    Status::SUCCESS
}
