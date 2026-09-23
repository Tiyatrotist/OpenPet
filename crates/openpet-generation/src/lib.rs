//! # OpenPet Generation Engine
//!
//! Durable state machine for generating customized desktop pets from 1-3 user photos.

use openpet_types::{GenerationJob, GenerationJobState};
use std::path::Path;
use thiserror::Error;
use tracing::info;

pub const MAX_PHOTO_BYTES: u64 = 20 * 1024 * 1024; // 20 MB per photo

#[derive(Error, Debug, PartialEq)]
pub enum GenerationError {
    #[error("Invalid photo count: {0}. Must provide between 1 and 3 photos.")]
    InvalidPhotoCount(usize),

    #[error("File '{0}' exceeds maximum size of 20MB")]
    FileTooLarge(String),

    #[error("Unsupported image format '{0}'. Allowed: PNG, JPEG, WEBP")]
    UnsupportedFormat(String),

    #[error("Generation provider error: {0}")]
    ProviderError(String),

    #[error("Job cancelled by user")]
    Cancelled,
}

pub struct GenerationService;

impl GenerationService {
    /// Validates user supplied photos before any network calls.
    pub fn validate_photos(photo_paths: &[&Path]) -> Result<(), GenerationError> {
        if photo_paths.is_empty() || photo_paths.len() > 3 {
            return Err(GenerationError::InvalidPhotoCount(photo_paths.len()));
        }

        for p in photo_paths {
            let ext = p
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                return Err(GenerationError::UnsupportedFormat(ext));
            }
        }

        Ok(())
    }

    /// Progresses a generation job through its deterministic stages.
    pub fn advance_job_state(job: &mut GenerationJob) -> Result<(), GenerationError> {
        match job.state {
            GenerationJobState::Pending => {
                info!("Generation job {}: Validating photos...", job.id);
                job.state = GenerationJobState::Validating;
                job.progress = 0.15;
            }
            GenerationJobState::Validating => {
                info!("Generation job {}: Segmenting foreground pet...", job.id);
                job.state = GenerationJobState::Segmenting;
                job.progress = 0.35;
            }
            GenerationJobState::Segmenting => {
                info!("Generation job {}: Generating animation poses...", job.id);
                job.state = GenerationJobState::Generating;
                job.progress = 0.65;
            }
            GenerationJobState::Generating => {
                info!("Generation job {}: Scoring pose consistency...", job.id);
                job.state = GenerationJobState::Scoring;
                job.progress = 0.85;
            }
            GenerationJobState::Scoring => {
                info!(
                    "Generation job {}: Preview ready for user inspection!",
                    job.id
                );
                job.state = GenerationJobState::PreviewReady;
                job.progress = 0.90;
            }
            GenerationJobState::PreviewReady => {
                info!(
                    "Generation job {}: User approved. Building PetPack...",
                    job.id
                );
                job.state = GenerationJobState::Building;
                job.progress = 0.95;
            }
            GenerationJobState::Building => {
                info!(
                    "Generation job {}: Complete! Installed in local packs.",
                    job.id
                );
                job.state = GenerationJobState::Complete;
                job.progress = 1.0;
            }
            GenerationJobState::Complete
            | GenerationJobState::Failed
            | GenerationJobState::Cancelled => {}
        }
        job.updated_at = chrono::Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_photo_validation_count() {
        assert!(matches!(
            GenerationService::validate_photos(&[]),
            Err(GenerationError::InvalidPhotoCount(0))
        ));

        let p1 = Path::new("test1.png");
        let p2 = Path::new("test2.jpg");
        let p3 = Path::new("test3.webp");
        let p4 = Path::new("test4.png");

        assert!(GenerationService::validate_photos(&[p1, p2, p3]).is_ok());
        assert!(matches!(
            GenerationService::validate_photos(&[p1, p2, p3, p4]),
            Err(GenerationError::InvalidPhotoCount(4))
        ));
    }

    #[test]
    fn test_job_state_machine_flow() {
        let mut job = GenerationJob::new("Fluffy", 2);
        assert_eq!(job.state, GenerationJobState::Pending);

        GenerationService::advance_job_state(&mut job).unwrap();
        assert_eq!(job.state, GenerationJobState::Validating);

        GenerationService::advance_job_state(&mut job).unwrap();
        assert_eq!(job.state, GenerationJobState::Segmenting);

        GenerationService::advance_job_state(&mut job).unwrap();
        assert_eq!(job.state, GenerationJobState::Generating);

        GenerationService::advance_job_state(&mut job).unwrap();
        assert_eq!(job.state, GenerationJobState::Scoring);

        GenerationService::advance_job_state(&mut job).unwrap();
        assert_eq!(job.state, GenerationJobState::PreviewReady);

        GenerationService::advance_job_state(&mut job).unwrap();
        assert_eq!(job.state, GenerationJobState::Building);

        GenerationService::advance_job_state(&mut job).unwrap();
        assert_eq!(job.state, GenerationJobState::Complete);
    }
}
