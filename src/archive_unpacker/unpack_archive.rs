pub mod unpacker {
    use std::path::Path;

    pub fn rar_unpack(archive_path: &Path, destination_path: &Path) -> compress_tools::Result<()> {
        let path_as_string = archive_path.to_str().unwrap().to_string();
        let archive = unrar::Archive::new(path_as_string);
        let result = archive.extract_to(destination_path.to_str().unwrap().to_string());
        if let Err(error) = result {
            return Err(compress_tools::Error::from(error.to_string()));
        }
        let mut open_archive = result.unwrap();
        let process_result = open_archive.process();
        if let Err(error) = process_result {
            return Err(compress_tools::Error::from(error.to_string()));
        }
        Ok(())
    }

    pub fn compress_tools_unpack(
        archive_path: &Path,
        destination_path: &Path,
    ) -> compress_tools::Result<()> {
        let mut source = std::fs::File::open(archive_path)?;
        compress_tools::uncompress_archive(
            &mut source,
            destination_path,
            compress_tools::Ownership::Ignore,
        )
    }
}
