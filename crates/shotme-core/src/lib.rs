pub mod config;
pub mod geometry;
pub mod naming;
pub mod sigv4;

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_builds() {
        assert_eq!(2 + 2, 4);
    }
}
