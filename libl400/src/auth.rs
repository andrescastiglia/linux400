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

/// Lee las autorizaciones de un objeto (formato "USER:PERM,PUBLIC:PERM")
pub fn get_object_authorities(path: &Path) -> Result<HashMap<String, L400Authority>, AuthError> {
    let mut auths = HashMap::new();
    if let Some(raw) = xattr::get(path, L400_AUTH_ATTR)? {
        if let Ok(s) = String::from_utf8(raw) {
            for part in s.split(',') {
                if let Some((user, perm)) = part.split_once(':') {
                    if let Ok(authority) = perm.parse() {
                        auths.insert(user.to_string(), authority);
                    }
                }
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
    }
    let serialized = parts.join(",");
    xattr::set(path, L400_AUTH_ATTR, serialized.as_bytes())?;
    Ok(())
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
    let auths = get_object_authorities(path)?;

    // El permiso explícito del usuario tiene mayor prioridad
    if let Some(auth) = auths.get(user) {
        if *auth == L400Authority::Exclude {
            return Ok(false);
        }
        return Ok(auth_level(*auth) >= auth_level(required));
    }

    // Fallback a permiso público (*PUBLIC)
    if let Some(auth) = auths.get("*PUBLIC") {
        if *auth == L400Authority::Exclude {
            return Ok(false);
        }
        return Ok(auth_level(*auth) >= auth_level(required));
    }

    // Por defecto, sin permiso explícito ni público, se deniega (OS/400 strict)
    // Opcionalmente se puede comprobar si el usuario es el dueño leyendo "user.l400.owner"
    if let Some(raw) = xattr::get(path, crate::object::L400_OWNER_ATTR)? {
        if let Ok(owner) = String::from_utf8(raw) {
            if owner == user {
                return Ok(true); // El dueño siempre tiene *ALL implícito
            }
        }
    }

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
        "DSPOBJD" | "DSPOBJAUT" | "DSPPFM" | "DSPDTAQ" | "WRKOBJ" | "WRKLIB" => L400Operation::Read,
        "GRTOBJAUT" | "RVKOBJAUT" | "CHGOBJD" | "DLTOBJ" | "CLRPFM" => L400Operation::Admin,
        "WRTPFM" | "SNDDTAQ" | "RCVDTAQ" | "CPYOBJ" => L400Operation::Change,
        _ => L400Operation::Read,
    }
}

pub fn check_command_authority(path: &Path, user: &str, command: &str) -> Result<bool, AuthError> {
    let operation = required_operation_for_command(command);
    check_authority(path, user, required_authority_for_operation(operation))
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
    use crate::object::{catalog_object, create_library};

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
}
