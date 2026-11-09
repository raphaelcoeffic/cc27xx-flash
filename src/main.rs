//! Flash algorithm for the TI CC27xx (CC2745, CC2755): main flash in 2 KB sectors, the CCFG at
//! 0x4E02_0000 and the SCFG at 0x4E04_0000.
//!
//! Erase and program go through the flash functions of the boot ROM (driverlib `hapi.h`).
//! The CCFG is erased and written only when it differs from the image. The boot ROM can't
//! write the SCFG: programming it fails unless the device already holds the image's SCFG.

#![no_std]
#![no_main]

use flash_algorithm::*;

struct Algorithm {
    /// VIMS:CCHCTL before the cache was turned off.
    vims_cchctl: u32,
}

algorithm!(Algorithm, {
    device_name: "cc27xx",
    device_type: DeviceType::Onchip,
    flash_address: 0x0,
    flash_size: 0x4E04_0800,
    page_size: 0x800,
    empty_value: 0xFF,
    program_time_out: 1000,
    erase_time_out: 2000,
    sectors: [{
        size: 0x800,
        address: 0x0,
    }]
});

const FLASH_KEY: u32 = 0xB7E3_A08F;

/// The boot ROM's function table.
const HAPI_TABLE: *const usize = 0x0F00_004C as *const usize;
/// `uint32_t flashSectorErase(uint32_t key, uint32_t sectorAddress)`
const HAPI_SECTOR_ERASE: usize = 3;
/// `uint32_t flashProgram(uint32_t key, const uint8_t *data, uint32_t address, uint32_t nBytes)`
const HAPI_PROGRAM: usize = 5;

const VIMS_CCHCTL: *mut u32 = 0x4002_4424 as *mut u32;
const VIMS_CCHSTA: *const u32 = 0x4002_4428 as *const u32;
const VIMS_CCHSTA_BUSY: u32 = 1 << 1;

const CCFG_START: u32 = 0x4E02_0000;
const SCFG_START: u32 = 0x4E04_0000;
const SCFG_SIZE: usize = 0x400;

/// Returned when the image's SCFG differs from the device's.
const ERROR_SCFG_CHANGED: u32 = 0x5CF6_0000;

fn hapi_function(index: usize) -> usize {
    unsafe { HAPI_TABLE.add(index).read_volatile() }
}

fn status(status: u32) -> Result<(), ErrorCode> {
    match ErrorCode::new(status) {
        None => Ok(()),
        Some(error) => Err(error),
    }
}

fn erase(address: u32) -> Result<(), ErrorCode> {
    let erase: extern "C" fn(u32, u32) -> u32 =
        unsafe { core::mem::transmute(hapi_function(HAPI_SECTOR_ERASE)) };
    status(erase(FLASH_KEY, address))
}

fn program(address: u32, data: &[u8]) -> Result<(), ErrorCode> {
    let program: extern "C" fn(u32, *const u8, u32, u32) -> u32 =
        unsafe { core::mem::transmute(hapi_function(HAPI_PROGRAM)) };
    status(program(FLASH_KEY, data.as_ptr(), address, data.len() as u32))
}

fn holds(address: u32, data: &[u8]) -> bool {
    let flash = unsafe { core::slice::from_raw_parts(address as *const u8, data.len()) };
    flash == data
}

fn wait_vims() {
    while unsafe { VIMS_CCHSTA.read_volatile() } & VIMS_CCHSTA_BUSY != 0 {}
}

impl FlashAlgorithm for Algorithm {
    fn new(_address: u32, _clock: u32, _function: Function) -> Result<Self, ErrorCode> {
        // The flash can't be read while it is busy.
        cortex_m::interrupt::disable();

        // The cache is off while programming, as in driverlib's FlashProgram.
        let vims_cchctl = unsafe { VIMS_CCHCTL.read_volatile() };
        unsafe { VIMS_CCHCTL.write_volatile(0) };
        wait_vims();

        Ok(Self { vims_cchctl })
    }

    fn erase_sector(&mut self, address: u32) -> Result<(), ErrorCode> {
        // The CCFG is erased when it is written. An erased CCFG keeps the device from booting.
        if address >= CCFG_START {
            return Ok(());
        }
        erase(address)
    }

    fn program_page(&mut self, address: u32, data: &[u8]) -> Result<(), ErrorCode> {
        if address >= SCFG_START {
            return match holds(address, &data[..data.len().min(SCFG_SIZE)]) {
                true => Ok(()),
                false => status(ERROR_SCFG_CHANGED),
            };
        }
        if address >= CCFG_START {
            if holds(address, data) {
                return Ok(());
            }
            erase(address)?;
        }
        program(address, data)
    }
}

impl Drop for Algorithm {
    fn drop(&mut self) {
        unsafe { VIMS_CCHCTL.write_volatile(self.vims_cchctl) };
        wait_vims();
        unsafe { cortex_m::interrupt::enable() };
    }
}
