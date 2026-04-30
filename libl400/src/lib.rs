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

pub use audit::{AuditRecord, audit_event, current_l400_user, qhst_path, read_audit_records};
pub use auth::{L400_AUTH_MANIFEST_VERSION, L400Authority, L400Identity, L400Operation};
pub use bootstrap::{BootstrapError, BootstrapReport, bootstrap_l400_root};
pub use cgroup::{
    CgroupError, CgroupParams, WorkloadJob, WorkloadType, assign_to_workload, cleanup_l400_slices,
    create_l400_slices, end_job, get_current_workload, get_workload_params, hold_job,
    is_cgroup_v2_available, job_log_path, kill_job, list_jobs, register_current_job, register_job,
    release_job, remove_job, set_cpu_priority, set_memory_limit, subsystem_description,
    subsystem_descriptions, update_job_status,
};
pub use cmd::{
    COMMAND_METADATA, COMMAND_METADATA_SCHEMA_VERSION, CommandMetadata, CommandParameter,
    command_metadata, format_command_params,
};
pub use db::{
    DEFAULT_PF_MEMBER, DbError, LogicalFile, PfField, PfSchema, PhysicalFile, QueryResult,
    SqlStatementResult, add_pf_member, create_lf, create_lf_filtered, create_pf, list_pf_members,
    read_pf_schema, run_select_query, run_sql_statement, write_pf_schema,
};
pub use dtaq::{DataQueue, DtaqError, crtdtaq};
pub use lam::{
    MemoryTaggingMode, detect_hardware_mode, enable_for_platform, get_space_bits, is_lam_enabled,
    is_tagged_pointer, tag_pointer, untag_pointer, untag_pointer_mut,
};
pub use object::{
    L400Object, ObjectError, SourceMemberInfo, catalog_object, copy_object, create_library,
    create_object, create_object_with_metadata, create_source_member, delete_object,
    describe_object, ensure_library, list_libraries, list_members, list_objects, lookup_object,
    member_path, open_object_direct, resolve_l400_root,
};
pub use runtime::{
    LoaderStatus, RuntimeStatusError, l400_run_dir, loader_status_path, read_loader_status,
    runtime_version, write_loader_status,
};
pub use status::{
    CPF_CATALOG, CommandStatus, CommandStatusOccurrence, command_status, command_status_occurrence,
    normalize_cpf,
};
pub use storage::{
    L400_BASE_PF_ATTR, L400_DATA_FORMAT_VERSION, L400_DATA_FORMAT_VERSION_ATTR,
    L400_FIELD_SCHEMA_ATTR, L400_KEY_FIELDS_ATTR, L400_OUTQ_DEFAULT_STATUS_ATTR,
    L400_OUTQ_RETENTION_DAYS_ATTR, L400_OUTQ_ROUTING_ATTR, L400_PF_MEMBERS_ATTR,
    L400_RECORD_LEN_ATTR, L400_STORAGE_BACKEND_ATTR, StorageBackend, StorageError,
    default_storage_backend, read_storage_backend, read_string_attr, read_u32_attr,
    write_storage_backend, write_string_attr, write_u32_attr,
};
pub use util::AlignedBuffer;
pub use zfs::{
    ZfsError, get_objtype, path_is_on_zfs, set_objtype, validate_objtype, zfs_dataset_for_path,
    zfs_xattr_mode,
};

#[unsafe(no_mangle)]
pub extern "C" fn init() {
    let _ = enable_for_platform();
}
