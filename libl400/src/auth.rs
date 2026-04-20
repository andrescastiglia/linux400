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
pub fn check_authority(path: &Path, user: &str, required: L400Authority) -> Result<bool, AuthError> {
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

fn auth_level(auth: L400Authority) -> u8 {
    match auth {
        L400Authority::Exclude => 0,
        L400Authority::Use => 1,
        L400Authority::Change => 2,
        L400Authority::All => 3,
    }
}
