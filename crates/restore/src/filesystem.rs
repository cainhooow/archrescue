//! Open directory handles keep Windows paths in place while verified output is staged.

use std::{
    fs::{self, File, OpenOptions},
    io,
    path::{Component, Path, PathBuf},
};

use archrescue_core::{Error, ErrorKind, Result};

pub struct Directory {
    path: PathBuf,
    key: String,
    #[cfg(windows)]
    _handles: Vec<File>,
}

impl Directory {
    pub fn open(path: &Path) -> Result<Self> {
        Self::walk(path, false)
    }

    pub fn create(path: &Path) -> Result<Self> {
        Self::walk(path, true)
    }

    fn walk(path: &Path, create: bool) -> Result<Self> {
        if !path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
        {
            return Err(Error::new(
                ErrorKind::Validation,
                "Diretório absoluto sem travessia obrigatório",
            ));
        }

        #[cfg(windows)]
        let mut handles = Vec::new();
        let mut current = PathBuf::new();
        let mut key = String::new();
        for component in path.components() {
            current.push(component);
            if matches!(component, Component::Prefix(_)) {
                continue;
            }

            if create && matches!(component, Component::Normal(_)) {
                match fs::create_dir(&current) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(Error::io("Criar diretório de restauração", error)),
                }
            }

            #[cfg(windows)]
            {
                let (handle, identity) = windows::open_directory(&current)?;
                key = identity;
                handles.push(handle);
            }
            #[cfg(not(windows))]
            {
                use std::os::unix::fs::MetadataExt;
                let metadata = fs::symlink_metadata(&current)
                    .map_err(|error| Error::io("Examinar diretório de restauração", error))?;
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(Error::new(
                        ErrorKind::Unsupported,
                        "Diretório indireto ou inválido",
                    ));
                }
                key = format!("{}:{}", metadata.dev(), metadata.ino());
            }
        }

        Ok(Self {
            path: current,
            key,
            #[cfg(windows)]
            _handles: handles,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

pub fn exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(Error::io("Examinar conflito no destino", error)),
    }
}

pub fn open_input(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        };

        options
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    #[cfg(unix)]
    if fs::symlink_metadata(path)
        .map_err(|error| Error::io("Examinar arquivo", error))?
        .file_type()
        .is_symlink()
    {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Link simbólico na origem",
        ));
    }

    let file = options
        .open(path)
        .map_err(|error| Error::io("Abrir arquivo de backup", error))?;
    let metadata = file
        .metadata()
        .map_err(|error| Error::io("Examinar arquivo aberto", error))?;

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & (0x400 | 0x4000) != 0 {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "Arquivo criptografado ou ponto de nova análise",
            ));
        }
    }

    if !metadata.is_file() {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "A origem deve ser um arquivo regular",
        ));
    }

    Ok(file)
}

pub fn stage(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::{
            Foundation::{GENERIC_READ, GENERIC_WRITE},
            Storage::FileSystem::{DELETE, FILE_SHARE_READ},
        };

        options
            .access_mode(GENERIC_READ | GENERIC_WRITE | DELETE)
            .share_mode(FILE_SHARE_READ);
    }

    options
        .open(path)
        .map_err(|error| Error::io("Criar arquivo temporário exclusivo", error))
}

pub fn read_staged(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        };

        // The writer pins the entry and grants other handles read access only.
        options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE);
    }

    options
        .open(path)
        .map_err(|error| Error::io("Reler arquivo temporário", error))
}

#[cfg(windows)]
pub use windows::publish;

#[cfg(not(windows))]
pub fn publish(_: &File, staged: &Path, target: &Path) -> Result<()> {
    fs::hard_link(staged, target).map_err(|error| Error::io("Publicar sem substituir", error))
}

#[cfg(windows)]
mod windows {
    use std::{
        mem::{offset_of, size_of},
        os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
        ptr,
    };

    use windows_sys::Win32::Storage::FileSystem::{
        FileRenameInfo, GetFileInformationByHandle, SetFileInformationByHandle,
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_ENCRYPTED,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_RENAME_INFO, FILE_SHARE_READ,
    };

    use super::*;

    pub fn open_directory(path: &Path) -> Result<(File, String)> {
        let handle = OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
            .map_err(|error| Error::io(format!("Proteger diretório {}", path.display()), error))?;
        let mut info = BY_HANDLE_FILE_INFORMATION::default();

        // SAFETY: the file owns this handle and info is writable for the call.
        if unsafe { GetFileInformationByHandle(handle.as_raw_handle(), &mut info) } == 0 {
            return Err(Error::io(
                "Identificar diretório",
                io::Error::last_os_error(),
            ));
        }

        if info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
            || info.dwFileAttributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_ENCRYPTED)
                != 0
        {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "Diretório indireto, criptografado ou inválido",
            ));
        }

        let identity = format!(
            "{:08x}:{:08x}{:08x}:{}:{}",
            info.dwVolumeSerialNumber,
            info.nFileIndexHigh,
            info.nFileIndexLow,
            info.ftCreationTime.dwHighDateTime,
            info.ftCreationTime.dwLowDateTime
        );

        Ok((handle, identity))
    }

    pub fn publish(file: &File, _: &Path, target: &Path) -> Result<()> {
        let name: Vec<_> = target.as_os_str().encode_wide().collect();
        let name_bytes = name.len() * size_of::<u16>();
        let bytes = (offset_of!(FILE_RENAME_INFO, FileName) + name_bytes)
            .max(size_of::<FILE_RENAME_INFO>());
        let length = u32::try_from(bytes).map_err(|_| {
            Error::new(
                ErrorKind::Validation,
                "Caminho de publicação excede o limite",
            )
        })?;
        let mut buffer = vec![0usize; bytes.div_ceil(size_of::<usize>())];
        let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();

        // SAFETY: the aligned, zeroed buffer holds the structure and all name bytes.
        // Zeroed ReplaceIfExists and RootDirectory request exclusive publication to an absolute path.
        let renamed = unsafe {
            (*info).FileNameLength = name_bytes as u32;
            ptr::copy_nonoverlapping(
                name.as_ptr(),
                ptr::addr_of_mut!((*info).FileName).cast::<u16>(),
                name.len(),
            );
            SetFileInformationByHandle(file.as_raw_handle(), FileRenameInfo, info.cast(), length)
        };
        if renamed == 0 {
            return Err(Error::io(
                "Publicar arquivo verificado sem substituir",
                io::Error::last_os_error(),
            ));
        }

        Ok(())
    }
}

#[cfg(windows)]
pub fn discard(file: &File, _: &Path) -> Result<()> {
    use std::{mem::size_of, os::windows::io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
    };

    let disposition = FILE_DISPOSITION_INFO {
        DeleteFile: true,
    };
    // SAFETY: the file is open with DELETE access and disposition is initialized.
    let marked = unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle(),
            FileDispositionInfo,
            (&disposition as *const FILE_DISPOSITION_INFO).cast(),
            size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    };
    if marked == 0 {
        return Err(Error::io("Remover temporário", io::Error::last_os_error()));
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn discard(_: &File, path: &Path) -> Result<()> {
    fs::remove_file(path).map_err(|error| Error::io("Remover temporário", error))
}
