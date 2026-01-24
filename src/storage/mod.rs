pub mod local;
pub mod s3;
pub mod r#trait;

pub use local::LocalStorage;
pub use r#trait::{StorageBackend, StorageError};
