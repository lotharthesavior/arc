pub mod domain;

#[cfg(test)]
pub mod helpers {
    pub mod config {
        include!("helpers/config.rs");
    }

    pub mod database {
        include!("helpers/database.rs");
    }

    pub mod test {
        pub struct InMemoryTestGuard;

        impl Drop for InMemoryTestGuard {
            fn drop(&mut self) {
                super::database::reset_pool();
            }
        }
    }
}
