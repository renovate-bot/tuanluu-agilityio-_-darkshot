pub mod actions;
pub mod config;
pub mod geometry;
pub mod i18n;
pub mod image;
pub mod migrate;
pub mod naming;
pub mod sigv4;
pub mod theme;
pub mod upload;

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_builds() {
        assert_eq!(2 + 2, 4);
    }
}
