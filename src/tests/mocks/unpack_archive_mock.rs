use mockall::automock;

#[allow(dead_code)]
#[automock]
pub mod unpacker {
    use std::path::Path;

    pub fn rar_unpack(
        _archive_path: &Path,
        _destination_path: &Path,
    ) -> compress_tools::Result<()> {
        unimplemented!()
    }

    pub fn compress_tools_unpack(
        _archive_path: &Path,
        _destination_path: &Path,
    ) -> compress_tools::Result<()> {
        unimplemented!()
    }
}
