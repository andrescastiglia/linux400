pub mod audit;
pub mod auth;
mod bdb_native;
pub mod bootstrap;
pub mod cgroup;
pub mod cmd;
pub mod db;
pub mod dtaq;
pub mod ffi;
pub mod ffi_commands;
pub mod lam;
pub mod object;
pub mod runtime;
pub mod space;
pub mod status;
pub mod storage;
pub mod usrprf;
pub mod util;
pub mod zfs;

pub use audit::{audit_event, current_l400_user, qhst_path, read_audit_records, AuditRecord};
pub use auth::{L400Authority, L400Identity, L400Operation};
pub use bootstrap::{bootstrap_l400_root, BootstrapError, BootstrapReport};
pub use cgroup::{
    assign_to_workload, cleanup_l400_slices, create_l400_slices, end_job, get_current_workload,
    get_workload_params, hold_job, is_cgroup_v2_available, job_log_path, list_jobs,
    register_current_job, register_job, release_job, remove_job, set_cpu_priority,
    set_memory_limit, subsystem_description, subsystem_descriptions, update_job_status,
    CgroupError, CgroupParams, WorkloadJob, WorkloadType,
};
pub use cmd::{
    command_metadata, format_command_params, CommandMetadata, CommandParameter, COMMAND_METADATA,
};
pub use db::{
    add_pf_member, create_lf, create_lf_filtered, create_pf, list_pf_members, read_pf_schema,
    run_select_query, run_sql_statement, write_pf_schema, DbError, LogicalFile, PfField, PfSchema,
    PhysicalFile, QueryResult, SqlStatementResult, DEFAULT_PF_MEMBER,
};
pub use dtaq::{crtdtaq, DataQueue, DtaqError};
pub use lam::{
    detect_hardware_mode, enable_for_platform, get_space_bits, is_lam_enabled, is_tagged_pointer,
    tag_pointer, untag_pointer, untag_pointer_mut, MemoryTaggingMode,
};
pub use object::{
    catalog_object, copy_object, create_library, create_object, create_object_with_metadata,
    create_source_member, delete_object, describe_object, ensure_library, list_libraries,
    list_members, list_objects, lookup_object, member_path, open_object_direct, resolve_l400_root,
    L400Object, ObjectError, SourceMemberInfo,
};
pub use runtime::{
    l400_run_dir, loader_status_path, read_loader_status, write_loader_status, LoaderStatus,
    RuntimeStatusError,
};
pub use status::{
    command_status, command_status_occurrence, normalize_cpf, CommandStatus,
    CommandStatusOccurrence, CPF_CATALOG,
};
pub use storage::{
    default_storage_backend, read_storage_backend, read_string_attr, read_u32_attr,
    write_storage_backend, write_string_attr, write_u32_attr, StorageBackend, StorageError,
    L400_BASE_PF_ATTR, L400_FIELD_SCHEMA_ATTR, L400_KEY_FIELDS_ATTR, L400_PF_MEMBERS_ATTR,
    L400_RECORD_LEN_ATTR, L400_STORAGE_BACKEND_ATTR,
};
pub use util::AlignedBuffer;
pub use zfs::{
    get_objtype, path_is_on_zfs, set_objtype, validate_objtype, zfs_dataset_for_path,
    zfs_xattr_mode, ZfsError,
};

#[no_mangle]
pub extern "C" fn init() {
    let _ = enable_for_platform();
}
