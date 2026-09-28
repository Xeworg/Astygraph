/// Application identity smoke tests.
mod app_identity {
    #[test]
    fn application_name_exported() {
        // The APPLICATION_NAME constant must be exported from the crate root.
        let name = astynex::APPLICATION_NAME;
        assert_eq!(name, "Astynex");
    }
}
