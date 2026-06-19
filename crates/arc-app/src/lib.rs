pub mod domain;
pub mod http {
    pub mod controllers {
        pub mod internal_projection_controller;
    }
}

pub mod helpers {
    pub mod config;
    pub mod database;

    #[cfg(test)]
    pub mod test {
        pub struct InMemoryTestGuard;

        impl Drop for InMemoryTestGuard {
            fn drop(&mut self) {
                crate::helpers::database::reset_pool();
            }
        }
    }
}
