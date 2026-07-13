//! .NET / Mono memory-reading helpers reusable across managed-runtime games

use asr::{Address, Process};

pub fn read_u32_ptr(process: &Process, addr: Address) -> Option<Address> {
    let v = process.read::<u32>(addr).ok()?;
    if v == 0 {
        return None;
    }
    Some(Address::new(v as u64))
}

pub fn read_u64_ptr(process: &Process, addr: Address) -> Option<Address> {
    let v = process.read::<u64>(addr).ok()?;
    if v == 0 {
        return None;
    }
    Some(Address::new(v))
}

// Read a 32-bit .NET Framework string object (length i32 @ +4, UTF-16 chars @ +8) into buf
pub fn read_net_string<'b>(process: &Process, obj: Address, buf: &'b mut [u8; 64]) -> &'b [u8] {
    let Ok(len) = process.read::<i32>(obj + 4u32) else {
        return &[];
    };
    if len <= 0 || len > 64 {
        return &[];
    }
    let len = len as usize;
    let mut units = [0u16; 64];
    if process
        .read_into_slice(obj + 8u32, &mut units[..len])
        .is_err()
    {
        return &[];
    }
    for i in 0..len {
        buf[i] = if units[i] < 128 { units[i] as u8 } else { b'?' };
    }
    &buf[..len]
}
