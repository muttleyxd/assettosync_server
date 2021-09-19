use std::path::Path;

use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(test)] {
        use crate::tests::mocks::unpack_archive_mock::mock_unpacker as unpacker;
    } else {
        mod unpack_archive;
        use unpack_archive::unpacker;
    }
}

pub fn unpack_archive(archive_path: &Path, destination_path: &Path) -> compress_tools::Result<()> {
    // compress_tools doesn't work with some rar archives, so unrar is used
    let extension = archive_path.extension();
    if extension.is_some() && extension.unwrap() == "rar" {
        unpacker::rar_unpack(archive_path, destination_path)
    } else {
        unpacker::compress_tools_unpack(archive_path, destination_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;
    lazy_static::lazy_static! {
        static ref MOCK_STABILITY_MUTEX: Mutex<()> = Mutex::new(());
    }

    #[test]
    fn test_unpack_archive_rar() {
        let _lock = MOCK_STABILITY_MUTEX.lock().unwrap();

        let context = unpacker::rar_unpack_context();
        context.expect().returning(|_, _| Ok(()));
        assert!(unpack_archive(Path::new("/archive.rar"), Path::new("/unpack")).is_ok());
    }

    #[test]
    fn test_unpack_archive_no_extension() {
        let _lock = MOCK_STABILITY_MUTEX.lock().unwrap();

        let context = unpacker::compress_tools_unpack_context();
        context.expect().returning(|_, _| Ok(()));
        assert!(unpack_archive(Path::new("/archive"), Path::new("/unpack")).is_ok());
    }

    #[test]
    fn test_unpack_archive_7z() {
        let _lock = MOCK_STABILITY_MUTEX.lock().unwrap();

        let context = unpacker::compress_tools_unpack_context();
        context.expect().returning(|_, _| Ok(()));
        assert!(unpack_archive(Path::new("/archive.7z"), Path::new("/unpack")).is_ok());
    }
}
