use std::sync::OnceLock;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LamError {
    #[error("Hardware LAM not supported on this CPU")]
    LamNotSupported,
    #[error("arch_prctl failed: {0}")]
    PrctlFailed(String),
    #[error("CPU feature detection failed")]
    CpuIdFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryTaggingMode {
    IntelLam48,
    ArmTbi,
    SoftwareMask,
    Unsupported,
}

const LAM_MASK_48BIT: u64 = 0x0000_FFFF_FFFF_FFFF;
const TBI_MASK: u64 = 0x00FF_FFFF_FFFF_FFFF;

const ARCH_SET_LAM: u32 = 0x1025;
const ARCH_LAM_U57_NOT_TRACKED: u64 = 1;

static DETECTED_MODE: OnceLock<MemoryTaggingMode> = OnceLock::new();
static LAM_ENABLED: OnceLock<bool> = OnceLock::new();

#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn arch_prctl_call(code: u32, addr: u64) -> i32 {
    let ret: i32;
    core::arch::asm!(
        "syscall",
        in("rax") code as u64,
        in("rdi") addr,
        out("rcx") _,
        out("r11") _,
        lateout("rax") ret
    );
    ret
}

pub fn detect_hardware_mode() -> MemoryTaggingMode {
    *DETECTED_MODE.get_or_init(detect_mode)
}

#[cfg(target_arch = "x86_64")]
fn detect_mode() -> MemoryTaggingMode {
    if is_intel_lam_available() {
        MemoryTaggingMode::IntelLam48
    } else {
        MemoryTaggingMode::SoftwareMask
    }
}

#[cfg(target_arch = "aarch64")]
fn detect_mode() -> MemoryTaggingMode {
    MemoryTaggingMode::ArmTbi
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
fn detect_mode() -> MemoryTaggingMode {
    MemoryTaggingMode::Unsupported
}

#[cfg(target_arch = "x86_64")]
fn is_intel_lam_available() -> bool {
    has_cpuid_lam() && is_sapphire_rapids_or_newer()
}

#[cfg(target_arch = "x86_64")]
fn has_cpuid_lam() -> bool {
    let result = std::arch::x86_64::__cpuid_count(7, 2);
    (result.edx & (1 << 28)) != 0
}

#[cfg(target_arch = "x86_64")]
fn is_sapphire_rapids_or_newer() -> bool {
    let cpuid = std::arch::x86_64::__cpuid(1);
    let family = (cpuid.eax >> 8) & 0xF;
    let extended_family = (cpuid.eax >> 20) & 0xFF;
    let model = (cpuid.eax >> 4) & 0xF;
    let extended_model = (cpuid.eax >> 16) & 0xF;

    let full_family = if family == 6 {
        extended_family.saturating_add(family)
    } else {
        family
    };
    let full_model = if family == 6 || family == 15 {
        (extended_model << 4) | model
    } else {
        model
    };

    full_family == 6 && full_model > 0xAF
}

#[cfg(target_arch = "x86_64")]
fn enable_lam48() -> Result<(), LamError> {
    #[cfg(target_os = "linux")]
    {
        let ret = unsafe { arch_prctl_call(ARCH_SET_LAM, ARCH_LAM_U57_NOT_TRACKED) };
        if ret == 0 {
            LAM_ENABLED.get_or_init(|| true);
            Ok(())
        } else {
            Err(LamError::PrctlFailed(format!(
                "arch_prctl returned {} (errno may indicate LAM not supported)",
                ret
            )))
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(LamError::LamNotSupported)
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn enable_lam48() -> Result<(), LamError> {
    Err(LamError::LamNotSupported)
}

#[cfg(target_arch = "aarch64")]
fn enable_tbi() -> Result<(), LamError> {
    LAM_ENABLED.get_or_init(|| true);
    Ok(())
}

#[cfg(not(target_arch = "aarch64"))]
fn enable_tbi() -> Result<(), LamError> {
    Err(LamError::LamNotSupported)
}

pub fn enable_for_platform() -> Result<MemoryTaggingMode, LamError> {
    let mode = detect_hardware_mode();

    match mode {
        MemoryTaggingMode::IntelLam48 => {
            enable_lam48()?;
            Ok(mode)
        }
        MemoryTaggingMode::ArmTbi => {
            enable_tbi()?;
            Ok(mode)
        }
        MemoryTaggingMode::SoftwareMask => Ok(mode),
        MemoryTaggingMode::Unsupported => Err(LamError::LamNotSupported),
    }
}

#[inline]
pub fn tag_pointer<T>(ptr: *const T, space: u16) -> *const T {
    let addr = ptr as u64;
    let tagged = addr | ((space as u64) << 48);
    tagged as *const T
}

#[inline]
pub fn untag_pointer<T>(ptr: *const T) -> *const T {
    let addr = ptr as u64;
    let mode = detect_hardware_mode();

    let mask = match mode {
        MemoryTaggingMode::IntelLam48 => LAM_MASK_48BIT,
        MemoryTaggingMode::ArmTbi => TBI_MASK,
        MemoryTaggingMode::SoftwareMask => LAM_MASK_48BIT,
        MemoryTaggingMode::Unsupported => LAM_MASK_48BIT,
    };

    (addr & mask) as *const T
}

#[inline]
pub fn untag_pointer_mut<T>(ptr: *mut T) -> *mut T {
    untag_pointer(ptr) as *mut T
}

#[inline]
pub fn is_tagged_pointer<T>(ptr: *const T) -> bool {
    let addr = ptr as u64;
    let upper = (addr >> 48) as u16;
    upper != 0 && upper != 0xFFFF
}

#[inline]
pub fn get_space_bits<T>(ptr: *const T) -> Option<u16> {
    if !is_tagged_pointer(ptr) {
        return None;
    }
    Some(((ptr as u64) >> 48) as u16)
}

pub fn is_lam_enabled() -> bool {
    *LAM_ENABLED.get().unwrap_or(&false)
}

/// Convierte un tipo de objeto OS/400 (ej. "*PGM") a un ID numérico predefinido para usar en LAM.
pub fn tag_for_objtype(objtype: &str) -> u16 {
    match objtype {
        "*PGM" => 1,
        "*FILE" => 2,
        "*USRPRF" => 3,
        "*LIB" => 4,
        "*DTAQ" => 5,
        "*CMD" => 6,
        "*SRVPGM" => 7,
        "*OUTQ" => 8,
        _ => 99, // Unknown/Other
    }
}

/// Convierte el tag LAM (bits 48-63) a un string con el tipo de objeto.
pub fn objtype_from_tag(tag: u16) -> &'static str {
    match tag {
        1 => "*PGM",
        2 => "*FILE",
        3 => "*USRPRF",
        4 => "*LIB",
        5 => "*DTAQ",
        6 => "*CMD",
        7 => "*SRVPGM",
        8 => "*OUTQ",
        _ => "*UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_software_mask_roundtrip() {
        let original: i32 = 42;
        let ptr = &original as *const i32;

        let tagged = tag_pointer(ptr, 0x1234);
        assert!(is_tagged_pointer(tagged));
        assert_eq!(get_space_bits(tagged), Some(0x1234));

        let untagged = untag_pointer(tagged);
        assert_eq!(untagged, ptr);
        assert!(!is_tagged_pointer(untagged));
    }

    #[test]
    fn test_tag_pointer_space_encoding() {
        let data = [0u8; 16];
        let ptr = data.as_ptr();

        for space in [1u16, 0x1234, 0x0001, 0x0002, 0x8000] {
            let tagged = tag_pointer(ptr, space);
            assert!(
                is_tagged_pointer(tagged),
                "Pointer should be tagged for space {:#x}",
                space
            );
            let retrieved_space = get_space_bits(tagged);
            assert_eq!(
                retrieved_space,
                Some(space),
                "Space bits mismatch for {:#x}",
                space
            );
        }
    }

    #[test]
    fn test_null_pointer_handling() {
        let null_ptr: *const i32 = std::ptr::null();
        let tagged = tag_pointer(null_ptr, 0x0001);

        let untagged = untag_pointer(tagged);
        assert_eq!(untagged, null_ptr);
    }

    #[test]
    fn test_detect_mode_returns_valid() {
        let mode = detect_hardware_mode();
        assert!(
            matches!(
                mode,
                MemoryTaggingMode::IntelLam48
                    | MemoryTaggingMode::ArmTbi
                    | MemoryTaggingMode::SoftwareMask
            ),
            "Invalid mode detected: {:?}",
            mode
        );
    }

    #[test]
    fn test_objtype_encoding() {
        assert_eq!(tag_for_objtype("*PGM"), 1);
        assert_eq!(tag_for_objtype("*FILE"), 2);
        
        assert_eq!(objtype_from_tag(1), "*PGM");
        assert_eq!(objtype_from_tag(2), "*FILE");
        assert_eq!(objtype_from_tag(99), "*UNKNOWN");
    }
}
