pub mod custom;
pub mod google_sheets;
pub mod make;
pub mod n8n;
pub mod ntfy;
pub mod zapier;

pub use custom::{AuthType, CustomTarget};
pub use google_sheets::GoogleSheetsTarget;
pub use make::MakeTarget;
pub use n8n::N8nTarget;
pub use ntfy::NtfyTarget;
pub use zapier::ZapierTarget;
