const RAPID_SEED: u64 = 0xf4d8ef1732c4ed6e;

const RAPID_SECRET_0: u64 = 0x2d358dccaa6c78a5;
const RAPID_SECRET_1: u64 = 0x8bb84b93962eacc9;
const RAPID_SECRET_2: u64 = 0x4b33a62ed433d4a3;

const INIT_SEED: u64 = {
    let a = RAPID_SEED ^ RAPID_SECRET_0;
    let b = RAPID_SECRET_1;
    let r = (a as u128) * (b as u128);
    RAPID_SEED ^ ((r as u64) ^ ((r >> 64) as u64))
};

const EMPTY_HASH: u64 = {
    let r = (INIT_SEED as u128) * (RAPID_SECRET_2 as u128);
    (r as u64) ^ ((r >> 64) as u64)
};

#[inline(always)]
fn rapid_mix(a: u64, b: u64) -> u64 {
    let r = (a as u128) * (b as u128);
    (r as u64) ^ ((r >> 64) as u64)
}

#[inline(always)]
fn read64(p: *const u8, offset: usize) -> u64 {
    unsafe { (p.add(offset) as *const u64).read_unaligned() }
}

#[inline(always)]
fn read32(p: *const u8, offset: usize) -> u32 {
    unsafe { (p.add(offset) as *const u32).read_unaligned() }
}

#[inline(always)]
fn read8(p: *const u8, offset: usize) -> u8 {
    unsafe { *p.add(offset) }
}

#[inline(always)]
pub fn hash_string(s: &str) -> u64 {
    let l = s.len();
    let mut seed = INIT_SEED ^ (l as u64);
    let p = s.as_ptr();

    // 16 byte or smaller strings
    if l <= 16 {
        let mut a: u64;
        let mut b: u64 = 0;
        if l >= 8 {
            a = read64(p, 0);
            b = read64(p, l - 8);
        } else if l >= 4 {
            a = read32(p, 0) as u64;
            b = read32(p, l - 4) as u64;
        } else if l > 0 {
            a = (read8(p, 0) as u64) << 16;
            a |= (read8(p, l >> 1) as u64) << 8;
            a |= read8(p, l - 1) as u64;
        } else {
            return EMPTY_HASH;
        }
        return rapid_mix(seed ^ a, RAPID_SECRET_2 ^ b);
    }

    if l <= 32 {
        seed = rapid_mix(
            read64(p, 0) ^ RAPID_SECRET_2,
            read64(p, 8) ^ seed ^ RAPID_SECRET_1,
        );
        let a = read64(p, l - 16);
        let b = read64(p, l - 8);
        return rapid_mix(seed ^ a, RAPID_SECRET_2 ^ b);
    }

    hash_string_long(p, l, seed)
}

#[cold]
#[inline(never)]
fn hash_string_long(p: *const u8, l: usize, mut seed: u64) -> u64 {
    let mut i: usize = 0;

    if l > 96 {
        // reads 96 bytes at a time and splits into 6 chunks continously and mixes n with n+3 to 3 results, then xor those.
        let mut see1 = seed;
        let mut see2 = seed;

        while i < l - 96 {
            seed = rapid_mix(read64(p, i) ^ RAPID_SECRET_0, read64(p, i + 8) ^ seed);
            see1 = rapid_mix(read64(p, i + 16) ^ RAPID_SECRET_1, read64(p, i + 24) ^ see1);
            see2 = rapid_mix(read64(p, i + 32) ^ RAPID_SECRET_2, read64(p, i + 40) ^ see2);
            seed = rapid_mix(read64(p, i + 48) ^ RAPID_SECRET_0, read64(p, i + 56) ^ seed);
            see1 = rapid_mix(read64(p, i + 64) ^ RAPID_SECRET_1, read64(p, i + 72) ^ see1);
            see2 = rapid_mix(read64(p, i + 80) ^ RAPID_SECRET_2, read64(p, i + 88) ^ see2);
            i += 96;
        }
        seed ^= see1 ^ see2;
    }

    while i < l - 16 {
        // consume 16 bytes at a time, first 8 bytes mixed with next 8
        seed = rapid_mix(
            read64(p, i) ^ RAPID_SECRET_2,
            read64(p, i + 8) ^ seed ^ RAPID_SECRET_1,
        );
        i += 16
    }

    // handle the remaning bytes
    let a = read64(p, l - 16);
    let b = read64(p, l - 8);

    return rapid_mix(seed ^ a, RAPID_SECRET_2 ^ b);
}
