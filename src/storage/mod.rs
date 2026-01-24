pub mod local;
pub mod r2;
pub mod s3;
pub mod r#trait;

pub use local::LocalStorage;
pub use r2::R2Storage;
pub use r#trait::{StorageBackend, StorageError};
