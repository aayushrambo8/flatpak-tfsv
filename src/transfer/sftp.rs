pub fn build_sftp_hint(remote_path: &str) -> String {
    format!("sftp user@host:{remote_path}")
}
