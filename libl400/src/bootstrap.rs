use crate::dtaq::{crtdtaq, DataQueue, DtaqError};
use crate::object::{
    catalog_object, create_object_with_metadata, create_source_member, describe_object,
    ensure_library, member_path, ObjectError,
};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const BASE_LIBRARIES: &[(&str, &str)] = &[
    ("QSYS", "System library"),
    ("QGPL", "General purpose library"),
    ("QUSRSYS", "User system library"),
    ("QTEMP", "Session temporary library"),
];

const BASE_PROFILES: &[(&str, &str)] = &[
    ("QSECOFR", "Security officer profile"),
    ("QPGMR", "Programmer profile"),
    ("QUSER", "Default user profile"),
];

const BASE_COMMANDS: &[(&str, &str)] = &[
    ("ADDLIBLE", "Add library list entry"),
    ("CHGCURLIB", "Change current library"),
    ("ADDPFM", "Add physical file member"),
    ("CRTLIB", "Create library"),
    ("CRTDTAQ", "Create data queue"),
    ("CRTLF", "Create logical file"),
    ("CRTPF", "Create physical file"),
    ("CRTCLPGM", "Create CL program"),
    ("CRTPGM", "Create program"),
    ("CALL", "Call program"),
    ("CLRPFM", "Clear physical file member"),
    ("CPYOBJ", "Copy object"),
    ("DLTLIB", "Delete library"),
    ("DLTOBJ", "Delete object"),
    ("DSPDTAQ", "Display data queue"),
    ("DSPOBJAUT", "Display object authority"),
    ("CHKOBJAUT", "Check object authority"),
    ("DSPOBJD", "Display object description"),
    ("DSPAUD", "Display audit log"),
    ("DSPPFM", "Display physical file member"),
    ("DSPPOLICY", "Display security policy"),
    ("DSPLOG", "Display log"),
    ("GO", "Show menu"),
    ("GRTOBJAUT", "Grant object authority"),
    ("PWRDWNSYS", "Power down system"),
    ("RNMOBJ", "Rename object"),
    ("RVKOBJAUT", "Revoke object authority"),
    ("SBMJOB", "Submit job"),
    ("SIGNOFF", "Sign off"),
    ("SNDDTAQ", "Send data queue message"),
    ("STRPDM", "Start PDM"),
    ("STRSEU", "Start SEU"),
    ("STRSQL", "Start SQL"),
    ("WRKACTJOB", "Work with active jobs"),
    ("WRKMBRPDM", "Work with members using PDM"),
    ("WRKOBJ", "Work with objects"),
    ("WRKSYSSTS", "Work with system status"),
    ("WRKSYSVAL", "Work with system values"),
    ("WRKUSRPRF", "Work with user profiles"),
    ("WRKSPLF", "Work with spool files"),
    ("RCVDTAQ", "Receive data queue message"),
    ("WRTPFM", "Write physical file member record"),
];

const HELLO_CL: &str = "PGM\n    /* Linux/400 bootstrap source member */\nENDPGM\n";

#[derive(Error, Debug)]
pub enum BootstrapError {
    #[error("Object error: {0}")]
    Object(#[from] ObjectError),
    #[error("Data queue error: {0}")]
    Dtaq(#[from] DtaqError),
    #[error("File system error: {0}")]
    Fs(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapReport {
    pub root: PathBuf,
    pub created: Vec<String>,
    pub existing: Vec<String>,
}

impl BootstrapReport {
    fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            created: Vec::new(),
            existing: Vec::new(),
        }
    }

    fn created(&mut self, name: impl Into<String>) {
        self.created.push(name.into());
    }

    fn existing(&mut self, name: impl Into<String>) {
        self.existing.push(name.into());
    }
}

fn ensure_cataloged_path(
    path: &Path,
    objtype: &str,
    attr: &str,
    text: &str,
    marker: &str,
    report: &mut BootstrapReport,
) -> Result<(), BootstrapError> {
    let existed = path.exists();
    if !existed {
        if objtype == "*FILE" {
            fs::create_dir_all(path)?;
        } else if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            fs::File::create(path)?;
        }
    }

    match describe_object(path) {
        Ok(object) if object.objtype == objtype => report.existing(marker),
        _ => {
            catalog_object(path, objtype, Some(attr), Some(text))?;
            if existed {
                report.existing(marker);
            } else {
                report.created(marker);
            }
        }
    }

    Ok(())
}

fn ensure_library_with_text(
    root: &Path,
    name: &str,
    text: &str,
    report: &mut BootstrapReport,
) -> Result<PathBuf, BootstrapError> {
    let existed = root.join(name).exists();
    let path = ensure_library(root, name)?;
    catalog_object(&path, "*LIB", Some("LIB"), Some(text))?;
    if existed {
        report.existing(format!("{name} *LIB"));
    } else {
        report.created(format!("{name} *LIB"));
    }
    Ok(path)
}

fn ensure_object(
    lib: &Path,
    name: &str,
    objtype: &str,
    attr: &str,
    text: &str,
    report: &mut BootstrapReport,
) -> Result<(), BootstrapError> {
    let marker = format!(
        "{}/{} {}",
        lib.file_name().unwrap_or_default().to_string_lossy(),
        name,
        objtype
    );
    let path = lib.join(name);
    if path.exists() {
        match describe_object(&path) {
            Ok(object) if object.objtype == objtype => {
                report.existing(marker);
                return Ok(());
            }
            _ => {
                catalog_object(&path, objtype, Some(attr), Some(text))?;
                report.existing(marker);
                return Ok(());
            }
        }
    }

    create_object_with_metadata(lib, name, objtype, Some(attr), Some(text))?;
    report.created(marker);
    Ok(())
}

fn ensure_data_queue(
    lib: &Path,
    name: &str,
    report: &mut BootstrapReport,
) -> Result<(), BootstrapError> {
    let path = lib.join(name);
    let marker = format!(
        "{}/{} *DTAQ",
        lib.file_name().unwrap_or_default().to_string_lossy(),
        name
    );
    if path.exists() {
        catalog_object(&path, "*DTAQ", Some("DTAQ"), Some("Job log data queue"))?;
        let _ = DataQueue::open(&path)?;
        report.existing(marker);
        return Ok(());
    }

    crtdtaq(lib, name)?;
    catalog_object(&path, "*DTAQ", Some("DTAQ"), Some("Job log data queue"))?;
    report.created(marker);
    Ok(())
}

fn ensure_source_member_file(
    lib: &Path,
    file: &str,
    member: &str,
    contents: &str,
    report: &mut BootstrapReport,
) -> Result<(), BootstrapError> {
    let path = member_path(lib, file, member)?;
    let marker = format!(
        "{}/{}/{}",
        lib.file_name().unwrap_or_default().to_string_lossy(),
        file,
        member
    );
    if path.exists() {
        report.existing(marker);
        return Ok(());
    }

    let member_path = create_source_member(lib, file, member)?;
    fs::write(member_path, contents)?;
    report.created(marker);
    Ok(())
}

pub fn bootstrap_l400_root(root: &Path) -> Result<BootstrapReport, BootstrapError> {
    fs::create_dir_all(root)?;
    let mut report = BootstrapReport::new(root);

    for (library, text) in BASE_LIBRARIES {
        ensure_library_with_text(root, library, text, &mut report)?;
    }

    let qsys = root.join("QSYS");
    let qgpl = root.join("QGPL");
    let qusrsys = root.join("QUSRSYS");

    let qclsrc = qgpl.join("QCLSRC");
    ensure_cataloged_path(
        &qclsrc,
        "*FILE",
        "SRC",
        "CL source file",
        "QGPL/QCLSRC *FILE",
        &mut report,
    )?;
    ensure_source_member_file(&qgpl, "QCLSRC", "HELLO.CLP", HELLO_CL, &mut report)?;

    ensure_data_queue(&qusrsys, "QEZJOBLOG", &mut report)?;

    for (profile, text) in BASE_PROFILES {
        ensure_object(&qsys, profile, "*USRPRF", "USRPRF", text, &mut report)?;
    }

    for (command, text) in BASE_COMMANDS {
        ensure_object(&qsys, command, "*CMD", "CMD", text, &mut report)?;
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{list_members, list_objects};

    #[test]
    fn bootstrap_creates_base_objects_and_is_idempotent() {
        let root = tempfile::tempdir().expect("tempdir");

        let first = bootstrap_l400_root(root.path()).expect("first bootstrap");
        assert!(!first.created.is_empty());

        let qsys = root.path().join("QSYS");
        let qgpl = root.path().join("QGPL");
        let qusrsys = root.path().join("QUSRSYS");

        assert_eq!(describe_object(&qsys).expect("qsys").objtype, "*LIB");
        assert_eq!(
            describe_object(&qgpl.join("QCLSRC"))
                .expect("qclsrc")
                .objtype,
            "*FILE"
        );
        assert_eq!(
            describe_object(&qusrsys.join("QEZJOBLOG"))
                .expect("qezjoblog")
                .objtype,
            "*DTAQ"
        );

        let members = list_members(&qgpl, "QCLSRC").expect("members");
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].file_name, "HELLO.CLP");

        let qsys_objects = list_objects(&qsys).expect("objects");
        assert!(qsys_objects.iter().any(|object| object.name == "QSECOFR"));
        assert!(qsys_objects.iter().any(|object| object.name == "WRKOBJ"));

        let second = bootstrap_l400_root(root.path()).expect("second bootstrap");
        assert!(second.created.is_empty());
        assert!(!second.existing.is_empty());
    }
}
