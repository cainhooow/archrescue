use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use archrescue_core::{Error, ErrorKind, Result};
use archrescue_storage::safe_existing;
use serde::{Deserialize, Serialize};

use crate::filesystem::Directory;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Folder {
    Desktop,
    Documents,
    Downloads,
    Pictures,
}

impl Folder {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "Desktop" => Ok(Self::Desktop),
            "Documents" => Ok(Self::Documents),
            "Downloads" => Ok(Self::Downloads),
            "Pictures" => Ok(Self::Pictures),
            _ => Err(Error::new(
                ErrorKind::Validation,
                "Pasta conhecida inválida",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub account_sid: String,
    pub root: PathBuf,
    pub folders: BTreeMap<Folder, PathBuf>,
}

impl Profile {
    #[cfg(windows)]
    pub fn current() -> Result<Self> {
        use windows_sys::Win32::UI::Shell::{
            FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Pictures,
            FOLDERID_Profile,
        };

        let account_sid = windows::sid()?;
        let root = windows::known_folder(&FOLDERID_Profile)?;
        let mut folders = BTreeMap::new();

        for (kind, id) in [
            (Folder::Desktop, FOLDERID_Desktop),
            (Folder::Documents, FOLDERID_Documents),
            (Folder::Downloads, FOLDERID_Downloads),
            (Folder::Pictures, FOLDERID_Pictures),
        ] {
            if let Ok(path) = windows::known_folder(&id) {
                folders.insert(kind, path);
            }
        }

        Ok(Self {
            account_sid,
            root,
            folders,
        })
    }

    #[cfg(not(windows))]
    pub fn current() -> Result<Self> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "A resolução de contas exige Windows",
        ))
    }

    pub(crate) fn pin(
        &self,
        selected_sid: &str,
        folders: impl Iterator<Item = Folder>,
    ) -> Result<PinnedProfile> {
        if self.account_sid != selected_sid {
            return Err(Error::new(
                ErrorKind::Permission,
                "A conta selecionada deve executar o Agente de Restauração",
            ));
        }

        let _original_root = Directory::open(&self.root)?;
        let root = safe_existing(&self.root)?;
        let profile = Directory::open(&root)?;
        let mut selected = BTreeMap::new();

        for folder in folders {
            let path = self.folders.get(&folder).ok_or_else(|| {
                Error::new(
                    ErrorKind::Unsupported,
                    format!("Pasta indisponível: {folder:?}"),
                )
            })?;
            let _original_folder = Directory::open(path)?;
            let path = safe_existing(path)?;
            if !path.starts_with(&root) || path == root {
                return Err(Error::new(
                    ErrorKind::Validation,
                    "Pasta redirecionada para fora do perfil",
                ));
            }
            if selected.values().any(|other: &Directory| {
                path.starts_with(other.path()) || other.path().starts_with(&path)
            }) {
                return Err(Error::new(
                    ErrorKind::Validation,
                    "Pastas selecionadas se sobrepõem",
                ));
            }

            selected.insert(folder, Directory::open(&path)?);
        }

        Ok(PinnedProfile {
            profile,
            selected,
        })
    }
}

pub(crate) struct PinnedProfile {
    pub profile: Directory,
    pub selected: BTreeMap<Folder, Directory>,
}

impl PinnedProfile {
    pub fn relative<'a>(&self, path: &'a Path) -> Result<&'a Path> {
        path.strip_prefix(self.profile.path())
            .map_err(|_| Error::new(ErrorKind::Validation, "Caminho fora do perfil selecionado"))
    }
}

#[cfg(windows)]
mod windows {
    use std::{ffi::OsString, io, os::windows::ffi::OsStringExt, ptr::null_mut};

    use windows_sys::{
        core::GUID,
        Win32::{
            Foundation::{CloseHandle, LocalFree},
            Security::{
                Authorization::ConvertSidToStringSidW, GetTokenInformation, TokenUser, TOKEN_QUERY,
                TOKEN_USER,
            },
            System::{
                Com::CoTaskMemFree,
                Threading::{GetCurrentProcess, OpenProcessToken},
            },
            UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DONT_VERIFY},
        },
    };

    use super::*;

    // Both native APIs return NUL-terminated allocations held alive by the caller.
    unsafe fn wide_string(pointer: *const u16) -> OsString {
        let length = (0..)
            .take_while(|offset| *pointer.add(*offset) != 0)
            .count();
        OsString::from_wide(std::slice::from_raw_parts(pointer, length))
    }

    pub fn known_folder(id: &GUID) -> Result<PathBuf> {
        let mut path = null_mut();
        // SAFETY: id and the output pointer live through the call. A null token selects this user.
        let result =
            unsafe { SHGetKnownFolderPath(id, KF_FLAG_DONT_VERIFY as u32, null_mut(), &mut path) };
        let output = if result >= 0 && !path.is_null() {
            // SAFETY: the successful call returned a live, terminated wide string.
            Ok(PathBuf::from(unsafe { wide_string(path) }))
        } else {
            Err(Error::new(
                ErrorKind::Environment,
                format!("Resolver pasta conhecida: HRESULT {result:#x}"),
            ))
        };
        // SAFETY: the API uses the COM task allocator and permits null.
        unsafe {
            CoTaskMemFree(path.cast());
        }

        output
    }

    pub fn sid() -> Result<String> {
        let mut token = null_mut();
        // SAFETY: token is writable and the process pseudo-handle remains valid.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(Error::io(
                "Abrir token da conta",
                io::Error::last_os_error(),
            ));
        }

        let mut size = 0;
        // SAFETY: query size only, with a null buffer and a writable size output.
        unsafe {
            GetTokenInformation(token, TokenUser, null_mut(), 0, &mut size);
        }
        let mut buffer = vec![0usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
        // SAFETY: buffer provides size bytes aligned for TOKEN_USER.
        let read = unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                buffer.as_mut_ptr().cast(),
                size,
                &mut size,
            )
        };
        let read_error = io::Error::last_os_error();
        // SAFETY: OpenProcessToken transferred ownership of this real handle.
        unsafe {
            CloseHandle(token);
        }
        if read == 0 {
            return Err(Error::io("Consultar SID da conta", read_error));
        }

        let mut string = null_mut();
        // SAFETY: successful TokenUser initializes the structure and its SID within buffer.
        let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
        // SAFETY: the SID remains valid in buffer and the output is writable.
        if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut string) } == 0 {
            return Err(Error::io(
                "Converter SID da conta",
                io::Error::last_os_error(),
            ));
        }

        // SAFETY: conversion produced a live, NUL-terminated allocation from LocalAlloc.
        let sid = unsafe { wide_string(string) }
            .to_string_lossy()
            .into_owned();
        // SAFETY: conversion transfers ownership to the caller.
        unsafe {
            LocalFree(string.cast());
        }

        Ok(sid)
    }
}
