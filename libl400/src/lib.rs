pub mod db;
pub mod dtaq;
pub mod object;
pub mod util;
pub mod zfs;

pub use db::{create_lf, create_pf, DbError, LogicalFile, PhysicalFile};
pub use dtaq::{crtdtaq, DataQueue, DtaqError};
pub use object::{
    copy_object, create_object, delete_object, list_objects, open_object_direct, L400Object,
    ObjectError,
};
pub use util::AlignedBuffer;
pub use zfs::{get_objtype, set_objtype, validate_objtype, ZfsError};

#[no_mangle]
pub extern "C" fn init() {}
