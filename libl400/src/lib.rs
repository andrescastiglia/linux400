pub mod cgroup;
pub mod db;
pub mod dtaq;
pub mod lam;
pub mod object;
pub mod util;
pub mod zfs;

pub use cgroup::{
    assign_to_workload, cleanup_l400_slices, create_l400_slices, get_current_workload,
    get_workload_params, is_cgroup_v2_available, set_cpu_priority, set_memory_limit, CgroupError,
    CgroupParams, WorkloadType,
};
pub use db::{create_lf, create_pf, DbError, LogicalFile, PhysicalFile};
pub use dtaq::{crtdtaq, DataQueue, DtaqError};
pub use lam::{
    detect_hardware_mode, enable_for_platform, get_space_bits, is_lam_enabled, is_tagged_pointer,
    tag_pointer, untag_pointer, untag_pointer_mut, MemoryTaggingMode,
};
pub use object::{
    catalog_object, copy_object, create_library, create_object, create_object_with_metadata,
    delete_object, describe_object, ensure_library, list_objects, lookup_object,
    open_object_direct, resolve_l400_root, L400Object, ObjectError,
};
pub use util::AlignedBuffer;
pub use zfs::{get_objtype, set_objtype, validate_objtype, ZfsError};

#[no_mangle]
pub extern "C" fn init() {
    let _ = enable_for_platform();
}
