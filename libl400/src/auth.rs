use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Object error: {0}")]
    Object(#[from] crate::object::ObjectError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid authority: {0}")]
    InvalidAuthority(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum L400Authority {
    Use,
    Change,
    All,
    Exclude,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum L400Operation {
    Read,
    Change,
    Execute,
    Admin,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L400Identity {
    pub profile: String,
    pub uid: u32,
    pub owner: String,
    pub groups: Vec<String>,
}

impl L400Identity {
    pub fn from_env() -> Self {
        let profile = crate::audit::current_l400_user();
        Self {
            owner: profile.clone(),
            profile,
            uid: unsafe { libc::geteuid() },
            groups: std::env::var("L400_GROUPS")
                .unwrap_or_default()
                .split(':')
                .map(str::trim)
                .filter(|group| !group.is_empty())
                .map(str::to_uppercase)
                .collect(),
        }
    }
}

impl std::fmt::Display for L400Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            L400Operation::Read => write!(f, "READ"),
            L400Operation::Change => write!(f, "CHANGE"),
            L400Operation::Execute => write!(f, "EXECUTE"),
            L400Operation::Admin => write!(f, "ADMIN"),
        }
    }
}

impl std::fmt::Display for L400Authority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            L400Authority::Use => write!(f, "*USE"),
            L400Authority::Change => write!(f, "*CHANGE"),
            L400Authority::All => write!(f, "*ALL"),
            L400Authority::Exclude => write!(f, "*EXCLUDE"),
        }
    }
}

impl std::str::FromStr for L400Authority {
    type Err = AuthError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "*USE" | "USE" => Ok(L400Authority::Use),
            "*CHANGE" | "CHANGE" => Ok(L400Authority::Change),
            "*ALL" | "ALL" => Ok(L400Authority::All),
            "*EXCLUDE" | "EXCLUDE" => Ok(L400Authority::Exclude),
            _ => Err(AuthError::InvalidAuthority(s.to_string())),
        }
    }
}

pub const L400_AUTH_ATTR: &str = "user.l400.auth";
pub const L400_AUTH_VERSION_ATTR: &str = "user.l400.auth.version";
pub const L400_AUTH_MANIFEST_ATTR: &str = "user.l400.auth.manifest";
pub const L400_AUTH_MANIFEST_VERSION: u32 = 2;

/// Lee las autorizaciones de un objeto (formato "USER:PERM,PUBLIC:PERM")
pub fn get_object_authorities(path: &Path) -> Result<HashMap<String, L400Authority>, AuthError> {
    let mut auths = HashMap::new();
    if let Some(raw) = xattr::get(path, L400_AUTH_ATTR)?
        && let Ok(s) = String::from_utf8(raw)
    {
        for part in s.split(',') {
            if let Some((user, perm)) = part.split_once(':')
                && let Ok(authority) = perm.parse()
            {
                auths.insert(user.to_string(), authority);
            }
        }
    }
    Ok(auths)
}

/// Guarda las autorizaciones en un objeto
pub fn set_object_authorities(
    path: &Path,
    auths: &HashMap<String, L400Authority>,
) -> Result<(), AuthError> {
    let mut parts = Vec::new();
    for (user, perm) in auths {
        parts.push(format!("{}:{}", user, perm));
        if let Some(uid) = uid_for_profile(path, user)? {
            parts.push(format!("UID:{}:{}", uid, perm));
        }
    }
    let serialized = parts.join(",");
    xattr::set(path, L400_AUTH_ATTR, serialized.as_bytes())?;
    xattr::set(
        path,
        L400_AUTH_VERSION_ATTR,
        L400_AUTH_MANIFEST_VERSION.to_string().as_bytes(),
    )?;
    xattr::set(
        path,
        L400_AUTH_MANIFEST_ATTR,
        build_auth_manifest(path, auths, &serialized)?.as_bytes(),
    )?;
    Ok(())
}

fn build_auth_manifest(
    path: &Path,
    auths: &HashMap<String, L400Authority>,
    serialized: &str,
) -> Result<String, AuthError> {
    let mut entries = Vec::new();
    for (profile, authority) in auths {
        let origin = if profile == "*PUBLIC" {
            "public"
        } else {
            "explicit"
        };
        let uid = uid_for_profile(path, profile)?.unwrap_or_else(|| "-".to_string());
        entries.push(format!("{profile}:{uid}:{authority}:{origin}"));
    }
    entries.sort();
    Ok(format!(
        "version={};entries={};flat={serialized}",
        L400_AUTH_MANIFEST_VERSION,
        entries.join(",")
    ))
}

fn uid_for_profile(object_path: &Path, profile: &str) -> Result<Option<String>, AuthError> {
    let profile = profile.trim().to_uppercase();
    if profile.is_empty() || profile.starts_with('*') || profile.starts_with("UID:") {
        return Ok(None);
    }

    let root = object_path
        .ancestors()
        .find(|candidate| candidate.join("QSYS").exists())
        .map(Path::to_path_buf)
        .unwrap_or_else(crate::object::resolve_l400_root);
    let profile_path = root.join("QSYS").join(&profile);
    if !profile_path.exists() {
        return Ok(None);
    }
    let Some(raw) = xattr::get(&profile_path, crate::object::L400_OWNER_UID_ATTR)? else {
        return Ok(None);
    };
    let Ok(uid) = String::from_utf8(raw) else {
        return Ok(None);
    };
    let uid = uid.trim();
    if uid.chars().all(|ch| ch.is_ascii_digit()) && !uid.is_empty() {
        Ok(Some(uid.to_string()))
    } else {
        Ok(None)
    }
}

/// Otorga un permiso específico a un usuario sobre un objeto
pub fn grant_object_authority(
    path: &Path,
    user: &str,
    authority: L400Authority,
) -> Result<(), AuthError> {
    let mut auths = get_object_authorities(path)?;
    auths.insert(user.to_string(), authority);
    set_object_authorities(path, &auths)?;
    Ok(())
}

/// Revoca los permisos específicos de un usuario sobre un objeto
pub fn revoke_object_authority(path: &Path, user: &str) -> Result<(), AuthError> {
    let mut auths = get_object_authorities(path)?;
    if auths.remove(user).is_some() {
        set_object_authorities(path, &auths)?;
    }
    Ok(())
}

/// Chequea si un usuario tiene al menos el permiso requerido
pub fn check_authority(
    path: &Path,
    user: &str,
    required: L400Authority,
) -> Result<bool, AuthError> {
    check_authority_with_groups(path, user, &[], required)
}

pub fn check_authority_for_identity(
    path: &Path,
    identity: &L400Identity,
    required: L400Authority,
) -> Result<bool, AuthError> {
    check_authority_with_groups(path, &identity.profile, &identity.groups, required)
}

fn check_authority_with_groups(
    path: &Path,
    user: &str,
    groups: &[String],
    required: L400Authority,
) -> Result<bool, AuthError> {
    let auths = get_object_authorities(path)?;

    // El permiso explícito del usuario tiene mayor prioridad
    if let Some(auth) = auths.get(user) {
        if *auth == L400Authority::Exclude {
            return Ok(false);
        }
        return Ok(auth_level(*auth) >= auth_level(required));
    }

    for group in groups {
        if let Some(auth) = auths.get(group) {
            if *auth == L400Authority::Exclude {
                return Ok(false);
            }
            return Ok(auth_level(*auth) >= auth_level(required));
        }
    }

    // El dueño conserva *ALL implícito antes del fallback público.
    if let Some(raw) = xattr::get(path, crate::object::L400_OWNER_ATTR)?
        && let Ok(owner) = String::from_utf8(raw)
        && owner == user
    {
        return Ok(true);
    }

    // Fallback a permiso público (*PUBLIC)
    if let Some(auth) = auths.get("*PUBLIC") {
        if *auth == L400Authority::Exclude {
            return Ok(false);
        }
        return Ok(auth_level(*auth) >= auth_level(required));
    }

    // Por defecto, sin permiso explícito ni público, se deniega (OS/400 strict)
    Ok(false)
}

pub fn required_authority_for_operation(operation: L400Operation) -> L400Authority {
    match operation {
        L400Operation::Read | L400Operation::Execute => L400Authority::Use,
        L400Operation::Change => L400Authority::Change,
        L400Operation::Admin => L400Authority::All,
    }
}

pub fn required_operation_for_command(command: &str) -> L400Operation {
    match command.trim().to_uppercase().as_str() {
        "CALL" => L400Operation::Execute,
        "DSPOBJ" | "DSPOBJAUT" | "DSPPFM" | "DSPDTAQ" | "WRKOBJ" | "WRKLIB" => L400Operation::Read,
        "GRTOBJAUT" | "RVKOBJAUT" | "CHGOBJD" | "DLTOBJ" | "CLRPFM" => L400Operation::Admin,
        "WRTPFM" | "SNDDTAQ" | "RCVDTAQ" | "CPYOBJ" => L400Operation::Change,
        "CRTUSRPRF" | "CHGUSRPRF" | "DLTUSRPRF" => L400Operation::Admin,
        "DSPUSRPRF" | "WRKUSRPRF" => L400Operation::Read,
        "CRTJOBQ" | "DLTJOBQ" | "HLDJOBQ" | "RLSJOBQ" => L400Operation::Admin,
        "WRKJOBQ" | "WRKACTJOB" | "WRKJOB" => L400Operation::Read,
        "CRTOUTQ" | "DLTOUTQ" | "HLDOUTQ" | "RLSOUTQ" => L400Operation::Admin,
        "WRKOUTQ" | "DSPOUTQ" | "WRKSPLF" => L400Operation::Read,
        "HLDSPOOL" | "RLSSPOOL" | "DLTSPLF" => L400Operation::Change,
        _ => L400Operation::Read,
    }
}

pub fn check_command_authority(path: &Path, user: &str, command: &str) -> Result<bool, AuthError> {
    let operation = required_operation_for_command(command);
    let identity = L400Identity {
        profile: user.trim().to_uppercase(),
        uid: unsafe { libc::geteuid() },
        owner: user.trim().to_uppercase(),
        groups: std::env::var("L400_GROUPS")
            .unwrap_or_default()
            .split(':')
            .map(str::trim)
            .filter(|group| !group.is_empty())
            .map(str::to_uppercase)
            .collect(),
    };
    let allowed =
        check_authority_for_identity(path, &identity, required_authority_for_operation(operation))?;
    if !allowed {
        let _ = crate::audit::audit_event(
            "AUTH_DENIED",
            user,
            path,
            &format!("source=runtime command={} operation={}", command, operation),
        );
    }
    Ok(allowed)
}

pub fn authority_matrix_rows() -> Vec<(&'static str, L400Operation, L400Authority)> {
    vec![
        ("CALL", L400Operation::Execute, L400Authority::Use),
        ("DSPOBJD/DSPOBJAUT", L400Operation::Read, L400Authority::Use),
        ("WRKOBJ/WRKLIB", L400Operation::Read, L400Authority::Use),
        ("DSPPFM/DSPDTAQ", L400Operation::Read, L400Authority::Use),
        (
            "WRTPFM/SNDDTAQ/RCVDTAQ",
            L400Operation::Change,
            L400Authority::Change,
        ),
        ("CPYOBJ", L400Operation::Change, L400Authority::Change),
        (
            "GRTOBJAUT/RVKOBJAUT",
            L400Operation::Admin,
            L400Authority::All,
        ),
        (
            "CHGOBJD/DLTOBJ/CLRPFM",
            L400Operation::Admin,
            L400Authority::All,
        ),
    ]
}

fn auth_level(auth: L400Authority) -> u8 {
    match auth {
        L400Authority::Exclude => 0,
        L400Authority::Use => 1,
        L400Authority::Change => 2,
        L400Authority::All => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{catalog_object, create_library, create_object_with_metadata};

    #[test]
    fn public_exclude_denies_call_authority() {
        let root = tempfile::tempdir().expect("tempdir");
        let lib = create_library(root.path(), "QGPL").expect("create library");
        let pgm = lib.join("HELLO");
        std::fs::write(&pgm, "#!/bin/sh\nexit 0\n").expect("write pgm");
        catalog_object(&pgm, "*PGM", Some("CL"), Some("test")).expect("catalog pgm");
        grant_object_authority(&pgm, "*PUBLIC", L400Authority::Exclude).expect("grant exclude");

        assert!(!check_command_authority(&pgm, "QPGMR", "CALL").expect("check authority"));
    }

    #[test]
    fn explicit_user_use_allows_call_when_public_missing() {
        let root = tempfile::tempdir().expect("tempdir");
        let lib = create_library(root.path(), "QGPL").expect("create library");
        let pgm = lib.join("HELLO");
        std::fs::write(&pgm, "#!/bin/sh\nexit 0\n").expect("write pgm");
        catalog_object(&pgm, "*PGM", Some("CL"), Some("test")).expect("catalog pgm");
        grant_object_authority(&pgm, "QPGMR", L400Authority::Use).expect("grant use");

        assert!(check_command_authority(&pgm, "QPGMR", "CALL").expect("check authority"));
    }

    #[test]
    fn grant_profile_authority_writes_uid_entry_for_ebpf() {
        let root = tempfile::tempdir().expect("tempdir");
        let qsys = root.path().join("QSYS");
        std::fs::create_dir_all(&qsys).expect("create qsys dir");
        catalog_object(&qsys, "*LIB", Some("LIB"), Some("System library")).expect("catalog qsys");
        create_object_with_metadata(&qsys, "QPGMR", "*USRPRF", Some("USRPRF"), Some("profile"))
            .expect("create profile");
        let lib = root.path().join("QGPL");
        std::fs::create_dir_all(&lib).expect("create qgpl dir");
        catalog_object(&lib, "*LIB", Some("LIB"), Some("General library")).expect("catalog qgpl");
        let pgm = lib.join("HELLO");
        std::fs::write(&pgm, "#!/bin/sh\nexit 0\n").expect("write pgm");
        catalog_object(&pgm, "*PGM", Some("CL"), Some("test")).expect("catalog pgm");

        grant_object_authority(&pgm, "*PUBLIC", L400Authority::Exclude).expect("grant exclude");
        grant_object_authority(&pgm, "QPGMR", L400Authority::Use).expect("grant use");

        let auth = xattr::get(&pgm, L400_AUTH_ATTR)
            .expect("auth attr")
            .expect("auth present");
        let auth = String::from_utf8(auth).expect("auth utf8");
        let uid = unsafe { libc::geteuid() };
        assert!(auth.contains("QPGMR:*USE"));
        assert!(auth.contains(&format!("UID:{uid}:*USE")));
    }

    #[test]
    fn group_authority_allows_runtime_operation_and_writes_manifest() {
        let root = tempfile::tempdir().expect("tempdir");
        let lib = create_library(root.path(), "QGPL").expect("create library");
        let file = lib.join("DATA");
        std::fs::write(&file, "data").expect("write file");
        catalog_object(&file, "*FILE", Some("PF"), Some("test")).expect("catalog file");
        grant_object_authority(&file, "DEVGRP", L400Authority::Use).expect("grant group use");

        let version = xattr::get(&file, L400_AUTH_VERSION_ATTR)
            .expect("version attr")
            .expect("version present");
        let manifest = xattr::get(&file, L400_AUTH_MANIFEST_ATTR)
            .expect("manifest attr")
            .expect("manifest present");
        assert_eq!(
            String::from_utf8(version).unwrap(),
            L400_AUTH_MANIFEST_VERSION.to_string()
        );
        let manifest = String::from_utf8(manifest).unwrap();
        assert!(manifest.contains("version=2"));
        assert!(manifest.contains("DEVGRP:-:*USE:explicit"));
        assert!(manifest.contains("flat=DEVGRP:*USE"));

        let identity = L400Identity {
            profile: "QUSER".to_string(),
            uid: 1000,
            owner: "QUSER".to_string(),
            groups: vec!["DEVGRP".to_string()],
        };
        assert!(
            check_authority_for_identity(&file, &identity, L400Authority::Use)
                .expect("check group authority")
        );
    }

    #[test]
    fn owner_authority_allows_runtime_operation_when_public_excluded() {
        let root = tempfile::tempdir().expect("tempdir");
        let lib = create_library(root.path(), "QGPL").expect("create library");
        let file = lib.join("OWNED");
        std::fs::write(&file, "data").expect("write file");
        catalog_object(&file, "*FILE", Some("PF"), Some("test")).expect("catalog file");
        xattr::set(&file, crate::object::L400_OWNER_ATTR, b"QOWNER").expect("owner attr");
        grant_object_authority(&file, "*PUBLIC", L400Authority::Exclude).expect("grant exclude");

        let identity = L400Identity {
            profile: "QOWNER".to_string(),
            uid: 1000,
            owner: "QOWNER".to_string(),
            groups: Vec::new(),
        };
        assert!(
            check_authority_for_identity(&file, &identity, L400Authority::Change)
                .expect("check owner authority")
        );
    }
}
