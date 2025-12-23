use flate2::read::GzDecoder;
use std::path::Path;
use tar::Archive;
use traits::OciSkillsError;

pub fn extract_tar_gz(data: &[u8], dest: &Path) -> Result<(), OciSkillsError> {
    let decoder = GzDecoder::new(data);
    let mut archive = Archive::new(decoder);
    archive.unpack(dest).map_err(|e| OciSkillsError::Io(e))?;
    Ok(())
}
