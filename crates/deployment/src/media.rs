//! Read-only inspection of direct images and mounted installation media.

use std::{
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use archrescue_core::{Error, ErrorKind, Result};
use archrescue_storage::safe_existing;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageType {
    Wim,
    Esd,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Image {
    pub index: u32,
    pub name: Option<String>,
    pub edition: Option<String>,
    pub architecture: Option<String>,
    pub version: Option<String>,
    pub expanded_bytes: Option<u64>,
    pub installation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Media {
    pub path: PathBuf,
    pub image_type: ImageType,
    pub container_bytes: u64,
    pub sha256: String,
    pub images: Vec<Image>,
}

fn unsupported(message: &str) -> Error {
    Error::new(ErrorKind::Unsupported, message)
}

fn image_path(input: &Path) -> Result<(PathBuf, ImageType)> {
    let input = safe_existing(input)?;
    let path = if input.is_dir() {
        let sources = safe_existing(&input.join("sources"))?;
        let mut candidates = Vec::new();

        for filename in ["install.wim", "install.esd"] {
            let candidate = sources.join(filename);
            match fs::symlink_metadata(&candidate) {
                Ok(_) => candidates.push(candidate),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(Error::io("Examinar sources", error)),
            }
        }

        match candidates.as_slice() {
            [] => return Err(unsupported("Mídia sem install.wim ou install.esd")),
            [path] => safe_existing(path)?,
            _ => {
                return Err(unsupported(
                    "Selecione diretamente install.wim ou install.esd",
                ))
            }
        }
    } else {
        input
    };

    let image_type = match path.extension().and_then(|name| name.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("wim") => ImageType::Wim,
        Some(extension) if extension.eq_ignore_ascii_case("esd") => ImageType::Esd,
        _ => return Err(unsupported("Use WIM/ESD ou a raiz de uma mídia montada")),
    };

    if !path.is_file() {
        return Err(unsupported(
            "O caminho de imagem deve ser um arquivo regular",
        ));
    }

    Ok((path, image_type))
}

fn header(file: &mut File, length: u64) -> Result<u32> {
    let invalid = || {
        Error::new(
            ErrorKind::Integrity,
            "Cabeçalho WIM/ESD inválido ou truncado",
        )
    };
    let mut bytes = [0u8; 208];
    file.read_exact(&mut bytes).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            invalid()
        } else {
            Error::io("Ler cabeçalho da imagem", error)
        }
    })?;

    let field = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
    let field64 = |offset| u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
    if &bytes[..8] != b"MSWIM\0\0\0" || field(8) != 208 {
        return Err(invalid());
    }
    if bytes[40..44] != [1, 0, 1, 0] {
        return Err(unsupported(
            "Imagens divididas ou dependentes de outras partes",
        ));
    }

    let count = field(44);
    if count == 0 || count > 1024 {
        return Err(invalid());
    }

    // The lookup table and image XML must both be fully present in this container.
    for descriptor in [48, 72] {
        let resource_bytes = field64(descriptor) & ((1 << 56) - 1);
        let offset = field64(descriptor + 8);
        if resource_bytes == 0
            || offset < 208
            || offset
                .checked_add(resource_bytes)
                .is_none_or(|end| end > length)
        {
            return Err(invalid());
        }
    }

    Ok(count)
}

pub fn inspect(
    input: &Path,
    expected_hash: Option<&str>,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<Media> {
    let (path, image_type) = image_path(input)?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;

        // Exclude replacement and writes while DISM opens the same file for inspection.
        options.share_mode(1);
    }

    let mut file = options
        .open(&path)
        .map_err(|error| Error::io("Abrir imagem", error))?;
    let metadata = file
        .metadata()
        .map_err(|error| Error::io("Consultar imagem", error))?;
    let length = metadata.len();
    let count = header(&mut file, length)?;
    let images = image_metadata(&path, count, cancel)?;

    file.seek(SeekFrom::Start(0))
        .map_err(|error| Error::io("Reler imagem", error))?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    let mut read = 0;
    loop {
        check_cancel(cancel)?;
        let bytes = file
            .read(&mut buffer)
            .map_err(|error| Error::io("Calcular SHA-256 da imagem", error))?;
        if bytes == 0 {
            break;
        }

        digest.update(&buffer[..bytes]);
        read += bytes as u64;
        progress(read, length);
    }

    let sha256 = format!("{:x}", digest.finalize());
    if read != length
        || expected_hash.is_some_and(|expected| !expected.eq_ignore_ascii_case(&sha256))
    {
        return Err(Error::new(
            ErrorKind::Integrity,
            "A imagem diverge do tamanho ou SHA-256 esperado",
        ));
    }

    Ok(Media {
        path,
        image_type,
        container_bytes: length,
        sha256,
        images,
    })
}

pub(crate) fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Error::new(ErrorKind::Cancelled, "Operação cancelada"));
    }

    Ok(())
}

#[cfg(not(windows))]
fn image_metadata(_: &Path, _: u32, _: &AtomicBool) -> Result<Vec<Image>> {
    Err(unsupported(
        "A inspeção dos índices requer DISM no Windows ou WinPE",
    ))
}

#[cfg(windows)]
fn image_metadata(path: &Path, count: u32, cancel: &AtomicBool) -> Result<Vec<Image>> {
    use std::{os::windows::process::CommandExt, process::Command};
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

    let mut directory = [0u16; 32768];
    // SAFETY: directory is writable for the supplied UTF-16 capacity.
    let length = unsafe { GetSystemDirectoryW(directory.as_mut_ptr(), directory.len() as u32) };
    if length == 0 || length as usize >= directory.len() {
        return Err(Error::io(
            "Localizar DISM do sistema",
            io::Error::last_os_error(),
        ));
    }

    let executable =
        PathBuf::from(String::from_utf16_lossy(&directory[..length as usize])).join("dism.exe");
    let mut image_argument = std::ffi::OsString::from("/ImageFile:");
    image_argument.push(path);
    let mut images = Vec::new();

    for index in 1..=count {
        check_cancel(cancel)?;
        let output = Command::new(&executable)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .args(["/English", "/Get-ImageInfo"])
            .arg(&image_argument)
            .arg(format!("/Index:{index}"))
            .output()
            .map_err(|error| Error::io("Executar inspeção DISM", error))?;

        let text = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() {
            return Err(Error::new(
                ErrorKind::Validation,
                format!("DISM {}: {}", output.status, text.trim()),
            ));
        }

        images.push(parse_details(&text, index)?);
    }

    Ok(images)
}

#[cfg(any(windows, test))]
fn parse_details(text: &str, index: u32) -> Result<Image> {
    // DISM prints its own Version before the indexed image metadata.
    let details = text
        .lines()
        .filter_map(|line| line.split_once(':'))
        .skip_while(|(key, _)| key.trim() != "Index");
    let field = |name: &str| {
        details.clone().find_map(|(key, value)| {
            let value = value.trim();
            (key.trim() == name && !value.is_empty() && value != "<undefined>")
                .then(|| value.to_owned())
        })
    };

    if field("Index").and_then(|value| value.parse::<u32>().ok()) != Some(index) {
        return Err(Error::new(
            ErrorKind::Integrity,
            "Índice retornado pelo DISM diverge do solicitado",
        ));
    }

    let expanded_bytes = field("Size").and_then(|value| {
        let digits = value.strip_suffix("bytes")?.trim().replace(',', "");
        digits.parse().ok()
    });

    Ok(Image {
        index,
        name: field("Name"),
        edition: field("Edition"),
        architecture: field("Architecture"),
        version: field("Version"),
        expanded_bytes,
        installation: field("Installation"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_preserves_missing_fields_and_rejects_wrong_index() {
        let text = "Index : 2\nName : Windows 10 Pro\nArchitecture : x64\nSize : 8,192 bytes\n";
        let image = parse_details(text, 2).unwrap();

        assert_eq!(image.expanded_bytes, Some(8192));
        assert_eq!(image.version, None);
        assert!(parse_details(text, 1).is_err());
    }

    #[test]
    fn inventory_uses_the_indexed_image_version() {
        let text = "Deployment Image Servicing and Management tool\nVersion: 10.0.26100.1\n\nIndex : 1\nVersion : 10.0.19041\n";
        let image = parse_details(text, 1).unwrap();

        assert_eq!(image.version.as_deref(), Some("10.0.19041"));
        let without_image_version = "Version: 10.0.26100.1\nIndex : 1\n";
        assert_eq!(
            parse_details(without_image_version, 1).unwrap().version,
            None
        );
    }

    #[test]
    fn malformed_or_truncated_image_is_rejected_before_dism() {
        let directory = std::env::temp_dir().join(archrescue_core::Operation::new().id);
        fs::create_dir(&directory).unwrap();
        let path = directory.join("invalid.wim");
        fs::write(&path, b"MSWIM\0\0\0").unwrap();

        let result = inspect(&path, None, &AtomicBool::new(false), |_, _| {});
        fs::remove_dir_all(directory).unwrap();

        assert_eq!(result.unwrap_err().kind, ErrorKind::Integrity);
    }
}
