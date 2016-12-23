#[cfg(feature = "postgres")]
mod postgres {
    use std::collections::HashMap;
    use std::fmt::Write;

    infer_enums!("dotenv:DATABASE_URL");

    #[test]
    fn inferred_existing_enums() {
        let _something = UserType::Admin;
        let _something = UserType::Guest;
        let _something = UserType::GroupOwner;
        let _something = UserType::Default;
    }

    #[test]
    fn enum_supports_eq() {
        assert_eq!(UserType::Admin, UserType::Admin);
        assert!(UserType::Admin != UserType::Guest);
    }

    #[test]
    fn enum_supports_clone_and_copy() {
        let something = UserType::Admin.clone();
        let something_else = UserType::Guest;
        let closed_1 = || { something == something_else };
        let closed_2 = || { something != something_else };
        assert!(closed_1() != closed_2());
    }

    #[test]
    fn enum_supports_debug() {
        let mut s = String::new();
        write!(&mut s, "{:?}", UserType::Default).expect("error writing to buffer");
        assert_eq!("Default", s);
    }

    #[test]
    fn enum_goes_into_hash_container() {
        let mut h = HashMap::new();
        h.insert(UserType::Default, UserType::Guest);
        assert!(h.contains_key(&UserType::Default));
    }
}
