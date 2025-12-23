use flate2::write::GzEncoder;
use flate2::Compression;
use std::path::Path;
use tar::Builder;
use traits::OciSkillsError;

pub fn create_tar_gz(dir: &Path) -> Result<Vec<u8>, OciSkillsError> {
    let mut tar_gz = Vec::new();
    {
        let encoder = GzEncoder::new(&mut tar_gz, Compression::default());
        let mut archive = Builder::new(encoder);

        // Add all contents of the directory
        archive
            .append_dir_all(".", dir)
            .map_err(|e| OciSkillsError::Io(e))?;

        archive.finish().map_err(|e| OciSkillsError::Io(e))?;
    }
    Ok(tar_gz)
}
