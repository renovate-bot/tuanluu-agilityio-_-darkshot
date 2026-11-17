pub mod actions;
pub mod annotate;
pub mod color;
pub mod config;
pub mod edit;
pub mod geometry;
pub mod history;
pub mod i18n;
pub mod image;
pub mod migrate;
pub mod naming;
pub mod sigv4;
pub mod theme;
pub mod update;
pub mod upload;

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_builds() {
        assert_eq!(2 + 2, 4);
    }
}
